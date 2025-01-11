use std::{io, net::Ipv4Addr};

use clap::Parser;
use ratatui::{backend::CrosstermBackend, Terminal};
use tamanoir_tui::{
    app::{App, AppResult},
    event::{Event, EventHandler},
    handler::handle_key_events,
    tui::Tui,
};

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

    let mut app = App::new(ip, port).await?;

    while app.running {
        tui.draw(&mut app)?;

        if let Event::Key(key_event) = tui.events.next().await? {
            handle_key_events(key_event, &mut app).await?
        }
    }

    tui.exit()?;
    Ok(())
}
