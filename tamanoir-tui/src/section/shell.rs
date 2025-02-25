use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    app::AppResult,
    grpc::{RceServiceClient, RemoteShellServiceClient},
};
#[derive(Debug, Clone, PartialEq)]
pub enum ShellHistoryEntryType {
    Command,
    Response,
}

#[derive(Debug, Clone)]
pub struct ShellCommandEntry {
    pub entry_type: ShellHistoryEntryType,
    pub text: String,
    pub session_id: String,
}

pub type ShellCommandHistoryMap = Arc<RwLock<HashMap<String, Vec<ShellCommandEntry>>>>;
#[derive(Debug)]
pub struct SessionShellSection {
    prompt: Input,
    shell_history_map: ShellCommandHistoryMap,
    vertical_scroll: u16,
    max_scroll: usize,
    vertical_scroll_state: ScrollbarState,
    pub manual_scroll: bool,
    history_index: usize,
    current_height: usize,
}

impl SessionShellSection {
    pub fn new(shell_history_map: ShellCommandHistoryMap) -> Self {
        Self {
            prompt: Input::default(),
            shell_history_map,
            vertical_scroll: 0,
            max_scroll: 2,
            manual_scroll: false,
            vertical_scroll_state: ScrollbarState::default(),
            current_height: 0,
            history_index: 0,
        }
    }
    fn get_stdin_history(&self, session_id: String) -> Vec<String> {
        self.shell_history_map
            .read()
            .unwrap()
            .get(&session_id)
            .unwrap()
            .iter()
            .filter(|cmd| cmd.entry_type == ShellHistoryEntryType::Command && cmd.text.len() > 0)
            .map(|cmd| cmd.text.clone())
            .collect::<Vec<String>>()
    }
    pub fn render(
        &mut self,
        frame: &mut Frame,
        block: Rect,
        is_focused: bool,
        session_id: String,
        shell_available: bool,
    ) {
        let highlight_color = if is_focused {
            Color::Yellow
        } else {
            Color::Blue
        };
        self.current_height = block.height as usize;
        if shell_available {
            let history = self.shell_history_map.read().unwrap();

            let mut text: Vec<Line> = if let Some(session_history) = history.get(&session_id) {
                session_history
                    .iter()
                    .map(|entry| match entry.entry_type {
                        ShellHistoryEntryType::Command => {
                            vec![Line::from(Span::raw(format!("$ {}", entry.text)).bold())]
                        }
                        ShellHistoryEntryType::Response => entry
                            .text
                            .split('\n')
                            .filter(|s| s.len() > 0)
                            .map(Line::from)
                            .collect(),
                    })
                    .flatten()
                    .collect()
            } else {
                vec![]
            };

            let prompt_text = Line::from(Span::from(format!("$ {}", self.prompt.value())).bold());
            text.push(prompt_text);

            self.max_scroll = text.len() - 1;
            self.vertical_scroll_state = self.vertical_scroll_state.content_length(self.max_scroll);
            if !self.manual_scroll {
                let cursor = self
                    .max_scroll
                    .saturating_sub(self.current_height.saturating_sub(3));
                self.vertical_scroll_state = self.vertical_scroll_state.position(cursor);
                self.vertical_scroll = cursor as u16;
            };

            let history = Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .scroll((self.vertical_scroll, 0))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(Span::styled(
                            "Remote Shell",
                            Style::default().fg(highlight_color).bold(),
                        ))
                        .border_style(Style::new().fg(highlight_color)),
                );

            frame.render_widget(history, block);
            if self.manual_scroll {
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));

                frame.render_stateful_widget(
                    scrollbar,
                    block.inner(Margin {
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut self.vertical_scroll_state,
                );
            }
        } else {
            let message =
                Paragraph::new(Line::from(Span::raw("SessionShellSection Not Connected")).bold())
                    .centered()
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(Span::styled(
                                "Remote Shell",
                                Style::default().fg(highlight_color).bold(),
                            ))
                            .border_style(Style::new().fg(highlight_color)),
                    );
            frame.render_widget(message, block);
        }
    }
    fn clear(&mut self) {
        self.shell_history_map.write().unwrap().clear();
    }
    pub fn scroll_up(&mut self) {
        self.manual_scroll = true;
        self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .position(self.vertical_scroll.into());
    }
    pub fn scroll_down(&mut self) {
        self.manual_scroll = true;
        self.vertical_scroll = self
            .vertical_scroll
            .saturating_add(1)
            .min(self.max_scroll as u16);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .position(self.vertical_scroll.into());
    }
    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        shell_client: &mut RemoteShellServiceClient,
        rce_service_client: &mut RceServiceClient,

        session_id: String,
    ) -> AppResult<()> {
        match key_event.code {
            KeyCode::Enter => {
                let command = self.prompt.value();
                if command.to_string() == "clear" {
                    self.clear();
                } else {
                    if shell_client
                        .send_cmd(session_id.clone(), command.to_string())
                        .await
                        .is_ok()
                    {
                        if let Some(session_history) = self
                            .shell_history_map
                            .write()
                            .unwrap()
                            .get_mut(&session_id.clone())
                        {
                            session_history.push(ShellCommandEntry {
                                text: command.to_string(),
                                entry_type: ShellHistoryEntryType::Command,
                                session_id: session_id,
                            })
                        };
                    } else {
                        rce_service_client.delete_session_rce(session_id).await?;
                    }
                    self.prompt.reset();
                }
            }
            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::CONTROL => self.clear(),

            KeyCode::Char('k') | KeyCode::Up if key_event.modifiers == KeyModifiers::CONTROL => {
                self.scroll_up();
            }

            KeyCode::Char('j') | KeyCode::Down if key_event.modifiers == KeyModifiers::CONTROL => {
                self.scroll_down();
            }
            KeyCode::Up => {
                self.history_index = self.history_index.saturating_sub(1);
                if let Some(cmd) = self.get_stdin_history(session_id).get(self.history_index) {
                    self.prompt = self.prompt.clone().with_value(cmd.clone());
                }
            }
            KeyCode::Down => {
                let current_idx: usize = self.get_stdin_history(session_id.clone()).len();
                self.history_index = current_idx.min(self.history_index + 1);
                if self.history_index < current_idx {
                    if let Some(cmd) = self.get_stdin_history(session_id).get(self.history_index) {
                        self.prompt = self.prompt.clone().with_value(cmd.clone());
                    }
                } else {
                    self.prompt.reset();
                }
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

#[derive(PartialEq, Debug)]
pub enum ShellAvailablilityStatus {
    Unavailable,
    Transmiting,
    Available,
    Connected,
}
