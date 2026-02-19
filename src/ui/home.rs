use ratatui::prelude::*;
use ratatui::widgets::*;

const BANNER: &str = r#"
 ╔═══════════════════════════════════════════════════════════════════╗
 ║  ██████╗ ██╗   ██╗███████╗████████╗ ██████╗ █████╗ ██████╗ ███████╗ ║
 ║  ██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔════╝██╔══██╗██╔══██╗██╔════╝ ║
 ║  ██████╔╝██║   ██║███████╗   ██║   ██║     ███████║██║  ██║█████╗   ║
 ║  ██╔══██╗██║   ██║╚════██║   ██║   ██║     ██╔══██║██║  ██║██╔══╝   ║
 ║  ██║  ██║╚██████╔╝███████║   ██║   ╚██████╗██║  ██║██████╔╝███████╗ ║
 ║  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝╚═╝  ╚═╝╚═════╝ ╚══════╝ ║
 ╚═══════════════════════════════════════════════════════════════════╝"#;

pub fn render_home(frame: &mut Frame, area: Rect) {
    // Use proportional layout that adapts to window size
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Banner
            Constraint::Length(2),  // Subtitle
            Constraint::Min(10),   // Games list
            Constraint::Min(12),   // Controls
            Constraint::Length(2),  // Footer
        ])
        .split(area);

    // Banner
    let banner = Paragraph::new(BANNER)
        .style(Style::default().fg(Color::Rgb(80, 200, 255)))
        .alignment(Alignment::Center);
    frame.render_widget(banner, chunks[0]);

    // Subtitle
    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled(
            "  ⚡ Your Terminal Arcade ⚡  ",
            Style::default()
                .fg(Color::Rgb(255, 220, 80))
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        ),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(subtitle, chunks[1]);

    // Games list
    let games = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [1] ", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("🐸 Frogger    ", Style::default().fg(Color::Rgb(80, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("─ Help the frog cross the road and river!", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [2] ", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("🧱 Breakout   ", Style::default().fg(Color::Rgb(220, 80, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("─ Smash all the bricks with the bouncing ball!", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [3] ", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("🦖 Dino Run   ", Style::default().fg(Color::Rgb(200, 120, 255)).add_modifier(Modifier::BOLD)),
            Span::styled("─ Jump over obstacles in this endless runner!", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [4] ", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("🎱 Pinball    ", Style::default().fg(Color::Rgb(255, 200, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("─ Launch the ball and hit bumpers for combos!", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [5] ", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("🟦 JezzBall   ", Style::default().fg(Color::Rgb(100, 180, 255)).add_modifier(Modifier::BOLD)),
            Span::styled("─ Build walls to trap the bouncing balls!", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [6] ", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("⚛ Beam       ", Style::default().fg(Color::Rgb(120, 200, 255)).add_modifier(Modifier::BOLD)),
            Span::styled("─ Steer a particle beam around an accelerator ring!", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(60, 150, 200)))
            .title(" 🎮 Games ")
            .title_style(Style::default().fg(Color::Rgb(200, 120, 255)).add_modifier(Modifier::BOLD)),
    )
    .alignment(Alignment::Center);
    frame.render_widget(games, chunks[2]);

    // Controls
    let controls = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  🔧 Navigation", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    Tab / Shift+Tab  ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Switch between tabs", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    1-6              ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Quick-launch a game", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    Esc              ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Return to Home", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    q / Ctrl+C       ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Quit RustCade", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🎮 In Games", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    Arrow Keys       ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Move / Control", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    Space            ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Action (Jump / Launch)", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Restart current game", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    P                ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Pause / Unpause", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(60, 150, 200)))
            .title(" ⌨ Controls ")
            .title_style(Style::default().fg(Color::Rgb(200, 120, 255)).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(controls, chunks[3]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "  Built with ❤ using Rust + Ratatui  ",
            Style::default().fg(Color::Rgb(80, 80, 100)).add_modifier(Modifier::ITALIC),
        ),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[4]);
}
