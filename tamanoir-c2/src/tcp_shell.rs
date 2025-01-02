use std::{pin::Pin, sync::Arc};

use log::{error, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{
        broadcast::{self, Sender},
        Mutex,
    },
};
use tokio_stream::Stream;
use tonic::{Code, Request, Response, Status};

use crate::tamanoir_grpc::{remote_shell_server::RemoteShell, Empty, ShellStd};
type ShellStdWatcher = Arc<Sender<String>>;

pub struct TcpShell {
    pub port: u16,
    pub tx: ShellStdWatcher,
    pub socket: Option<Arc<Mutex<TcpStream>>>,
}

impl TcpShell {
    pub fn new(port: u16) -> Self {
        let (tx, _) = broadcast::channel(16);
        TcpShell {
            port,
            tx: Arc::new(tx),
            socket: None,
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
        self.socket = Some(Arc::new(Mutex::new(socket)));
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
        Pin<Box<dyn Stream<Item = Result<ShellStd, Status>> + Send + 'static>>;

    async fn watch_shell_std_out(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::WatchShellStdOutStream>, Status> {
        let mut rx = self.tx.subscribe();

        let stream = async_stream::try_stream! {
        while let Ok(msg) = rx.recv().await {
                yield ShellStd {
                    message: msg,
                };
        }
        };
        Ok(Response::new(
            Box::pin(stream) as Self::WatchShellStdOutStream
        ))
    }
    async fn send_shell_std_in(
        &self,
        request: Request<ShellStd>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        if let Some(sock) = &self.socket {
            let mut sock = sock.lock().await;
            sock.write_all(req.message.as_bytes())
                .await
                .map_err(|_| Status::new(Code::Internal, "couldnt write to socket"))?;
            return Ok(Response::new(Empty {}));
        } else {
            Err(Status::new(Code::NotFound, "socket not created"))
        }
    }
}
