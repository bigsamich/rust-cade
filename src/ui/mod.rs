pub mod home;
pub mod tabs;

use ratatui::prelude::*;

use crate::app::{App, Tab};
use crate::games::Game;

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),   // Content
        ])
        .split(frame.area());

    tabs::render_tabs(frame, app, chunks[0]);

    match app.current_tab {
        Tab::Home => home::render_home(frame, chunks[1], app.selected_game),
        Tab::Frogger => app.frogger.render(frame, chunks[1]),
        Tab::Breakout => app.breakout.render(frame, chunks[1]),
        Tab::DinoRun => app.dino_run.render(frame, chunks[1]),
        Tab::Pinball => app.pinball.render(frame, chunks[1]),
        Tab::JezzBall => app.jezzball.render(frame, chunks[1]),
        Tab::Beam => app.beam.render(frame, chunks[1]),
    }
}
