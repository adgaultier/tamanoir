pub mod utils;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Margin, Rect},
    style::{Modifier, Style, Stylize},
    text::Text,
    widgets::{
        Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState,
    },
    Frame,
};
use utils::{TableColors, PALETTES};

use crate::{
    app::{AppResult, SessionsMap},
    grpc::SessionServiceClient,
    tamanoir_grpc::SessionResponse,
};

#[derive(Debug)]
pub struct SessionsSection {
    sessions: SessionsMap,
    colors: TableColors,
    state: TableState,
    scroll_state: ScrollbarState,
    color_index: usize,
}
impl SessionsSection {
    pub fn new(sessions: SessionsMap) -> Self {
        Self {
            sessions,
            colors: TableColors::new(&PALETTES[2]),
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            color_index: 0,
        }
    }
    pub async fn init(&mut self, client: &mut SessionServiceClient) -> AppResult<()> {
        for s in client.list_sessions().await? {
            self.sessions.write().unwrap().insert(s.ip.clone(), s);
        }
        Ok(())
    }

    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.sessions.read().unwrap().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.sessions.read().unwrap().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }
    pub fn unselect(&mut self) {
        self.state.select(None);
        self.scroll_state = self
            .scroll_state
            .position(self.sessions.read().unwrap().len());
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
    pub fn render(&mut self, frame: &mut Frame, block: Rect) {
        self.set_colors();

        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let bar = " █ ";

        let reader = self.sessions.read().unwrap();

        let rows = reader.iter().enumerate().map(|(idx, (k, v))| {
            let color = match idx % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };

            match v.parse_keycodes(utils::Layout::Azerty) {
                Err(e) => Row::new([
                    Cell::from(k.clone()),
                    Cell::from(format!("parsing error: {}", e)),
                ])
                .style(Style::new().fg(self.colors.row_fg).bg(color)),
                Ok(keycodes) => {
                    let keys = SessionResponse::format_keys(keycodes);
                    Row::new([Cell::from(k.clone()), Cell::from(keys)])
                        .style(Style::new().fg(self.colors.row_fg).bg(color))
                }
            }
        });

        let table: Table<'_> = Table::new(
            rows,
            vec![Constraint::Length(12), Constraint::Percentage(100)],
        )
        .row_highlight_style(selected_row_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(self.colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Never);

        frame.render_stateful_widget(table, block, &mut self.state);

        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            block.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    pub async fn handle_keys(
        &mut self,
        key_event: KeyEvent,
        _sessions_client: &mut SessionServiceClient,
    ) -> AppResult<()> {
        match key_event.code {
            KeyCode::Enter => {}
            KeyCode::Char('j') | KeyCode::Down => self.next_row(),
            KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
            _ => {}
        }
        Ok(())
    }
}
