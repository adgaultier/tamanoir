use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Span,
    widgets::{Block, BorderType, Cell, HighlightSpacing, Row, ScrollbarState, Table, TableState},
    Frame,
};

use crate::{
    app::{AppResult, SessionsMap},
    grpc::SessionServiceClient,
    tamanoir_grpc::SessionResponse,
};

#[derive(Debug)]
pub struct SessionSection {
    sessions: SessionsMap,
    state: TableState,
    scroll_state: ScrollbarState,
    pub selected_session: Option<SessionResponse>,
}
impl SessionSection {
    pub fn new(sessions: SessionsMap) -> Self {
        Self {
            sessions,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),

            selected_session: None,
        }
    }
    pub async fn init(&mut self, client: &mut SessionServiceClient) -> AppResult<()> {
        for s in client.list_sessions().await? {
            self.sessions.write().unwrap().insert(s.ip.clone(), s);
        }
        self.selected_session = self.sessions.read().unwrap().values().nth(0).cloned();
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
        self.selected_session = if let Some((_, v)) = self.sessions.read().unwrap().iter().nth(i) {
            Some(v.clone())
        } else {
            None
        };
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
        self.selected_session = self.sessions.read().unwrap().values().nth(i).cloned();
    }
    pub fn unselect(&mut self) {
        self.state.select(None);
        self.scroll_state = self
            .scroll_state
            .position(self.sessions.read().unwrap().len());
    }

    pub fn render(&mut self, frame: &mut Frame, block: Rect, is_focused: bool) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::LightBlue);

        let reader = self.sessions.read().unwrap();

        let rows = reader
            .iter()
            .map(|(k, _)| Row::new([Cell::from(k.clone())]).style(Style::new().fg(Color::White)));

        let highlight_color = if is_focused {
            Color::Yellow
        } else {
            Color::Blue
        };
        let table: Table<'_> = Table::new(rows, vec![Constraint::Percentage(100)])
            .row_highlight_style(selected_row_style)
            .highlight_spacing(HighlightSpacing::Never)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(highlight_color))
                    .title(Span::styled(
                        "Current Sessions",
                        Style::default().fg(highlight_color).bold(),
                    )),
            );

        frame.render_stateful_widget(table, block, &mut self.state);
    }

    pub async fn handle_keys(&mut self, key_event: KeyEvent) -> AppResult<()> {
        match key_event.code {
            KeyCode::Enter => {}
            KeyCode::Char('j') | KeyCode::Down => self.next_row(),
            KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
            _ => {}
        }
        Ok(())
    }
}
