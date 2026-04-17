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
        Tab::SpaceInvaders => app.space_invaders.render(frame, chunks[1]),
        Tab::JezzBall => app.jezzball.render(frame, chunks[1]),
        Tab::Asteroids => app.asteroids.render(frame, chunks[1]),
        Tab::Booster => app.booster.render(frame, chunks[1]),
        Tab::Beam => app.beam.render(frame, chunks[1]),
    }

    // Help overlay (renders on top of everything)
    if app.show_help {
        render_help_overlay(frame, frame.area(), &app.current_tab, app.help_scroll);
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

fn help_key(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<18}", key), Style::default().fg(Color::Rgb(80, 200, 255))),
        Span::styled(desc.to_string(), Style::default().fg(Color::Rgb(170, 170, 190))),
    ])
}

fn help_section(title: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {}", title), Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
    ])
}

fn help_text(text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {}", text), Style::default().fg(Color::Rgb(140, 140, 160))),
    ])
}

fn help_blank() -> Line<'static> {
    Line::from("")
}

fn help_lines_for_tab(tab: &Tab) -> Vec<Line<'static>> {
    match tab {
        Tab::Home => vec![
            help_section("Rust-Cade Arcade"),
            help_blank(),
            help_text("Select a game from the home screen and jump in!"),
            help_blank(),
            help_section("Navigation"),
            help_key("1-8", "Quick-launch game by number"),
            help_key("Arrow keys", "Select game tile"),
            help_key("Enter", "Play selected game"),
            help_key("Tab / Shift+Tab", "Switch between game tabs"),
            help_key("Esc", "Return to Home from any game"),
            help_key("H", "Toggle high scores display"),
            help_key("?", "Show this help screen"),
            help_key("Q / Ctrl+C", "Quit"),
            help_blank(),
            help_section("Common In-Game Controls"),
            help_key("P", "Pause / Unpause"),
            help_key("R", "Reset / Restart"),
            help_key("Enter / Space", "Restart after game over"),
        ],
        Tab::Frogger => vec![
            help_section("Frogger"),
            help_blank(),
            help_text("Guide the frog from the bottom to the goal pads at the top."),
            help_text("Cross busy roads without getting hit by cars."),
            help_text("Hop onto floating logs to cross the river -- don't fall in!"),
            help_text("Reach all 5 goal pads to win. You have 3 lives."),
            help_blank(),
            help_section("Scoring"),
            help_text("+10 pts per upward hop"),
            help_text("+100 pts per goal pad reached"),
            help_text("+500 pts for reaching all 5 goals"),
            help_blank(),
            help_section("Controls"),
            help_key("Up", "Move frog up (toward goals)"),
            help_key("Down", "Move frog down"),
            help_key("Left / Right", "Move frog sideways"),
            help_key("P", "Pause"),
            help_key("R", "Restart"),
        ],
        Tab::Breakout => vec![
            help_section("Breakout"),
            help_blank(),
            help_text("Bounce the ball off your paddle to destroy all bricks."),
            help_text("6 rows of colored bricks with increasing point values."),
            help_text("Top rows are worth more. Ball speed increases as you go."),
            help_text("You have 3 lives. Lose one each time the ball falls."),
            help_blank(),
            help_section("Scoring"),
            help_text("Top row (red): 60 pts   |  Row 2: 50 pts"),
            help_text("Row 3: 40 pts           |  Row 4: 30 pts"),
            help_text("Row 5: 20 pts           |  Bottom row: 10 pts"),
            help_blank(),
            help_section("Tips"),
            help_text("Hit the ball near paddle edges for sharper angles."),
            help_text("Clear all bricks to win!"),
            help_blank(),
            help_section("Controls"),
            help_key("Left / Right", "Move paddle"),
            help_key("Space / Up", "Launch ball"),
            help_key("P", "Pause"),
            help_key("R", "Restart"),
        ],
        Tab::DinoRun => vec![
            help_section("Dino Run"),
            help_blank(),
            help_text("Classic endless runner -- the dino runs automatically."),
            help_text("Jump over cacti and duck under birds to survive."),
            help_text("Speed increases gradually up to 1.5x as you progress."),
            help_text("Birds start appearing after 200 points."),
            help_blank(),
            help_section("Scoring"),
            help_text("Score increases over time as long as you survive."),
            help_text("Higher score = faster speed = more challenge!"),
            help_blank(),
            help_section("Controls"),
            help_key("Space / Up", "Jump (also starts game)"),
            help_key("Down", "Duck (on ground) / Fast fall (in air)"),
            help_key("P", "Pause"),
            help_key("R", "Restart"),
        ],
        Tab::SpaceInvaders => vec![
            help_section("Space Invaders"),
            help_blank(),
            help_text("Defend Earth! Destroy all 55 aliens in the formation."),
            help_text("Aliens march sideways and descend. They shoot back!"),
            help_text("4 shields protect you but degrade from both sides."),
            help_text("Clear all aliens to advance to the next level."),
            help_blank(),
            help_section("Scoring"),
            help_text("Top row aliens:    30 pts each"),
            help_text("Middle row aliens: 20 pts each"),
            help_text("Bottom row aliens: 10 pts each"),
            help_blank(),
            help_section("Game Over"),
            help_text("Lose a life when hit by an alien bullet."),
            help_text("Instant loss if aliens reach the bottom."),
            help_text("3 lives total."),
            help_blank(),
            help_section("Controls"),
            help_key("Left / Right", "Move ship"),
            help_key("Space / Up", "Fire (max 3 bullets)"),
            help_key("P", "Pause"),
            help_key("R", "Restart"),
        ],
        Tab::JezzBall => vec![
            help_section("JezzBall"),
            help_blank(),
            help_text("Trap bouncing balls by building walls across the grid."),
            help_text("Walls grow from your cursor in both directions."),
            help_text("If a ball hits a growing wall, you lose a life!"),
            help_text("Fill 75% of the grid to advance to the next level."),
            help_text("Each level adds more balls (up to 8)."),
            help_blank(),
            help_section("Scoring"),
            help_text("+10 pts per wall completed"),
            help_text("+1 pt per cell filled"),
            help_text("+100 x level for clearing a level"),
            help_blank(),
            help_section("Strategy"),
            help_text("Watch ball trajectories before placing walls."),
            help_text("Toggle wall direction to best divide the space."),
            help_text("Isolate balls into small regions."),
            help_blank(),
            help_section("Controls"),
            help_key("Arrow keys", "Move cursor"),
            help_key("Space / Enter", "Place wall"),
            help_key("D", "Toggle direction (H/V)"),
            help_key("P", "Pause"),
            help_key("R", "Restart"),
        ],
        Tab::Asteroids => vec![
            help_section("Asteroids"),
            help_blank(),
            help_text("Pilot your ship through an asteroid field."),
            help_text("Destroy all asteroids to clear the level."),
            help_text("Large asteroids split into 2 medium, medium into 2 small."),
            help_text("Your ship wraps around screen edges. So do asteroids."),
            help_text("3 lives. Brief invulnerability after each hit."),
            help_blank(),
            help_section("Scoring"),
            help_text("Large asteroids:  20 pts"),
            help_text("Medium asteroids: 50 pts"),
            help_text("Small asteroids:  100 pts"),
            help_blank(),
            help_section("Tips"),
            help_text("Use thrust sparingly -- momentum carries you."),
            help_text("Friction slowly slows you down (0.99x per tick)."),
            help_text("Max 8 bullets on screen, 5-tick fire cooldown."),
            help_blank(),
            help_section("Controls"),
            help_key("Left / Right", "Rotate ship"),
            help_key("Up", "Thrust forward"),
            help_key("Space", "Fire"),
            help_key("P", "Pause"),
            help_key("R", "Restart"),
        ],
        Tab::Beam => vec![
            help_section("Beam -- Particle Beam Simulation"),
            help_blank(),
            help_text("Keep a particle beam stable for 5 orbits around a"),
            help_text("24-section accelerator ring by tuning magnets."),
            help_blank(),
            help_section("How It Works"),
            help_text("The beam circulates through 24 sections. Each section"),
            help_text("has 6 magnets you must configure:"),
            help_blank(),
            help_text("  QF (Focus Quad)    Focuses beam in X, defocuses in Y"),
            help_text("  D1 (Dipole 1)      Bends beam around the ring"),
            help_text("  QD (Defocus Quad)  Defocuses in X, focuses in Y"),
            help_text("  D2 (Dipole 2)      Bends beam around the ring"),
            help_text("  VT (Vertical Trim) Fine vertical position correction"),
            help_text("  HT (Horiz. Trim)   Fine horizontal position correction"),
            help_blank(),
            help_section("Getting Started"),
            help_text("1. Set BOTH dipoles (D1, D2) to ~0.131 in every section."),
            help_text("   Without this, the beam flies straight into the wall!"),
            help_text("   Tip: set one section, then press C to copy to all."),
            help_text("2. Adjust QF/QD to control beam size (focusing)."),
            help_text("3. Press SPACE to start the beam."),
            help_text("4. Use trims (VT/HT) to correct orbit if needed."),
            help_blank(),
            help_section("Loss Conditions"),
            help_text("Position beyond +/-50: instant beam loss (hard wall)."),
            help_text("Beam edges past +/-25: accumulating losses."),
            help_text("Game over when accumulated losses reach 100."),
            help_text("Red diamond markers show dynamic aperture restrictions."),
            help_blank(),
            help_section("Power Supply Ramps"),
            help_text("Each magnet has 10 ramp points (keys 0-9), one per turn."),
            help_text("This lets you program different magnet strengths per orbit."),
            help_text("Ramp values are constrained within +/-0.5 of neighbors."),
            help_blank(),
            help_section("Bump Mode (B key)"),
            help_text("Creates a controlled orbit perturbation using trim dipoles"),
            help_text("across 3, 4, or 5 consecutive sections."),
            help_text("Coefficients sum to zero so the net angle cancels out."),
            help_text("  3-bump: [+1, -2, +1]"),
            help_text("  4-bump: [+1, -1, -1, +1]"),
            help_text("  5-bump: [+1, -2, +2, -2, +1]"),
            help_blank(),
            help_section("Difficulty"),
            help_text("Easy: beam size stays constant."),
            help_text("Hard: beam grows 0.05 units per element (phase instability)."),
            help_text("Press D to toggle before starting."),
            help_blank(),
            help_section("Scoring"),
            help_text("Score = sum of |magnet powers| x 100."),
            help_text("Lower power usage = more efficient = better score!"),
            help_blank(),
            help_section("Controls"),
            help_key("Space", "Start beam / Restart after game over"),
            help_key("Up / Down", "Select magnet (or adjust bump trims)"),
            help_key("Left / Right", "Decrease / Increase magnet power"),
            help_key("[ / ]", "Jump to prev / next section"),
            help_key("+ / -", "Double / Halve power step size"),
            help_key("0-9", "Select ramp point for current turn"),
            help_key("C", "Copy current section to all sections"),
            help_key("Z", "Zero selected magnet ramp value"),
            help_key("X", "Zero all ramp values in current section"),
            help_key("B", "Toggle bump mode (off/3/4/5)"),
            help_key("W / S", "Bump: adjust H-trim only"),
            help_key("E / Q", "Bump: adjust V-trim only"),
            help_key("D", "Toggle difficulty (Easy/Hard)"),
            help_key("P", "Pause"),
            help_key("R", "Restart"),
        ],
        Tab::Booster => vec![
            help_section("Booster -- Fermilab Booster Synchrotron"),
            help_blank(),
            help_text("Accelerate protons from 400 MeV to 8 GeV in a realistic"),
            help_text("simulation of the Fermilab Booster synchrotron."),
            help_text("Navigate the critical transition energy crossing and"),
            help_text("extract the beam successfully."),
            help_blank(),
            help_section("Game Phases"),
            help_text("SETUP        Configure correctors before injection"),
            help_text("INJECT       Beam enters at 400 MeV (press SPACE)"),
            help_text("RAMP         Energy ramping, tune correction needed"),
            help_text("PRE-Xt       Approaching transition -- chromaticity critical"),
            help_text("TRANSITION   At gamma ~= gamma_t, must flip RF phase (T)"),
            help_text("POST-Xt      Damp oscillations after transition"),
            help_text("EXTRACTED!   Success -- beam reached 8 GeV!"),
            help_blank(),
            help_section("Key Concept: Transition Crossing"),
            help_text("At ~turn 7100, beam energy reaches transition gamma (5.446)."),
            help_text("The slip factor eta crosses zero -- RF phase must flip!"),
            help_text("Press T to toggle RF phase at the right moment."),
            help_text("Good chromaticity correction (sextupoles) reduces losses."),
            help_text("Target: chromaticity ~7 for clean transition."),
            help_blank(),
            help_section("Corrector Magnets (6 types per cell)"),
            help_text("H-Trim      Horizontal orbit correction (rad)"),
            help_text("V-Trim      Vertical orbit correction (rad)"),
            help_text("Trim-Quad   Quadrupole strength fine-tune (m^-2)"),
            help_text("Skew-Quad   X-Y coupling correction (m^-2)"),
            help_text("Sext-A      Chromaticity family A (m^-3)"),
            help_text("Sext-B      Chromaticity family B (m^-3)"),
            help_blank(),
            help_section("Bus Corrections"),
            help_text("MQAT (J/K)   Quad bus trim -- adjusts all quad strengths"),
            help_text("MDAT (M/N)   Bend bus trim -- adjusts all dipole angles"),
            help_blank(),
            help_section("Display Modes (V to cycle)"),
            help_text("Orbit        Turn-by-turn X-Y position plot"),
            help_text("X-X'         Horizontal phase space + Courant-Snyder ellipse"),
            help_text("Y-Y'         Vertical phase space + ellipse"),
            help_text("Longitudinal RF bucket diagram (phi vs dE)"),
            help_text("Tune         Qx-Qy working point with resonance lines"),
            help_blank(),
            help_section("Bump Mode (B)"),
            help_text("Apply coordinated trim corrections across 3/4/5 sections."),
            help_text("Coefficients cancel so the bump is localized."),
            help_text("Use W/S for H-trims, E/Q for V-trims in bump mode."),
            help_blank(),
            help_section("Loss Conditions"),
            help_text("Hard wall at +/-50 display units = instant loss."),
            help_text("Beam edges beyond +/-25 units accumulate losses."),
            help_text("Game over when losses reach 100 or intensity drops to 0."),
            help_blank(),
            help_section("Scoring"),
            help_text("Score = (intensity x 1000) + turns completed"),
            help_text("       + 500 if transition crossed + 2000 if extracted"),
            help_blank(),
            help_section("Controls -- General"),
            help_key("Space", "Inject beam"),
            help_key("I", "Inject at custom X,Y coordinates"),
            help_key("[ / ]", "Navigate cells 0-23"),
            help_key("Up / Down", "Cycle corrector type"),
            help_key("Left / Right", "Adjust selected corrector"),
            help_key("+ / -", "Double / Halve adjustment step"),
            help_key("C", "Copy cell correctors to all 24 cells"),
            help_key("Z", "Zero current corrector"),
            help_key("V", "Cycle display mode"),
            help_key(". (period)", "Cycle sim speed (Slow/Normal/Fast)"),
            help_blank(),
            help_section("Controls -- RF & Bus"),
            help_key("F / G", "Increase / Decrease RF voltage"),
            help_key("T", "Toggle RF phase (for transition)"),
            help_key("J / K", "Increase / Decrease quad bus (MQAT)"),
            help_key("M / N", "Increase / Decrease bend bus (MDAT)"),
            help_blank(),
            help_section("Controls -- Bump Mode"),
            help_key("B", "Toggle bump mode (off/3/4/5)"),
            help_key("Up / Down", "Adjust all bump trims"),
            help_key("Left / Right", "Shift bump position"),
            help_key("W / S", "Adjust H-trim only"),
            help_key("E / Q", "Adjust V-trim only"),
            help_key("Z", "Zero all bump trims"),
            help_blank(),
            help_key("P", "Pause"),
            help_key("R", "Reset (preserves corrector settings)"),
        ],
    }
}

