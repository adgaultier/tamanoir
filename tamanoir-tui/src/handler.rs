use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppResult};

pub async fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match key_event.code {
        KeyCode::Esc => {
            app.quit();
        }

        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            } else {
                app.sections
                    .handle_keys(key_event, &mut app.shell_client, &mut app.session_client)
                    .await?
            }
        }
        _ => {
            app.sections
                .handle_keys(key_event, &mut app.shell_client, &mut app.session_client)
                .await?
        }
    }

    Ok(())
}
