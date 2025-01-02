use std::{pin::Pin, sync::Arc};

use log::{error, info};
use tokio::{
    io::AsyncReadExt,
    net::TcpListener,
    sync::broadcast::{self, Sender},
};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::tamanoir_grpc::{remote_shell_server::RemoteShell, Empty, ShellStdOut};
type ShellStdOutWatcher = Arc<Sender<String>>;

#[derive(Clone)]
pub struct TcpShell {
    pub port: u16,
    pub tx: ShellStdOutWatcher,
}

impl TcpShell {
    pub fn new(port: u16) -> Self {
        let (tx, _) = broadcast::channel(16);
        TcpShell {
            port,
            tx: Arc::new(tx),
        }
    }

    fn try_send(&self, msg: String) -> anyhow::Result<()> {
        if self.tx.receiver_count() > 0 {
            self.tx.send(msg)?;
        }
        Ok(())
    }

    pub async fn serve(&mut self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Waiting for incoming tcp connection");
        let (mut socket, remote_addr) = listener.accept().await?;
        info!("New connection from: {}", remote_addr);

        let mut buffer = vec![0; 1024];

        loop {
            // Read data from the socket
            match socket.read(&mut buffer).await {
                Ok(0) => {
                    info!("Connection closed by client: {}", remote_addr);
                    break;
                }
                Ok(n) => {
                    let received = String::from_utf8_lossy(&buffer[..n]);
                    info!("Received: {}", received);
                    if let Err(_) = self.try_send(received.into()) {
                        error!("error sending stdout message");
                        break;
                    };
                }
                Err(e) => {
                    error!("Failed to read from socket: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl RemoteShell for TcpShell {
    type WatchShellStdOutStream =
        Pin<Box<dyn Stream<Item = Result<ShellStdOut, Status>> + Send + 'static>>;

    async fn watch_shell_std_out(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::WatchShellStdOutStream>, Status> {
        let mut rx = self.tx.subscribe();

        let stream = async_stream::try_stream! {
        while let Ok(msg) = rx.recv().await {
                yield ShellStdOut {
                    message: msg,
                };
        }
        };
        Ok(Response::new(
            Box::pin(stream) as Self::WatchShellStdOutStream
        ))
    }
}
