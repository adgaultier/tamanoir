use std::sync::{Arc, RwLock};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{palette::tailwind, Color, Modifier, Style, Stylize},
    text::Text,
    widgets::{
        Block, BorderType, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{app::AppResult, grpc::RemoteShellServiceClient};
pub type ShellCmdHistory = Arc<RwLock<Vec<ShellCmd>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum ShellStdType {
    StdIn,
    StdOut,
}
#[derive(Debug, Clone)]
pub struct ShellCmd {
    pub std_type: ShellStdType,
    pub inner: String,
}
#[derive(Debug)]
struct TableColors {
    buffer_bg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}
impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}
const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

#[derive(Debug)]
pub struct ShellSection {
    pub shell_input: Input,
    colors: TableColors,
    state: TableState,
    scroll_state: ScrollbarState,
    color_index: usize,
    items: ShellCmdHistory,
    history_index: usize,
}

impl ShellSection {
    pub fn new(app_shell: ShellCmdHistory) -> Self {
        Self {
            shell_input: Input::default(),
            colors: TableColors::new(&PALETTES[2]),
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            color_index: 0,
            items: app_shell,
            history_index: 0,
        }
    }
    fn compute_scroll_height(&self, idx: usize) -> usize {
        self.items
            .read()
            .unwrap()
            .iter()
            .take(idx)
            .fold(0, |acc, x| {
                acc + x.inner.chars().filter(|&c| c == '\n').count() + 1
            })
    }
    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.read().unwrap().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(self.compute_scroll_height(i));
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.read().unwrap().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(self.compute_scroll_height(i));
    }
    pub fn unselect(&mut self) {
        self.state.select(None);
        self.scroll_state = self
            .scroll_state
            .position(self.compute_scroll_height(self.items.read().unwrap().len()));
    }
    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }
    fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }
    fn get_stdin_history(&self) -> Vec<String> {
        self.items
            .read()
            .unwrap()
            .iter()
            .filter(|cmd| cmd.std_type == ShellStdType::StdIn)
            .map(|cmd| cmd.inner.clone())
            .collect::<Vec<String>>()
    }
    pub fn render(&mut self, frame: &mut Frame, block: Rect) {
        self.set_colors();

        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let bar = " █ ";
        let (shell_history_block, input_block) = {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(3)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(block);

            (chunks[0], chunks[1])
        };
        let reader = self.items.read().unwrap();

        let rows = reader.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };

            let length = data.inner.chars().filter(|&c| c == '\n').count() + 1;
            let text = match data.std_type {
                ShellStdType::StdIn => format!("{}", data.inner),
                ShellStdType::StdOut => format!("{}", data.inner),
            };
            Row::new([text])
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(length as u16)
        });

        let table: Table<'_> = Table::new(rows, vec![Constraint::Percentage(100)])
            .row_highlight_style(selected_row_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Never);

        frame.render_stateful_widget(table, shell_history_block, &mut self.state);

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            shell_history_block.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
        let cmd_input = Paragraph::new(Text::from(format!("{}", self.shell_input.value())))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .left_aligned()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );

        frame.render_widget(cmd_input, input_block);
    }

    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        shell_client: &mut RemoteShellServiceClient,
    ) -> AppResult<()> {
        if key_event.modifiers.contains(KeyModifiers::SHIFT) {
            match key_event.code {
                KeyCode::Char('j') | KeyCode::Down => self.next_row(),
                KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
                KeyCode::Char('l') | KeyCode::Right => self.next_color(),
                KeyCode::Char('h') | KeyCode::Left => self.previous_color(),

                _ => {}
            }
        }
        match key_event.code {
            KeyCode::Enter => {
                shell_client.send_cmd(self.shell_input.to_string()).await?;
                self.shell_input.reset();
                self.history_index = self.get_stdin_history().len();
                self.unselect();
            }
            KeyCode::Up => {
                self.history_index = self.history_index.saturating_sub(1);
                if let Some(cmd) = self.get_stdin_history().get(self.history_index) {
                    self.shell_input = self.shell_input.clone().with_value(cmd.clone());
                }
            }
            KeyCode::Down => {
                let current_idx: usize = self.get_stdin_history().len();
                self.history_index = current_idx.min(self.history_index + 1);
                if self.history_index < current_idx {
                    if let Some(cmd) = self.get_stdin_history().get(self.history_index) {
                        self.shell_input = self.shell_input.clone().with_value(cmd.clone());
                    }
                } else {
                    self.shell_input.reset();
                }
            }
            _ => {
                self.shell_input.handle_event(&Event::Key(key_event));
            }
        }
        Ok(())
    }
}
