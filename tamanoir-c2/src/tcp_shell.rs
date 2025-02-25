use std::{
    collections::{hash_map::Entry, HashMap},
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::{Arc, RwLock},
};

use log::{debug, error, info};
use mpsc::channel;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc, Mutex},
};

use crate::{tamanoir_grpc::ShellStd, SessionsStore};
type ShellStdOutWatcher = Arc<broadcast::Sender<ShellStd>>;
type ShellStdInTx = Arc<mpsc::Sender<ShellStd>>;
type ShellStdInRx = Arc<Mutex<mpsc::Receiver<ShellStd>>>;

#[derive(Clone)]
pub struct RxTxContainer {
    pub stdin_rx: ShellStdInRx,
    pub stdin_tx: ShellStdInTx,
}
impl RxTxContainer {
    pub fn new() -> Self {
        let (tx, rx) = channel::<ShellStd>(16);
        Self {
            stdin_rx: Arc::new(Mutex::new(rx)),
            stdin_tx: Arc::new(tx),
        }
    }
}

#[derive(Clone)]
pub struct TcpShell {
    pub port: u16,
    pub stdout_broadcast_tx: ShellStdOutWatcher,
    pub rx_tx_map: Arc<RwLock<HashMap<String, RxTxContainer>>>,
    pub session_store: SessionsStore,
}

impl TcpShell {
    pub fn new(port: u16, session_store: SessionsStore) -> Self {
        let (stdout_broadcast_tx, _) = broadcast::channel(16);
        TcpShell {
            port,
            stdout_broadcast_tx: Arc::new(stdout_broadcast_tx),
            rx_tx_map: Arc::new(RwLock::new(HashMap::new())),
            session_store,
        }
    }
    fn try_send(
        ip: String,
        stdout_broadcast_tx: ShellStdOutWatcher,
        msg: String,
    ) -> anyhow::Result<()> {
        if stdout_broadcast_tx.receiver_count() > 0 {
            stdout_broadcast_tx.send(ShellStd {
                ip: ip,
                message: msg,
            })?;
        }
        Ok(())
    }
    pub fn shell_available(&self, ip: String) -> bool {
        self.get_rx_tx(ip).is_ok()
    }
    pub fn get_rx_tx(&self, ip: String) -> Result<RxTxContainer, String> {
        Ok(self
            .rx_tx_map
            .read()
            .unwrap()
            .get(&ip)
            .ok_or(format!("{} not found", ip))?
            .clone())
    }

    pub async fn handle_connection(
        &mut self,
        socket: TcpStream,
        addr: SocketAddr,
    ) -> anyhow::Result<()> {
        info!("New connection from: {}", addr);
        let ip = addr.ip().to_string();

        if let Entry::Vacant(e) = self.rx_tx_map.write().unwrap().entry(ip.clone()) {
            e.insert(RxTxContainer::new());
        }

        let (mut reader, mut writer) = socket.into_split();
        let rxtx = self.get_rx_tx(ip.clone()).map_err(anyhow::Error::msg)?;

        let rx = rxtx.stdin_rx;
        let broadcast_tx = self.stdout_broadcast_tx.clone();

        let write_task = tokio::spawn(async move {
            while let Some(shell_std) = rx.lock().await.recv().await {
                debug!("Stdin Command Received: {}", shell_std.message);
                let mut cmd = Vec::from(shell_std.message.as_bytes());
                cmd.push(10u8); //add ENTER
                if let Err(e) = writer.write_all(cmd.as_slice()).await {
                    error!("Error writing to socket: {}", e);
                    break;
                }
            }
        });

        let rx_tx_map_clone = self.rx_tx_map.clone();

        // change session shell_availability state to true
        {
            let mut current_sessions = self.session_store.sessions.lock().await;
            let current_session = current_sessions
                .get_mut(&Ipv4Addr::from_str(&ip).unwrap())
                .unwrap();
            current_session.set_shell_availibility(true);
            let _ = self.session_store.notify_update(current_session.clone());
        }

        let sessions_store = self.session_store.clone();
        let read_task = tokio::spawn(async move {
            let mut buffer = vec![0; 1024];
            loop {
                // Read data from the socket
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        info!("Connection closed by client: {}", addr);
                        //remove connection
                        rx_tx_map_clone.write().unwrap().remove(&ip.clone());
                        // change session shell_availability state to false
                        let mut current_sessions = sessions_store.sessions.lock().await;
                        let current_session = current_sessions
                            .get_mut(&Ipv4Addr::from_str(&ip).unwrap())
                            .unwrap();
                        current_session.set_shell_availibility(false);
                        let _ = sessions_store.notify_update(current_session.clone());
                        break;
                    }
                    Ok(n) => {
                        let received = String::from_utf8_lossy(&buffer[..n]);
                        debug!("Stdout Received: {}", received);

                        if let Err(_) =
                            Self::try_send(ip.clone(), broadcast_tx.clone(), received.into())
                        {
                            error!("error sending stdout message");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from socket: {}", e);
                        let mut current_sessions = sessions_store.sessions.lock().await;
                        let current_session = current_sessions
                            .get_mut(&Ipv4Addr::from_str(&ip).unwrap())
                            .unwrap();
                        current_session.set_shell_availibility(false);
                        let _ = sessions_store.notify_update(current_session.clone());
                        break;
                    }
                }
            }
        });
        let _ = tokio::join!(read_task, write_task);
        Ok(())
    }

    pub async fn serve(&mut self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Waiting for incoming tcp connections");
        loop {
            let (socket, remote_addr) = listener.accept().await?;
            // Spawn a new task to handle the connection
            let mut cloned = self.clone();
            tokio::spawn(async move {
                if let Err(e) = cloned.handle_connection(socket, remote_addr).await {
                    error!("Error handling connection from {}: {:?}", remote_addr, e);
                }
            });
        }
    }
}
