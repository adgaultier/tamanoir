use std::{io, net::Ipv4Addr};

use clap::Parser;
use ratatui::{backend::CrosstermBackend, Terminal};
use tamanoir_tui::{
    app::{App, AppResult},
    event::EventHandler,
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

    let mut app = App::new(ip, port, tui.events.sender.clone()).await?;

    while app.running {
        tui.draw(&mut app)?;
        let event = tui.events.next().await?;
        app.handle_tui_event(event).await?;
    }
    tui.exit()?;
    Ok(())
}
