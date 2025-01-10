use ratatui::Frame;

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    app.sections.render(frame);
}
