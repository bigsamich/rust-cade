use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Tab};

pub fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.current_tab {
                Style::default()
                    .fg(Color::Rgb(255, 220, 80))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(120, 120, 140))
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 150, 200)))
                .border_type(BorderType::Rounded)
                .title(" 🕹 RustCade ")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(200, 120, 255))
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .select(app.current_tab.index())
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Rgb(255, 220, 80))
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" │ ", Style::default().fg(Color::Rgb(60, 60, 80))));

    frame.render_widget(tabs, area);
}
