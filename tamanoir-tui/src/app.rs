use std::{error, net::Ipv4Addr};

use anyhow::Result;
use ratatui::Frame;

use crate::{grpc::Grpc, notifications::Notification};

pub type AppResult<T> = Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub notifications: Vec<Notification>,
    pub grpc: Grpc,
}

impl App {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        Ok(Self {
            running: true,
            notifications: Vec::new(),
            grpc: Grpc::new(ip, port).await?,
        })
    }

    pub fn render(&mut self, frame: &mut Frame) {}

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn tick(&mut self) {
        self.notifications.retain(|n| n.ttl > 0);
        self.notifications.iter_mut().for_each(|n| n.ttl -= 1);
    }
}
