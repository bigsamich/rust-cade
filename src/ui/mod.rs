pub mod home;
pub mod tabs;

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Tab};
use crate::games::Game;
use crate::scores::GAME_NAMES;

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
        Tab::Home => home::render_home(frame, chunks[1], app.selected_game, app.show_high_scores, &app.high_scores),
        Tab::Frogger => app.frogger.render(frame, chunks[1]),
        Tab::Breakout => app.breakout.render(frame, chunks[1]),
        Tab::DinoRun => app.dino_run.render(frame, chunks[1]),
        Tab::Pinball => app.pinball.render(frame, chunks[1]),
        Tab::JezzBall => app.jezzball.render(frame, chunks[1]),
        Tab::Booster => app.booster.render(frame, chunks[1]),
    }

    // Name entry overlay (renders on top of everything)
    if app.entering_name {
        render_name_entry(frame, frame.area(), &app.name_buffer, app.name_game_idx, app.name_score);
    }
}

fn render_name_entry(frame: &mut Frame, area: Rect, name_buffer: &str, game_idx: usize, score: u32) {
    let overlay_w = 44u16.min(area.width.saturating_sub(4));
    let overlay_h = 13u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(overlay_w)) / 2;
    let y = area.y + (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    // Clear background
    frame.render_widget(Clear, overlay_area);

    let game_name = if game_idx < GAME_NAMES.len() {
        GAME_NAMES[game_idx]
    } else {
        "Unknown"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Rgb(255, 220, 80)))
        .title(" 🏆 NEW HIGH SCORE! ")
        .title_style(Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));
    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    // Build the name input display: show typed chars + underscores for remaining
    let max_len = 9;
    let typed_len = name_buffer.chars().count();
    let remaining = max_len - typed_len;
    let display_name = format!("{}{}", name_buffer, "_".repeat(remaining));

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  🎮 ", Style::default()),
            Span::styled(game_name, Style::default().fg(Color::Rgb(80, 200, 255)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  Score: {}", score), Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter your name:", Style::default().fg(Color::Rgb(180, 180, 200))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    [ ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled(&display_name, Style::default().fg(Color::Rgb(255, 255, 255)).add_modifier(Modifier::BOLD)),
            Span::styled(" ]", Style::default().fg(Color::Rgb(100, 100, 130))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Rgb(80, 200, 255)).add_modifier(Modifier::BOLD)),
            Span::styled(" confirm  ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled("Esc", Style::default().fg(Color::Rgb(80, 200, 255)).add_modifier(Modifier::BOLD)),
            Span::styled(" skip", Style::default().fg(Color::Rgb(100, 100, 130))),
        ]),
    ];

    let p = Paragraph::new(lines).style(Style::default().bg(Color::Rgb(15, 15, 25)));
    frame.render_widget(p, inner);
}
