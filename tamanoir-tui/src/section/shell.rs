use std::sync::{Arc, RwLock};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Span, Text},
    widgets::{
        Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{app::AppResult, grpc::RemoteShellServiceClient};

pub type ShellCommandHistory = Arc<RwLock<Vec<ShellCommandEntry>>>;
use crate::tamanoir_grpc::SessionResponse;
#[derive(Debug, Clone, PartialEq)]
pub enum ShellHistoryEntryType {
    Command,
    Response,
}

#[derive(Debug, Clone)]
pub struct ShellCommandEntry {
    pub entry_type: ShellHistoryEntryType,
    pub text: String,
}

#[derive(Debug)]
pub struct Shell {
    prompt: Input,
    history: ShellCommandHistory,
    vertical_scroll: u16,
    manual_scroll: bool,
}

impl Shell {
    pub fn new(history: ShellCommandHistory) -> Self {
        Self {
            prompt: Input::default(),
            history,
            vertical_scroll: 0,
            manual_scroll: false,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, block: Rect, is_focused: bool) {
        let highlight_color = if is_focused {
            Color::Yellow
        } else {
            Color::Blue
        };

        let mut text: Text<'_> = self
            .history
            .read()
            .unwrap()
            .iter()
            .map(|entry| match entry.entry_type {
                ShellHistoryEntryType::Command => {
                    format!("$ {}", entry.text)
                }
                ShellHistoryEntryType::Response => entry.text.clone(),
            })
            .collect();

        let prompt_text = Text::from(format!("$ {}", self.prompt.value()));
        text.extend(prompt_text);

        let vertical_scroll = (text.height() + 2).saturating_sub(block.height as usize);

        if !self.manual_scroll {
            self.vertical_scroll = vertical_scroll as u16;
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state =
            ScrollbarState::new(vertical_scroll).position(self.vertical_scroll.into());

        let hisotory = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .scroll((
                {
                    if !self.manual_scroll {
                        self.vertical_scroll
                    } else {
                        vertical_scroll as u16
                    }
                },
                0,
            ))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(Span::styled(
                        "Remote Shell",
                        Style::default().fg(highlight_color).bold(),
                    ))
                    .border_style(Style::new().fg(highlight_color)),
            );

        frame.render_widget(hisotory, block);
        frame.render_stateful_widget(
            scrollbar,
            block.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        shell_client: &mut RemoteShellServiceClient,
        current_session: Option<SessionResponse>,
    ) -> AppResult<()> {
        match key_event.code {
            KeyCode::Enter => {
                let command = self.prompt.value();

                shell_client
                    .send_cmd(current_session.unwrap().ip, command.to_string())
                    .await?;

                self.history.write().unwrap().push(ShellCommandEntry {
                    text: command.to_string(),
                    entry_type: ShellHistoryEntryType::Command,
                });

                self.prompt.reset();
            }

            KeyCode::Char('k') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.manual_scroll = true;
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
            }

            KeyCode::Char('j') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.manual_scroll = true;
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
            }

            KeyCode::Esc => {
                self.manual_scroll = false;
            }

            _ => {
                self.prompt.handle_event(&Event::Key(key_event));
            }
        }
        Ok(())
    }
}
