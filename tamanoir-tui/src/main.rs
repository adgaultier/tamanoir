use std::io;

use clap::{crate_description, crate_version, Command};
use ratatui::{backend::CrosstermBackend, Terminal};
use tamanoir_tui::{
    app::{App, AppResult},
    event::{Event, EventHandler},
    handler::handle_key_events,
    tui::Tui,
};

#[tokio::main]
async fn main() -> AppResult<()> {
    Command::new("tamanoir-tui")
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::new("ip")
                .long("ip")
                .short('i')
                .takes_value(true)
                .required(true)
                .validator(|v| {
                    v.parse::<Ipv4Addr>()
                        .map(|_| ())
                        .map_err(|_| "Invalid IPv4 address".to_string())
                })
                .help("IP address to connect to"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .takes_value(true)
                .required(true)
                .validator(|v| {
                    v.parse::<u16>()
                        .map(|_| ())
                        .map_err(|_| "Invalid port".to_string())
                })
                .help("Port number to use"),
        )
        .get_matches();
    let ip: Ipv4Addr = matches.value_of("ip").unwrap().parse().unwrap();
    let port: u16 = matches.value_of("port").unwrap().parse().unwrap()
    let mut app = App::new(ip,port).await?;

    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(1_000);

    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    while app.running {
        tui.draw(&mut app)?;
        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => {
                handle_key_events(key_event, &mut app, tui.events.sender.clone()).await?
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
