use std::net::Ipv4Addr;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::StreamExt;
use tonic::{transport::Channel, Request};

use crate::{
    app::{AppResult, SessionsMap},
    section::{
        session::utils::init_keymaps,
        shell::{ShellCmd, ShellCmdHistory, ShellStdType},
    },
    tamanoir_grpc::{
        remote_shell_client::RemoteShellClient, session_client::SessionClient, Empty,
        SessionResponse, ShellStd,
    },
};
#[derive(Debug, Clone)]
pub enum GrpcEvent {
    ShellEvent(ShellCmd),
    SessionEvent(SessionResponse),
}

#[derive(Debug, Clone)]
pub struct SessionServiceClient {
    pub client: SessionClient<Channel>,
    pub tx: UnboundedSender<GrpcEvent>,
}

#[derive(Debug, Clone)]
pub struct RemoteShellServiceClient {
    pub client: RemoteShellClient<Channel>,
    pub tx: UnboundedSender<GrpcEvent>,
}

impl SessionServiceClient {
    pub async fn new(ip: Ipv4Addr, port: u16, tx: UnboundedSender<GrpcEvent>) -> AppResult<Self> {
        let mut client = SessionClient::connect(format!("http://{}:{}", ip, port)).await?;
        init_keymaps();
        let request = tonic::Request::new(Empty {});

        let sessions = client.list_sessions(request).await?.into_inner().sessions;
        for s in sessions.into_iter() {
            tx.send(GrpcEvent::SessionEvent(s))?;
        }

        Ok(Self { client, tx })
    }
}
impl RemoteShellServiceClient {
    pub async fn new(ip: Ipv4Addr, port: u16, tx: UnboundedSender<GrpcEvent>) -> AppResult<Self> {
        let client = RemoteShellClient::connect(format!("http://{}:{}", ip, port)).await?;
        Ok(Self { client, tx })
    }
    pub async fn send_cmd(&mut self, cmd: String) -> AppResult<()> {
        let shell_msg = ShellStd {
            message: cmd.clone(),
        };
        let msg = Request::new(shell_msg);
        self.client.send_shell_std_in(msg).await?;
        self.tx.send(GrpcEvent::ShellEvent(ShellCmd {
            std_type: ShellStdType::StdIn,
            inner: cmd,
        }))?;
        Ok(())
    }
}
pub trait StreamReceiver {
    fn listen(&mut self) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

impl StreamReceiver for RemoteShellServiceClient {
    async fn listen(&mut self) -> anyhow::Result<()> {
        let mut stream = self
            .client
            .watch_shell_std_out(Request::new(Empty {}))
            .await?
            .into_inner();
        while let Some(msg) = stream.next().await {
            self.tx.send(GrpcEvent::ShellEvent(ShellCmd {
                inner: msg?.message,
                std_type: ShellStdType::StdOut,
            }))?;
        }
        Ok(())
    }
}
impl StreamReceiver for SessionServiceClient {
    async fn listen(&mut self) -> anyhow::Result<()> {
        let mut stream = self
            .client
            .watch_sessions(Request::new(Empty {}))
            .await?
            .into_inner();
        while let Some(msg) = stream.next().await {
            let updated_session = msg?;
            self.tx.send(GrpcEvent::SessionEvent(updated_session))?;
        }
        Ok(())
    }
}

pub async fn sync_grpc_events(
    rx: &mut UnboundedReceiver<GrpcEvent>,
    sessions: SessionsMap,
    shell_std: ShellCmdHistory,
) -> anyhow::Result<()> {
    while let Some(event) = rx.recv().await {
        match event {
            GrpcEvent::ShellEvent(msg) => {
                shell_std.write().unwrap().push(msg);
            }
            GrpcEvent::SessionEvent(msg) => {
                sessions.write().unwrap().insert(msg.ip.clone(), msg);
            }
        }
    }
    Ok(())
}