fn render_help_overlay(frame: &mut Frame, area: Rect, tab: &Tab, scroll: u16) {
    let lines = help_lines_for_tab(tab);
    let content_height = lines.len() as u16 + 4; // +4 for border + title/footer padding

    let overlay_w = 64u16.min(area.width.saturating_sub(4));
    let overlay_h = content_height.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(overlay_w)) / 2;
    let y = area.y + (area.height.saturating_sub(overlay_h)) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let title = match tab {
        Tab::Home => " ? Help ",
        Tab::Frogger => " ? Frogger Help ",
        Tab::Breakout => " ? Breakout Help ",
        Tab::DinoRun => " ? Dino Run Help ",
        Tab::SpaceInvaders => " ? Space Invaders Help ",
        Tab::JezzBall => " ? JezzBall Help ",
        Tab::Asteroids => " ? Asteroids Help ",
        Tab::Beam => " ? Beam Help ",
        Tab::Booster => " ? Booster Help ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Rgb(80, 200, 255)))
        .title(title)
        .title_style(Style::default().fg(Color::Rgb(80, 200, 255)).add_modifier(Modifier::BOLD))
        .title_bottom(Line::from(vec![
            Span::styled(" Press ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled("?", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled(" or ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled("Esc", Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD)),
            Span::styled(" to close ", Style::default().fg(Color::Rgb(100, 100, 130))),
        ]))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)));

    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let max_scroll = (lines.len() as u16).saturating_sub(inner.height);
    let scroll_pos = scroll.min(max_scroll);

    let p = Paragraph::new(lines)
        .style(Style::default().bg(Color::Rgb(15, 15, 25)))
        .scroll((scroll_pos, 0));
    frame.render_widget(p, inner);
}
