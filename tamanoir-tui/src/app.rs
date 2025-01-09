use std::{
    collections::HashMap,
    error,
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use ratatui::Frame;

use crate::{
    grpc::{RemoteShellServiceClient, SessionServiceClient, StreamReceiver},
    notifications::Notification,
    section::{shell::ShellCmdHistory, Sections},
    tamanoir_grpc::SessionResponse,
};

pub type AppResult<T> = Result<T, Box<dyn error::Error>>;
pub type SessionsMap = Arc<RwLock<HashMap<String, SessionResponse>>>;

#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub notifications: Vec<Notification>,
    pub is_editing: bool,
    pub sections: Sections,
    pub shell_client: RemoteShellServiceClient,
    pub session_client: SessionServiceClient,
}

impl App {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        let sessions: SessionsMap = SessionsMap::default();

        let shell_std: ShellCmdHistory = Arc::new(RwLock::new(Vec::new()));

        let mut session_client = SessionServiceClient::new(ip, port).await?;
        let shell_client = RemoteShellServiceClient::new(ip, port).await?;

        let mut shell_receiver = shell_client.clone();
        let mut session_receiver = session_client.clone();

        let mut sections = Sections::new(shell_std.clone(), sessions.clone());
        sections.session_section.init(&mut session_client).await?;

        tokio::spawn(async move {
            tokio::try_join!(
                session_receiver.listen(sessions.clone()),
                shell_receiver.listen(shell_std.clone()),
            )
        });

        Ok(Self {
            running: true,
            notifications: Vec::new(),
            is_editing: false,
            sections,
            shell_client,
            session_client,
        })
    }

    pub fn render(&mut self, frame: &mut Frame) {
        self.sections.render(frame, frame.area());
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn tick(&mut self) {
        self.notifications.retain(|n| n.ttl > 0);
        self.notifications.iter_mut().for_each(|n| n.ttl -= 1);
    }
}
