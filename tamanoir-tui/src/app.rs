use std::{
    collections::HashMap,
    error,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::{notifications::Notification, section::Section, tamanoir_grpc::SessionResponse};

pub type AppResult<T> = Result<T, Box<dyn error::Error>>;
pub type SessionsMap = HashMap<String, SessionResponse>;

#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub notifications: Vec<Notification>,
    pub is_editing: bool,
    pub focus_section: Section,
    pub sessions: Arc<RwLock<SessionsMap>>,
    pub shell_std: Arc<RwLock<Vec<String>>>,
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ActivePopup {
    Help,
}

impl App {
    pub async fn new(
        sessions: Arc<RwLock<SessionsMap>>,
        shell_std: Arc<RwLock<Vec<String>>>,
    ) -> AppResult<Self> {
        Ok(Self {
            running: true,
            notifications: Vec::new(),
            is_editing: false,
            focus_section: Section::new(shell_std.clone()),
            sessions,
            shell_std,
        })
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let (_settings_block, section_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .split(frame.area());
            (chunks[0], chunks[2])
        };
        self.focus_section.render(frame, section_block, None);
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn tick(&mut self) {
        self.notifications.retain(|n| n.ttl > 0);
        self.notifications.iter_mut().for_each(|n| n.ttl -= 1);
    }
}
