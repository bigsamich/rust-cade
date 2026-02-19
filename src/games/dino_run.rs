use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::games::Game;

const DINO_X: usize = 10;
const GRAVITY: f32 = 0.065;
const JUMP_VELOCITY: f32 = -1.05;
const DUCK_DURATION: u32 = 8; // ticks ducking lasts per keypress

#[derive(Clone)]
struct Obstacle {
    x: f32,
    width: usize,
    height: usize,
    is_bird: bool,
    bird_y_offset: f32, // offset from ground (negative = above ground)
}

pub struct DinoRun {
    dino_y: f32,
    dino_vy: f32,
    ducking: bool,
    duck_timer: u32,
    obstacles: Vec<Obstacle>,
    score: u32,
    high_score: u32,
    speed: f32,
    game_over: bool,
    started: bool,
    paused: bool,
    tick: u64,
    next_obstacle_tick: u64,
    ground_offset: usize,
    // Dynamic dimensions (updated each render)
    field_width: usize,
    ground_y: f32,
}

impl DinoRun {
    pub fn new() -> Self {
        Self {
            dino_y: 0.0, // will be set to ground_y
            dino_vy: 0.0,
            ducking: false,
            duck_timer: 0,
            obstacles: Vec::new(),
            score: 0,
            high_score: 0,
            speed: 0.5,
            game_over: false,
            started: false,
            paused: false,
            tick: 0,
            next_obstacle_tick: 60,
            ground_offset: 0,
            field_width: 70,
            ground_y: 15.0,
        }
    }

    fn spawn_obstacle(&mut self) {
        let mut rng = rand::thread_rng();
        let is_bird = self.score > 200 && rng.gen_bool(0.3);

        let obs = if is_bird {
            let bird_y_offset = if rng.gen_bool(0.5) {
                -2.0 // low bird - must duck
            } else {
                -5.0 // high bird - can run under or jump over
            };
            Obstacle {
                x: self.field_width as f32 + 5.0,
                width: 4,
                height: 1,
                is_bird: true,
                bird_y_offset,
            }
        } else {
            let variants: Vec<(usize, usize)> = vec![
                (2, 2), // small cactus
                (3, 3), // medium cactus
                (4, 2), // wide short cactus
                (2, 4), // tall thin cactus
                (5, 3), // wide medium cactus
            ];
            let (w, h) = variants[rng.gen_range(0..variants.len())];
            Obstacle {
                x: self.field_width as f32 + 5.0,
                width: w,
                height: h,
                is_bird: false,
                bird_y_offset: 0.0,
            }
        };
        self.obstacles.push(obs);

        // Schedule next obstacle
        let min_gap = (40.0 / self.speed) as u64;
        let max_gap = (80.0 / self.speed) as u64;
        self.next_obstacle_tick = self.tick + rng.gen_range(min_gap.max(20)..=max_gap.max(30));
    }

    fn check_collision(&self) -> bool {
        let ground_y = self.ground_y;
        let dino_top = self.dino_y as i32;
        let dino_height: i32 = if self.ducking { 1 } else { 3 };
        let dino_bottom = dino_top + dino_height;
        let dino_left = DINO_X as i32;
        let dino_right = DINO_X as i32 + 3;

        for obs in &self.obstacles {
            let obs_left = obs.x as i32;
            let obs_right = obs_left + obs.width as i32;
            let (obs_top, obs_bottom) = if obs.is_bird {
                let by = (ground_y + obs.bird_y_offset) as i32;
                (by, by + obs.height as i32)
            } else {
                let base = ground_y as i32 + 1;
                (base - obs.height as i32, base)
            };

            // AABB collision
            if dino_right > obs_left
                && dino_left < obs_right
                && dino_bottom > obs_top
                && dino_top < obs_bottom
            {
                return true;
            }
        }
        false
    }

