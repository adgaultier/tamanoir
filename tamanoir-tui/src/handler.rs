use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::{App, AppResult},
    grpc::{RemoteShellServiceClient, SessionServiceClient},
};

pub async fn handle_key_events(
    key_event: KeyEvent,
    app: &mut App,
    shell_client: &mut RemoteShellServiceClient,
    session_client: &mut SessionServiceClient,
) -> AppResult<()> {
    // if app.is_editing {
    // match key_event.code {
    //     KeyCode::Esc | KeyCode::Enter => app.is_editing = false,
    //     _ => {}
    // }
    match key_event.code {
        KeyCode::Esc => {
            app.quit();
        }

        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            } else {
                app.sections
                    .handle_keys(key_event, shell_client, session_client)
                    .await?
            }
        }
        _ => {
            app.sections
                .handle_keys(key_event, shell_client, session_client)
                .await?
        }
    }

    //}

    //     KeyCode::Char('l') => {
    //         dbg!(app.grpc.sessions.keys());
    //     }
    //     KeyCode::Char('p') => {
    //         let s = app.grpc.sessions.get("192.168.1.180").ok_or("Not found")?;
    //         let keys = s.parse_keycodes(crate::section::session::Layout::Azerty)?;
    //         dbg!(SessionResponse::format_keys(keys));
    //     }
    //     KeyCode::Char('t') => {
    //         dbg!(app.grpc.sessions.keys());
    //     }
    //     _ => {}
    // }

    Ok(())
}
