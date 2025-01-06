pub mod session;
pub mod shell;

use std::{
    cell::Cell,
    sync::{Arc, RwLock},
};

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{palette::tailwind, Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, HighlightSpacing, Padding, Row, Table, TableState},
    Frame,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    app::{ActivePopup, AppResult, SessionsMap},
    grpc::{RemoteShellServiceClient, SessionServiceClient},
};

#[derive(Debug, PartialEq)]
pub enum FocusedSection {
    Sessions,
    Shell,
    Rce,
}
#[derive(Debug)]
pub struct Section {
    pub focused_section: FocusedSection,
    pub shell_section: shell::ShellSection,
}

impl Section {
    pub fn new(app_shell: Arc<RwLock<Vec<String>>>) -> Self {
        Self {
            focused_section: FocusedSection::Shell,
            shell_section: shell::ShellSection::new(app_shell),
        }
    }
    fn title_span(&self, header_section: FocusedSection) -> Span {
        let is_focused = self.focused_section == header_section;
        match header_section {
            FocusedSection::Sessions => {
                if is_focused {
                    Span::styled(
                        "  Sessions 󰏖   ",
                        Style::default().bg(Color::Green).fg(Color::White).bold(),
                    )
                } else {
                    Span::from("  Sessions 󰏖   ").fg(Color::DarkGray)
                }
            }
            FocusedSection::Shell => {
                if is_focused {
                    Span::styled(
                        "  Shell    ",
                        Style::default().bg(Color::Green).fg(Color::White).bold(),
                    )
                } else {
                    Span::from("  Shell    ").fg(Color::DarkGray)
                }
            }
            FocusedSection::Rce => {
                if is_focused {
                    Span::styled(
                        "  Rce 󱕍   ",
                        Style::default().bg(Color::Green).fg(Color::White).bold(),
                    )
                } else {
                    Span::from("  Rce 󱕍   ").fg(Color::DarkGray)
                }
            }
        }
    }

    fn render_footer_help(
        &self,
        frame: &mut Frame,
        block: Rect,
        active_popup: Option<&ActivePopup>,
    ) {
        let message = {
            match active_popup {
                _ => Line::from(vec![
                    Span::from("f").bold(),
                    Span::from(" Filters").bold(),
                    Span::from(" | ").bold(),
                    Span::from(" ").bold(),
                    Span::from(" Nav").bold(),
                ]),
            }
        };

        let help = Text::from(vec![Line::from(""), message]).blue().centered();
        frame.render_widget(
            help,
            block.inner(Margin {
                horizontal: 1,
                vertical: 0,
            }),
        );
    }
    pub fn render_header(&mut self, frame: &mut Frame, block: Rect) {
        frame.render_widget(
            Block::default()
                .title({
                    Line::from(vec![
                        self.title_span(FocusedSection::Sessions),
                        self.title_span(FocusedSection::Shell),
                        self.title_span(FocusedSection::Rce),
                    ])
                })
                .title_alignment(Alignment::Left)
                .padding(Padding::top(1))
                .borders(Borders::ALL)
                .style(Style::default())
                .border_type(BorderType::default())
                .border_style(Style::default().green()),
            block,
        );
    }

    pub fn render(&mut self, frame: &mut Frame, block: Rect, active_popup: Option<&ActivePopup>) {
        let (section_block, help_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(3)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(block);

            (chunks[0], chunks[1])
        };

        self.render_header(frame, section_block);
        self.render_footer_help(frame, help_block, active_popup);
        let section_block = Layout::default()
            .direction(Direction::Horizontal)
            .margin(2)
            .constraints([Constraint::Fill(1)])
            .split(section_block)[0];

        match self.focused_section {
            FocusedSection::Sessions => {
                unimplemented!()
            }
            FocusedSection::Shell => {
                self.shell_section.render(frame, section_block);
            }
            FocusedSection::Rce => {
                unimplemented!()
            }
        }
    }

    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        shell_client: &mut RemoteShellServiceClient,
        _session_client: &mut SessionServiceClient,
    ) -> AppResult<()> {
        match key_event.code {
            KeyCode::Tab => match self.focused_section {
                FocusedSection::Sessions => self.focused_section = FocusedSection::Shell,
                FocusedSection::Shell => self.focused_section = FocusedSection::Rce,
                FocusedSection::Rce => self.focused_section = FocusedSection::Sessions,
            },

            KeyCode::BackTab => match self.focused_section {
                FocusedSection::Sessions => self.focused_section = FocusedSection::Rce,
                FocusedSection::Shell => self.focused_section = FocusedSection::Sessions,
                FocusedSection::Rce => self.focused_section = FocusedSection::Shell,
            },

            _ => match self.focused_section {
                FocusedSection::Sessions => match key_event.code {
                    KeyCode::Char('l') => {
                        unimplemented!();
                    }
                    _ => {}
                },
                FocusedSection::Shell => {
                    self.shell_section
                        .handle_keys(key_event, shell_client)
                        .await?;
                }

                _ => {}
            },
        }
        Ok(())
    }
}
