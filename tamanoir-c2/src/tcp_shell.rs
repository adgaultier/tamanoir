use std::{pin::Pin, sync::Arc};

use log::{error, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{broadcast, mpsc, Mutex},
};
use tokio_stream::Stream;
use tonic::{Code, Request, Response, Status};

use crate::tamanoir_grpc::{remote_shell_server::RemoteShell, Empty, ShellStd};
type ShellStdOutWatcher = Arc<broadcast::Sender<String>>;
type ShellStdInTx = Arc<mpsc::Sender<String>>;
type ShellStdInRx = Arc<Mutex<mpsc::Receiver<String>>>;
#[derive(Clone)]
pub struct TcpShell {
    pub port: u16,
    pub stdout_broadcast: ShellStdOutWatcher,
    pub stdin_tx: ShellStdInTx,
    pub stdin_rx: ShellStdInRx,
}

impl TcpShell {
    pub fn new(port: u16) -> Self {
        let (tx, _) = broadcast::channel(16);
        let (ttx, rx) = mpsc::channel::<String>(16);
        TcpShell {
            port,
            stdout_broadcast: Arc::new(tx),
            stdin_tx: Arc::new(ttx),
            stdin_rx: Arc::new(Mutex::new(rx)),
        }
    }

    fn try_send(tx: ShellStdOutWatcher, msg: String) -> anyhow::Result<()> {
        if tx.receiver_count() > 0 {
            tx.send(msg)?;
        }
        Ok(())
    }

    pub async fn serve(&mut self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Waiting for incoming tcp connection");
        let (socket, remote_addr) = listener.accept().await?;
        info!("New connection from: {}", remote_addr);

        let (mut reader, mut writer) = socket.into_split();
        let tx = self.stdout_broadcast.clone();
        let rx = self.stdin_rx.clone();

        let write_task = tokio::spawn(async move {
            while let Some(msg) = rx.lock().await.recv().await {
                info!("STDIN RECEIVED: {}", msg);
                if let Err(e) = writer.write_all(msg.as_bytes()).await {
                    error!("Error writing to socket: {}", e);
                    break;
                }
            }
        });

        let read_task = tokio::spawn(async move {
            let mut buffer = vec![0; 1024];
            loop {
                // Read data from the socket
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        info!("Connection closed by client: {}", remote_addr);
                        break;
                    }
                    Ok(n) => {
                        let received = String::from_utf8_lossy(&buffer[..n]);
                        info!("Received: {}", received);
                        if let Err(_) = Self::try_send(tx.clone(), received.into()) {
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
        });
        let _ = tokio::join!(read_task, write_task);
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
        let mut rx = self.stdout_broadcast.subscribe();

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
        self.stdin_tx
            .send(req.message.into())
            .await
            .map_err(|_| Status::new(Code::Internal, "couldnt write to socket"))?;
        return Ok(Response::new(Empty {}));
    }
}
