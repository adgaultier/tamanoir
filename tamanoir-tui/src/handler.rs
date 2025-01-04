use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::{
    app::{App, AppResult},
    event::Event,
    notifications::{Notification, NotificationLevel},
    tamanoir_grpc::SessionResponse,
};

pub async fn handle_key_events(
    key_event: KeyEvent,
    app: &mut App,
    sender: mpsc::UnboundedSender<Event>,
) -> AppResult<()> {
    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }

        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }
        KeyCode::Char('l') => {
            dbg!(app.grpc.sessions.keys());
        }
        KeyCode::Char('p') => {
            let s = app.grpc.sessions.get("192.168.1.180").ok_or("Not found")?;
            let keys = s.parse_keycodes(crate::session::Layout::Azerty)?;
            dbg!(SessionResponse::format_keys(keys));
        }
        _ => {}
    }

    Ok(())
}
