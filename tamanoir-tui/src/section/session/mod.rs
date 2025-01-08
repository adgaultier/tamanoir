use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use crate::{
    app::{AppResult, SessionsMap},
    grpc::SessionServiceClient,
};

pub mod utils;
#[derive(Debug)]
pub struct SessionsSection {
    pub sessions: SessionsMap,
}
impl SessionsSection {
    pub fn new(sessions: SessionsMap) -> Self {
        Self { sessions }
    }
    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        sessions_client: &mut SessionServiceClient,
    ) -> AppResult<()> {
        Ok(())
    }
    pub fn render(&mut self, frame: &mut Frame, block: Rect) {}
}
