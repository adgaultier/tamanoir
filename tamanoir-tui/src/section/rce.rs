use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};

use crate::{
    app::AppResult,
    grpc::RceServiceClient,
    tamanoir_grpc::{SessionRcePayload, SessionResponse},
};

#[derive(Debug)]
pub struct RceSection {
    available_payloads: Option<Vec<SessionRcePayload>>,
    current_payload: Option<SessionRcePayload>,
    current_session: Option<SessionResponse>,
}

impl RceSection {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            available_payloads: None,
            current_payload
            current_payload
        })
    }
    pub async fn fetch_available_payloads(
        &mut self,
        rce_client: &mut RceServiceClient,
    ) -> AppResult<()> {
        self.available_payloads = Some(rce_client.list_available_rce().await?);
        Ok(())
    }
    pub fn render(&mut self, frame: &mut Frame, block: Rect) {}
    pub async fn handle_keys(&mut self, key_event: KeyEvent) -> AppResult<()> {
        match key_event.code {
            KeyCode::Char('l') => if let Some(payloads) = self.available_payloads {},
            _ => {}
        }
        Ok(())
    }
}
