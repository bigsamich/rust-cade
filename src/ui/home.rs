use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::scores::{HighScores, GAME_NAMES};

const BANNER: &str = r#"
 ╔═════════════════════════════════════════════════════════════════════════════╗
 ║  ██████╗ ██╗   ██╗███████╗████████╗         ██████╗ █████╗ ██████╗ ███████╗ ║
 ║  ██╔══██╗██║   ██║██╔════╝╚══██╔══╝         ██╔════╝██╔══██╗██╔══██╗██╔════╝ ║
 ║  ██████╔╝██║   ██║███████╗   ██║   ███████╗ ██║     ███████║██║  ██║█████╗   ║
 ║  ██╔══██╗██║   ██║╚════██║   ██║   ╚══════╝ ██║     ██╔══██║██║  ██║██╔══╝   ║
 ║  ██║  ██║╚██████╔╝███████║   ██║            ╚██████╗██║  ██║██████╔╝███████╗ ║
 ║  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝             ╚═════╝╚═╝  ╚═╝╚═════╝ ╚══════╝ ║
 ╚═════════════════════════════════════════════════════════════════════════════╝"#;

struct GameTile {
    key: &'static str,
    icon: &'static str,
    name: &'static str,
    desc: &'static str,
    color: Color,
    border_color: Color,
}

const GAME_TILES: [GameTile; 8] = [
    GameTile { key: "1", icon: "🐸", name: "Frogger", desc: "Cross the road\nand river!", color: Color::Rgb(80, 220, 80), border_color: Color::Rgb(40, 120, 40) },
    GameTile { key: "2", icon: "🧱", name: "Breakout", desc: "Smash bricks\nwith the ball!", color: Color::Rgb(220, 80, 80), border_color: Color::Rgb(120, 40, 40) },
    GameTile { key: "3", icon: "🦖", name: "Dino Run", desc: "Jump obstacles\nin endless run!", color: Color::Rgb(200, 120, 255), border_color: Color::Rgb(100, 60, 140) },
    GameTile { key: "4", icon: "👾", name: "Invaders", desc: "Defend Earth\nfrom aliens!", color: Color::Rgb(80, 255, 80), border_color: Color::Rgb(40, 140, 40) },
    GameTile { key: "5", icon: "🟦", name: "JezzBall", desc: "Build walls to\ntrap the balls!", color: Color::Rgb(100, 180, 255), border_color: Color::Rgb(50, 90, 140) },
    GameTile { key: "6", icon: "☄", name: "Asteroids", desc: "Shoot rocks\nin deep space!", color: Color::Rgb(100, 200, 255), border_color: Color::Rgb(50, 100, 140) },
    GameTile { key: "7", icon: "⚛", name: "Booster", desc: "Steer particles\naround the ring!", color: Color::Rgb(120, 200, 255), border_color: Color::Rgb(50, 100, 140) },
    GameTile { key: "8", icon: "💫", name: "Beam", desc: "Tune the ring\nfor 5 orbits!", color: Color::Rgb(255, 160, 60), border_color: Color::Rgb(140, 80, 30) },
];

