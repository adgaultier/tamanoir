use std::{
    io,
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

use clap::Parser;
use ratatui::{backend::CrosstermBackend, Terminal};
use tamanoir_tui::{
    app::{App, AppResult, SessionsMap},
    event::{Event, EventHandler},
    grpc::{RemoteShellServiceClient, SessionServiceClient, StreamReceiver},
    handler::handle_key_events,
    section::shell::ShellCmdHistory,
    tui::Tui,
};
use tokio::sync::mpsc;
#[derive(Parser)]
pub struct Opt {
    #[clap(long, short)]
    ip: Ipv4Addr,
    #[clap(short, long, default_value = "50051")]
    port: u16,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let Opt { ip, port } = Opt::parse();

    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(1_000);

    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    let sessions: SessionsMap = SessionsMap::default();
    let shell_std: ShellCmdHistory = Arc::new(RwLock::new(Vec::new()));

    let mut app = App::new(sessions.clone(), shell_std.clone()).await?;

    let mut session_client = SessionServiceClient::new(ip, port).await?;
    let mut shell_client = RemoteShellServiceClient::new(ip, port).await?;

    let mut shell_receiver = shell_client.clone();
    let mut session_receiver = session_client.clone();

    tokio::spawn(async move {
        tokio::try_join!(
            session_receiver.listen(sessions.clone()),
            shell_receiver.listen(shell_std.clone()),
        )
    });

    while app.running {
        tui.draw(&mut app)?;
        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => {
                handle_key_events(key_event, &mut app, &mut shell_client, &mut session_client)
                    .await?
            }
            Event::Notification(notification) => {
                app.notifications.push(notification);
            }
            _ => {}
        }
    }

    tui.exit()?;
    Ok(())
}
