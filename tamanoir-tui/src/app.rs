use std::{
    collections::HashMap,
    error,
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

use anyhow::Result;

use crate::{
    grpc::{RemoteShellServiceClient, SessionServiceClient, StreamReceiver},
    section::{shell::ShellCommandHistory, Sections},
    tamanoir_grpc::SessionResponse,
};

pub type AppResult<T> = Result<T, Box<dyn error::Error>>;
pub type SessionsMap = Arc<RwLock<HashMap<String, SessionResponse>>>;

#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub is_editing: bool,
    pub sections: Sections,
    pub shell_client: RemoteShellServiceClient,
    pub session_client: SessionServiceClient,
}

impl App {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        let sessions: SessionsMap = SessionsMap::default();

        let shell_history: ShellCommandHistory = Arc::new(RwLock::new(Vec::new()));

        let mut session_client = SessionServiceClient::new(ip, port).await?;
        let shell_client = RemoteShellServiceClient::new(ip, port).await?;

        let mut shell_receiver = shell_client.clone();
        let mut session_receiver = session_client.clone();

        let mut sections = Sections::new(shell_history.clone(), sessions.clone());
        sections.session_section.init(&mut session_client).await?;

        tokio::spawn(async move {
            tokio::try_join!(
                session_receiver.listen(sessions.clone()),
                shell_receiver.listen(shell_history.clone()),
            )
        });

        Ok(Self {
            running: true,
            is_editing: false,
            sections,
            shell_client,
            session_client,
        })
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}