fn render_game_tile(frame: &mut Frame, area: Rect, tile: &GameTile, selected: bool) {
    let border_color = if selected { Color::Rgb(255, 220, 80) } else { tile.border_color };
    let border_type = if selected { BorderType::Double } else { BorderType::Rounded };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 { return; }

    let mut lines: Vec<Line> = Vec::new();

    // Key + Icon + Name line
    let name_color = if selected { Color::Rgb(255, 255, 255) } else { tile.color };
    lines.push(Line::from(vec![
        Span::styled(format!("[{}] ", tile.key), Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", tile.icon), Style::default()),
        Span::styled(tile.name, Style::default().fg(name_color).add_modifier(Modifier::BOLD)),
    ]));

    // Description lines
    for desc_line in tile.desc.split('\n') {
        lines.push(Line::from(vec![
            Span::styled(desc_line, Style::default().fg(if selected { Color::Rgb(180, 180, 200) } else { Color::Rgb(120, 120, 140) })),
        ]));
    }

    // Selected indicator
    if selected {
        lines.push(Line::from(vec![
            Span::styled("▶ Enter to play", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        ]));
    }

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(p, inner);
}

fn game_controls(game_idx: usize) -> Vec<Line<'static>> {
    match game_idx {
        0 => vec![ // Frogger
            Line::from(""),
            Line::from(vec![
                Span::styled("  🐸 Frogger", Style::default().fg(Color::Rgb(80, 220, 80)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Help the frog cross safely!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    ↑ ↓ ← →         ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Move frog", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Restart", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    P                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Pause", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        1 => vec![ // Breakout
            Line::from(""),
            Line::from(vec![
                Span::styled("  🧱 Breakout", Style::default().fg(Color::Rgb(220, 80, 80)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Smash all the bricks!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    ← / →            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Move paddle", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    Space            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Launch ball", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Restart", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    P                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Pause", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        2 => vec![ // Dino Run
            Line::from(""),
            Line::from(vec![
                Span::styled("  🦖 Dino Run", Style::default().fg(Color::Rgb(200, 120, 255)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Endless runner — dodge everything!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    Space / ↑        ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Jump", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    ↓                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Duck", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Restart", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    P                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Pause", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        3 => vec![ // Space Invaders
            Line::from(""),
            Line::from(vec![
                Span::styled("  \u{1f47e} Space Invaders", Style::default().fg(Color::Rgb(80, 255, 80)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Defend Earth from alien waves!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    \u{2190} / \u{2192}            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Move cannon", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    Space / \u{2191}        ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Shoot", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Restart", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    P                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Pause", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        4 => vec![ // JezzBall
            Line::from(""),
            Line::from(vec![
                Span::styled("  🟦 JezzBall", Style::default().fg(Color::Rgb(100, 180, 255)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Build walls to trap balls!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    ↑ ↓ ← →         ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Move cursor", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    Space            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Place wall", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Restart", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    P                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Pause", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        5 => vec![ // Asteroids
            Line::from(""),
            Line::from(vec![
                Span::styled("  \u{2604} Asteroids", Style::default().fg(Color::Rgb(100, 200, 255)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Blast asteroids in deep space!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    \u{2190} / \u{2192}            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Rotate ship", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    \u{2191}                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Thrust", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    Space            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Shoot", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Restart", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    P                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Pause", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        6 => vec![ // Booster
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⚛ Booster", Style::default().fg(Color::Rgb(120, 200, 255)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Tune magnets, steer a beam", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(vec![
                Span::styled("  5 turns, lowest score wins!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    ↑ / ↓            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Select magnet", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    ← / →            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Adjust power", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    [ / ]            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Prev/next section", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    + / -            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Step size", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    0-9              ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Ramp point", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    B                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Bump mode (3/4/5/off)", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    C                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Copy section to all", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    Z                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Zero magnet", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    D                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Difficulty toggle", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    W/S  E/Q         ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Bump X / Y only", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        7 => vec![ // Beam
            Line::from(""),
            Line::from(vec![
                Span::styled("  💫 Beam", Style::default().fg(Color::Rgb(255, 160, 60)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  Tune magnets, steer a beam", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(vec![
                Span::styled("  5 turns, lowest score wins!", Style::default().fg(Color::Rgb(100, 100, 120))),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    ↑ / ↓            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Select magnet", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    ← / →            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Adjust power", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    [ / ]            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Prev/next section", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    + / -            ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Step size", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    0-9              ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Ramp point", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    B                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Bump mode (3/4/5/off)", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    C                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Copy section to all", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    Z                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Zero magnet", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    D                ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Difficulty toggle", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
            Line::from(vec![
                Span::styled("    W/S  E/Q         ", Style::default().fg(Color::Rgb(80, 200, 255))),
                Span::styled("Bump X / Y only", Style::default().fg(Color::Rgb(140, 140, 140))),
            ]),
        ],
        _ => vec![],
    }
}

pub fn render_home(frame: &mut Frame, area: Rect, selected_game: usize, show_high_scores: bool, high_scores: &HighScores) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Banner
            Constraint::Length(2),  // Subtitle
            Constraint::Length(12), // Game tiles (2 rows)
            Constraint::Min(10),   // Controls area
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

    // Games section title block
    let games_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 150, 200)))
        .title(" 🎮 Games — ↑↓←→ Select, Enter to Play ")
        .title_style(Style::default().fg(Color::Rgb(200, 120, 255)).add_modifier(Modifier::BOLD));
    let games_inner = games_block.inner(chunks[2]);
    frame.render_widget(games_block, chunks[2]);

    // 2 rows of tiles: 4 on top, 3 on bottom
    let tile_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
        ])
        .split(games_inner);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(tile_rows[0]);

    let bot_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(tile_rows[1]);

    for i in 0..4 {
        render_game_tile(frame, top_cols[i], &GAME_TILES[i], selected_game == i);
    }
    for i in 0..4 {
        render_game_tile(frame, bot_cols[i], &GAME_TILES[i + 4], selected_game == i + 4);
    }

    // Controls area: split horizontally - navigation left, game controls right
    let ctrl_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(chunks[3]);

    // Navigation Control (left)
    let controls = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  🔧 Navigation", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    Tab / Shift+Tab  ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Switch tabs", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    1-8              ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Launch game", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    ↑ ↓ ← →         ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Select game", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    Enter            ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Play selected", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    Esc              ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Return to Home", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(vec![
            Span::styled("    q / Ctrl+C       ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Quit", Style::default().fg(Color::Rgb(140, 140, 140))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  🎮 Common", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("    R                ", Style::default().fg(Color::Rgb(80, 200, 255))),
            Span::styled("Restart game", Style::default().fg(Color::Rgb(140, 140, 140))),
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
            .title(" ⌨ Navigation Control ")
            .title_style(Style::default().fg(Color::Rgb(200, 120, 255)).add_modifier(Modifier::BOLD)),
    );
    frame.render_widget(controls, ctrl_cols[0]);

    // Game Control (right) - shows controls for the selected game
    let game_ctrl_lines = game_controls(selected_game);
    let game_ctrl = Paragraph::new(game_ctrl_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(50, 100, 140)))
                .title(format!(" 🎮 {} Control ", GAME_TILES[selected_game].name))
                .title_style(Style::default().fg(GAME_TILES[selected_game].color).add_modifier(Modifier::BOLD)),
        );
    frame.render_widget(game_ctrl, ctrl_cols[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  🦀 ", Style::default().fg(Color::Rgb(255, 100, 50))),
        Span::styled("v0.10.1", Style::default().fg(Color::Rgb(80, 80, 100))),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(40, 40, 60))),
        Span::styled("H", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        Span::styled(" High Scores", Style::default().fg(Color::Rgb(100, 100, 130))),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[4]);

    // High scores overlay
    if show_high_scores {
        render_high_scores_overlay(frame, area, high_scores);
    }
}

fn render_high_scores_overlay(frame: &mut Frame, area: Rect, high_scores: &HighScores) {
    // Center overlay
    let overlay_w = 50u16.min(area.width.saturating_sub(4));
    let overlay_h = 30u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(overlay_w)) / 2;
    let y = area.y + (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    // Clear background
    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Rgb(255, 200, 80)))
        .title(" 🏆 High Scores ")
        .title_style(Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));
    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let icons = ["🐸", "🧱", "🦖", "👾", "🟦", "☄", "⚛", "💫"];
    let colors = [
        Color::Rgb(80, 220, 80),
        Color::Rgb(220, 80, 80),
        Color::Rgb(200, 120, 255),
        Color::Rgb(80, 255, 80),
        Color::Rgb(100, 180, 255),
        Color::Rgb(100, 200, 255),
        Color::Rgb(120, 200, 255),
        Color::Rgb(255, 160, 60),
    ];
    let medal_colors = [
        Color::Rgb(255, 215, 0),   // Gold
        Color::Rgb(192, 192, 192), // Silver
        Color::Rgb(205, 127, 50),  // Bronze
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for game_idx in 0..8 {
        let scores = high_scores.top_scores(game_idx);
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", icons[game_idx]), Style::default()),
            Span::styled(
                GAME_NAMES[game_idx],
                Style::default().fg(colors[game_idx]).add_modifier(Modifier::BOLD),
            ),
        ]));

        let has_any = scores.iter().any(|e| e.score > 0);
        if has_any {
            for rank in 0..3 {
                if scores[rank].score > 0 {
                    let medal = match rank {
                        0 => "🥇",
                        1 => "🥈",
                        _ => "🥉",
                    };
                    let name_display = if scores[rank].name.is_empty() {
                        "???".to_string()
                    } else {
                        format!("{:<9}", scores[rank].name)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("    {} ", medal), Style::default()),
                        Span::styled(
                            format!("{} ", name_display),
                            Style::default().fg(Color::Rgb(200, 200, 220)),
                        ),
                        Span::styled(
                            format!("{}", scores[rank].score),
                            Style::default().fg(medal_colors[rank]).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled("    No scores yet", Style::default().fg(Color::Rgb(60, 60, 80))),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(Color::Rgb(80, 80, 100))),
        Span::styled("H", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
        Span::styled(" to close", Style::default().fg(Color::Rgb(80, 80, 100))),
    ]));

    let p = Paragraph::new(lines).style(Style::default().bg(Color::Rgb(15, 15, 25)));
    frame.render_widget(p, inner);
}
