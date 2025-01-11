use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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

fn compute_payload_tx_pct(total_len: u32, remaining_len: u32) -> u8 {
    ((total_len.saturating_sub(remaining_len) as f32 / total_len as f32) * 100f32).min(100f32) as u8
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
        let (session_selection, session_info) = {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(18), Constraint::Fill(1)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(block);
            (chunks[0], chunks[1])
        };
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

        frame.render_stateful_widget(table, session_selection, &mut self.state);
        if let Some(s) = &self.selected_session {
            let (stats, rce) = {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Fill(1), Constraint::Fill(1)])
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .split(session_info);
                (chunks[0], chunks[1])
            };
            let cells_stats = vec![
                Cell::from(Span::from("Received Packets:").bold()),
                Cell::from(Span::from(s.n_packets.to_string())),
                Cell::from(Span::from("First Packet:").bold()),
                Cell::from(Span::from(s.first_packet.clone())),
                Cell::from(Span::from("Latest Packet:").bold()),
                Cell::from(Span::from(s.latest_packet.clone())),
                Cell::from(Span::from("Shell Status:").bold()),
                Cell::from(Span::from(format!("{}", s.get_shell_status()))),
            ];
            let rows_stats: Vec<Row> = cells_stats
                .chunks(2)
                .map(|r| Row::new(r.to_vec()))
                .collect();
            let table_stats: Table<'_> =
                Table::new(rows_stats, [Constraint::Fill(1), Constraint::Fill(1)]).block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Style::new().fg(Color::Blue))
                        .title(Span::styled(
                            "Session Info",
                            Style::default().fg(Color::Blue).bold(),
                        )),
                );

            let rce_info: (String, String, String, String) = match &s.rce_payload {
                None => ("".into(), "".into(), "".into(), "".into()),
                Some(rce) => (
                    rce.name.clone(),
                    format!("{} bytes", rce.length),
                    rce.target_arch.clone(),
                    format!(
                        "{} %",
                        compute_payload_tx_pct(rce.length, rce.buffer_length)
                    ),
                ),
            };
            let cells_rce = vec![
                Cell::from(Span::from("Selected Payload:").bold()),
                Cell::from(Span::from(rce_info.0)),
                Cell::from(Span::from("Size:").bold()),
                Cell::from(Span::from(rce_info.1)),
                Cell::from(Span::from("Target Arch:").bold()),
                Cell::from(Span::from(rce_info.2)),
                Cell::from(Span::from("Tx Status:").bold()),
                Cell::from(Span::from(rce_info.3)),
            ];
            let rows_rce: Vec<Row> = cells_rce.chunks(2).map(|r| Row::new(r.to_vec())).collect();
            let table_rce = Table::new(rows_rce, [Constraint::Length(20), Constraint::Fill(1)])
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Style::new().fg(Color::Blue))
                        .title(Span::styled(
                            "Payload Transmission",
                            Style::default().fg(Color::Blue).bold(),
                        )),
                );
            frame.render_widget(table_stats, stats);
            frame.render_widget(table_rce, rce);
        } else {
            let table: Table<'_> = Table::new(
                [Row::new([Cell::from("No session available")])
                    .style(Style::new().fg(Color::White))],
                [Constraint::Fill(1)],
            )
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(Color::Blue))
                    .title(Span::styled(
                        "Session Info",
                        Style::default().fg(Color::Blue).bold(),
                    )),
            );

            frame.render_widget(table, session_info);
        };
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
