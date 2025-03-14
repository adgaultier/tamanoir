use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc;

use crate::{app::AppResult, event::Event};

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub ttl: u16,
}

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Error,
    Warning,
    Info,
}

impl Notification {
    pub fn render(&self, index: usize, frame: &mut Frame) {
        let (color, title) = match self.level {
            NotificationLevel::Info => (Color::Green, "󰋼 "),
            NotificationLevel::Warning => (Color::Yellow, " "),
            NotificationLevel::Error => (Color::Red, " "),
        };

        let mut text = Text::from(vec![
            Line::from(title).style(Style::new().fg(color).add_modifier(Modifier::BOLD))
        ]);

        text.extend(Text::from(self.message.as_str()));

        let notification_height = text.height() as u16 + 2;
        let notification_width = text.width() as u16 + 4;

        let block = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default())
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(color)),
            );

        let area = notification_rect(
            index as u16,
            notification_height,
            notification_width,
            frame.area(),
        );

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }
}

fn notification_rect(offset: u16, height: u16, width: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(height * offset),
                Constraint::Length(height),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(width),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[derive(Debug, Clone)]
pub struct NotificationSender {
    pub sender: mpsc::UnboundedSender<Event>,
    pub ttl: u16,
}

impl NotificationSender {
    fn send<T: ToString>(&self, message: T, level: NotificationLevel) -> AppResult<()> {
        let notif = Notification {
            message: message.to_string(),
            level,
            ttl: self.ttl,
        };

        self.sender.send(Event::Notification(notif))?;

        Ok(())
    }
    pub fn info<T: ToString>(&self, message: T) -> AppResult<()> {
        self.send(message, NotificationLevel::Info)?;
        Ok(())
    }
    pub fn warning<T: ToString>(&self, message: T) -> AppResult<()> {
        self.send(message, NotificationLevel::Warning)?;
        Ok(())
    }
    pub fn error<T: ToString>(&self, message: T) -> AppResult<()> {
        self.send(message, NotificationLevel::Error)?;
        Ok(())
    }
}