    fn render_field(&self, width: usize, height: usize) -> Vec<Line<'static>> {
        let w = width;
        let h = height;
        let ground_row = self.ground_y as usize + 1;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default()); w]; h];

        // Draw sky gradient (subtle)
        for y in 0..h.min(ground_row) {
            let brightness = 15 + (y * 5).min(40);
            let sky_style = Style::default().fg(Color::Rgb(brightness as u8, brightness as u8, (brightness + 20).min(60) as u8));
            for x in 0..w {
                grid[y][x] = (' ', sky_style);
            }
        }

        // Draw ground line
        if ground_row < h {
            for x in 0..w {
                let ch = if (x + self.ground_offset) % 8 == 0 {
                    '▪'
                } else if (x + self.ground_offset) % 4 == 0 {
                    '·'
                } else {
                    '━'
                };
                grid[ground_row][x] = (
                    ch,
                    Style::default().fg(Color::Rgb(140, 120, 100)),
                );
            }
        }

        // Draw terrain details below ground
        for dy in 1..3 {
            let row = ground_row + dy;
            if row < h {
                for x in 0..w {
                    let hash = (x.wrapping_mul(7) + self.ground_offset.wrapping_mul(3) + dy * 13) % 11;
                    let (ch, col) = match hash {
                        0 => ('.', Color::Rgb(80, 70, 55)),
                        3 => ('·', Color::Rgb(60, 55, 45)),
                        7 => (',', Color::Rgb(70, 60, 50)),
                        _ => (' ', Color::Rgb(30, 25, 20)),
                    };
                    grid[row][x] = (ch, Style::default().fg(col).bg(Color::Rgb(30, 25, 20)));
                }
            }
        }

        // Draw clouds (decorative, scrolling)
        let cloud_art = ["  .-~~~-.  ", " /       \\ ", "(  ~cloud~ )", " \\_______/ "];
        let cloud_starts = [5usize, 30, 55, 80, 110];
        for &cx_base in &cloud_starts {
            let cx = ((cx_base + 500).wrapping_sub(self.ground_offset / 4)) % (w + 40);
            let cy = (cx_base % 4) + 1;
            for (row_i, row_str) in cloud_art.iter().enumerate() {
                let y = cy + row_i;
                if y < h && y < ground_row {
                    for (col_i, ch) in row_str.chars().enumerate() {
                        let x = cx.wrapping_add(col_i);
                        if x < w && ch != ' ' {
                            grid[y][x] = (ch, Style::default().fg(Color::Rgb(50, 50, 65)));
                        }
                    }
                }
            }
        }

        // Draw obstacles
        for obs in &self.obstacles {
            let ox = obs.x as i32;
            if ox < -(obs.width as i32) || ox >= w as i32 + 5 {
                continue; // Off screen
            }
            if obs.is_bird {
                let by = (self.ground_y + obs.bird_y_offset) as i32;
                if by >= 0 {
                    let by = by as usize;
                    // Draw bird with animation
                    let wing_up = self.tick % 8 < 4;
                    let bird_chars = if wing_up {
                        vec![' ', '/', '▬', '\\', ' ']
                    } else {
                        vec!['\\', '_', '▬', '_', '/']
                    };
                    for (dx, &ch) in bird_chars.iter().enumerate() {
                        let x = ox + dx as i32;
                        if x >= 0 && (x as usize) < w && by < h && ch != ' ' {
                            grid[by][x as usize] = (
                                ch,
                                Style::default()
                                    .fg(Color::Rgb(220, 80, 80))
                                    .add_modifier(Modifier::BOLD),
                            );
                        }
                    }
                }
            } else {
                // Draw cactus with improved graphics
                let base_y = self.ground_y as i32 + 1;
                for dy in 0..obs.height {
                    let y = base_y - 1 - dy as i32;
                    if y >= 0 && (y as usize) < h {
                        for dx in 0..obs.width {
                            let x = ox + dx as i32;
                            if x >= 0 && (x as usize) < w {
                                let (ch, color) = if dy == obs.height - 1 {
                                    // Top of cactus
                                    if obs.width == 1 {
                                        ('▲', Color::Rgb(40, 160, 40))
                                    } else if dx == 0 {
                                        ('╔', Color::Rgb(30, 140, 30))
                                    } else if dx == obs.width - 1 {
                                        ('╗', Color::Rgb(30, 140, 30))
                                    } else {
                                        ('▓', Color::Rgb(40, 160, 40))
                                    }
                                } else if dy == 0 {
                                    // Base of cactus
                                    if dx == 0 {
                                        ('╚', Color::Rgb(25, 120, 25))
                                    } else if dx == obs.width - 1 {
                                        ('╝', Color::Rgb(25, 120, 25))
                                    } else {
                                        ('█', Color::Rgb(30, 130, 30))
                                    }
                                } else {
                                    // Middle
                                    if dx == 0 || dx == obs.width - 1 {
                                        ('║', Color::Rgb(25, 120, 25))
                                    } else {
                                        ('█', Color::Rgb(35, 150, 35))
                                    }
                                };
                                grid[y as usize][x as usize] = (
                                    ch,
                                    Style::default().fg(color),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Draw dino
        let dy = self.dino_y as i32;
        if self.ducking {
            // Ducking dino (1 row, wider)
            if dy >= 0 && (dy as usize) < h {
                let dino_chars = [
                    ('▶', Color::Rgb(200, 200, 200)),
                    ('▓', Color::Rgb(180, 180, 180)),
                    ('▓', Color::Rgb(160, 160, 160)),
                    ('▬', Color::Rgb(140, 140, 140)),
                    ('─', Color::Rgb(120, 120, 120)),
                ];
                for (i, &(ch, color)) in dino_chars.iter().enumerate() {
                    let x = DINO_X + i;
                    if x < w {
                        grid[dy as usize][x] = (
                            ch,
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
        } else {
            // Standing dino (3 rows)
            // Row 0: head
            let head_y = dy;
            if head_y >= 0 && (head_y as usize) < h {
                let head = [
                    (' ', Color::Reset),
                    ('▄', Color::Rgb(180, 180, 180)),
                    ('█', Color::Rgb(200, 200, 200)),
                    ('▀', Color::Rgb(200, 50, 50)), // eye
                ];
                for (i, &(ch, color)) in head.iter().enumerate() {
                    let x = DINO_X + i;
                    if x < w && ch != ' ' {
                        grid[head_y as usize][x] = (
                            ch,
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
            // Row 1: body
            let body_y = dy + 1;
            if body_y >= 0 && (body_y as usize) < h {
                let body = [
                    ('▄', Color::Rgb(160, 160, 160)),
                    ('█', Color::Rgb(180, 180, 180)),
                    ('█', Color::Rgb(200, 200, 200)),
                    ('▌', Color::Rgb(160, 160, 160)),
                ];
                for (i, &(ch, color)) in body.iter().enumerate() {
                    let x = DINO_X + i;
                    if x < w {
                        grid[body_y as usize][x] = (
                            ch,
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
            // Row 2: legs (animated)
            let leg_y = dy + 2;
            if leg_y >= 0 && (leg_y as usize) < h {
                let legs = if self.dino_y as i32 + 2 >= self.ground_y as i32 {
                    // On ground - animate running
                    if self.tick % 10 < 5 {
                        [('▘', true), (' ', false), ('▝', true), (' ', false)]
                    } else {
                        [(' ', false), ('▝', true), (' ', false), ('▘', true)]
                    }
                } else {
                    // In air - legs tucked
                    [(' ', false), ('▔', true), ('▔', true), (' ', false)]
                };
                for (i, &(ch, visible)) in legs.iter().enumerate() {
                    let x = DINO_X + i;
                    if x < w && visible {
                        grid[leg_y as usize][x] = (
                            ch,
                            Style::default()
                                .fg(Color::Rgb(160, 160, 160))
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
        }

        // Convert to lines
        grid.into_iter()
            .map(|row| {
                let spans: Vec<Span<'static>> = row
                    .into_iter()
                    .map(|(ch, style)| Span::styled(String::from(ch), style))
                    .collect();
                Line::from(spans)
            })
            .collect()
    }
}

impl Game for DinoRun {
    fn update(&mut self) {
        if self.game_over || self.paused || !self.started {
            return;
        }

        self.tick += 1;
        self.ground_offset = (self.ground_offset + 1) % 10000;

        // Update duck timer
        if self.duck_timer > 0 {
            self.duck_timer -= 1;
            if self.duck_timer == 0 {
                self.ducking = false;
            }
        }

        // Update score
        if self.tick % 3 == 0 {
            self.score += 1;
        }

        // Gradually increase speed
        if self.tick % 200 == 0 {
            self.speed = (self.speed + 0.05).min(1.5);
        }

        // Apply gravity
        if self.dino_y < self.ground_y {
            self.dino_vy += GRAVITY;
        }
        self.dino_y += self.dino_vy;

        // Land on ground
        if self.dino_y >= self.ground_y {
            self.dino_y = self.ground_y;
            self.dino_vy = 0.0;
        }

        // Can't duck in air
        if self.dino_y < self.ground_y {
            self.ducking = false;
            self.duck_timer = 0;
        }

        // Move obstacles
        for obs in &mut self.obstacles {
            obs.x -= self.speed;
        }

        // Remove off-screen obstacles
        self.obstacles.retain(|obs| obs.x + obs.width as f32 > -10.0);

        // Spawn new obstacles
        if self.tick >= self.next_obstacle_tick {
            self.spawn_obstacle();
        }

        // Check collision
        if self.check_collision() {
            self.game_over = true;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
        }
    }

    fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.reset();
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if !self.game_over && self.started {
                    self.paused = !self.paused;
                }
            }
            _ => {
                if self.game_over {
                    match key.code {
                        KeyCode::Enter | KeyCode::Char(' ') => self.reset(),
                        _ => {}
                    }
                    return;
                }
                if !self.started {
                    match key.code {
                        KeyCode::Char(' ') | KeyCode::Up | KeyCode::Enter => {
                            self.started = true;
                            self.dino_y = self.ground_y;
                        }
                        _ => {}
                    }
                    return;
                }
                if self.paused {
                    return;
                }
                match key.code {
                    KeyCode::Char(' ') | KeyCode::Up => {
                        // Jump (only if on ground)
                        if self.dino_y >= self.ground_y {
                            self.dino_vy = JUMP_VELOCITY;
                            self.ducking = false;
                            self.duck_timer = 0;
                        }
                    }
                    KeyCode::Down => {
                        if self.dino_y >= self.ground_y {
                            // Duck - set timer
                            self.ducking = true;
                            self.duck_timer = DUCK_DURATION;
                        } else {
                            // Fast fall when in air
                            self.dino_vy += GRAVITY * 4.0;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(180, 100, 220)))
            .title(" 🦖 Dino Run ")
            .title_style(
                Style::default()
                    .fg(Color::Rgb(200, 120, 255))
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Update dynamic dimensions based on available space
        let field_height = inner.height.saturating_sub(2) as usize;
        let new_field_width = inner.width as usize;
        let new_ground_y = (field_height as f32 * 0.72).max(8.0);

        // Update dimensions (only if not mid-game or if starting)
        if !self.started || self.game_over {
            self.ground_y = new_ground_y;
            self.dino_y = new_ground_y;
        }
        self.field_width = new_field_width;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Status bar
                Constraint::Min(8),     // Game field
                Constraint::Length(1),  // Help
            ])
            .split(inner);

        // Status bar
        let status = Line::from(vec![
            Span::styled(" 🦖 ", Style::default()),
            Span::styled(
                format!("Score: {:05} ", self.score),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("🏆 High: {:05} ", self.high_score),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("⚡ Speed: {:.1}x ", self.speed / 0.5),
                Style::default().fg(Color::Green),
            ),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[0]);

        // Game field
        let fw = chunks[1].width as usize;
        let fh = chunks[1].height as usize;
        let lines = self.render_field(fw, fh);
        frame.render_widget(Paragraph::new(lines), chunks[1]);

        // Help / overlay
        if self.game_over {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(
                    " 💀 GAME OVER! ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("Score: {} │ ", self.score),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    "Press ENTER to restart, Esc for menu",
                    Style::default().fg(Color::Gray),
                ),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if !self.started {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ▶ Press SPACE to start! ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "SPACE/↑ Jump │ ↓ Duck │ P Pause │ R Restart │ Esc Menu",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.paused {
            let msg = Paragraph::new(Line::from(vec![Span::styled(
                " ⏸ PAUSED - Press P to resume ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            frame.render_widget(msg, chunks[2]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" SPACE/↑ Jump ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("↓ Duck ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("P Pause ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("R Restart ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("Esc Menu", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(help, chunks[2]);
        }
    }

    fn reset(&mut self) {
        let hs = self.high_score;
        let fw = self.field_width;
        let gy = self.ground_y;
        *self = DinoRun::new();
        self.high_score = hs;
        self.field_width = fw;
        self.ground_y = gy;
        self.dino_y = gy;
    }
}
