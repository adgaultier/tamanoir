use std::{
    collections::HashMap,
    error,
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use tokio::sync::mpsc;

use crate::{
    event::Event,
    grpc::{RceServiceClient, RemoteShellServiceClient, SessionServiceClient, StreamReceiver},
    notification::{Notification, NotificationSender},
    section::{shell::ShellCommandHistoryMap, Sections},
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
    pub rce_client: RceServiceClient,
    pub notifications: Vec<Notification>,
}

impl App {
    pub async fn new(
        ip: Ipv4Addr,
        port: u16,
        event_sender: mpsc::UnboundedSender<Event>,
    ) -> AppResult<Self> {
        let sessions: SessionsMap = SessionsMap::default();
        let shell_history: ShellCommandHistoryMap = ShellCommandHistoryMap::default();

        let mut session_client = SessionServiceClient::new(ip, port, event_sender.clone()).await?;
        let shell_client = RemoteShellServiceClient::new(ip, port).await?;
        let mut rce_client = RceServiceClient::new(ip, port).await?;

        let mut shell_receiver = shell_client.clone();
        let mut session_receiver = session_client.clone();
        let notification_sender = NotificationSender {
            ttl: 3,
            sender: event_sender,
        };
        let sections = Sections::new(
            shell_history.clone(),
            sessions.clone(),
            &mut session_client,
            &mut rce_client,
            notification_sender,
        )
        .await?;

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
            rce_client,
            notifications: Vec::new(),
        })
    }
    pub async fn handle_tui_event(&mut self, event: Event) -> AppResult<()> {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Char('c') | KeyCode::Char('C')
                    if key_event.modifiers == KeyModifiers::CONTROL =>
                {
                    self.quit()
                }
                _ => {
                    self.sections
                        .handle_keys(
                            key_event,
                            &mut self.shell_client,
                            &mut self.session_client,
                            &mut self.rce_client,
                        )
                        .await?
                }
            },
            Event::Mouse(mouse_event) => self.sections.handle_mouse(mouse_event).await?,
            Event::Notification(notification) => {
                self.notifications.push(notification);
            }
            Event::Tick => {
                self.notifications.iter_mut().for_each(|n| n.ttl -= 1);
                self.notifications.retain(|n| n.ttl > 0);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn render(&mut self, frame: &mut ratatui::Frame) {
        self.sections.render(frame);
        for (index, notification) in self.notifications.iter().enumerate() {
            notification.render(index, frame);
        }
    }
    pub fn quit(&mut self) {
        self.running = false;
    }
}
