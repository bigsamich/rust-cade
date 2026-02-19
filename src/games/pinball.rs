use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::games::Game;

const GRAVITY: f32 = 0.04;
const BALL_DAMPING: f32 = 0.98;
const FLIPPER_FORCE: f32 = -1.8;
const BUMPER_FORCE: f32 = 1.2;

#[derive(Clone)]
struct Bumper {
    x: f32,
    y: f32,
    radius: f32,
    points: u32,
    hit_timer: u32, // visual flash when hit
}

#[derive(Clone)]
struct Rail {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

pub struct Pinball {
    ball_x: f32,
    ball_y: f32,
    ball_dx: f32,
    ball_dy: f32,
    bumpers: Vec<Bumper>,
    rails: Vec<Rail>,
    left_flipper: bool,
    right_flipper: bool,
    flipper_timer_l: u32,
    flipper_timer_r: u32,
    score: u32,
    high_score: u32,
    balls_left: u32,
    game_over: bool,
    paused: bool,
    launched: bool,
    launch_power: f32,
    charging: bool,
    tick: u64,
    combo: u32,
    combo_timer: u32,
    // Dynamic dimensions
    field_width: f32,
    field_height: f32,
}

impl Pinball {
    pub fn new() -> Self {
        let fw = 40.0;
        let fh = 35.0;
        let mut p = Self {
            ball_x: fw - 3.0,
            ball_y: fh - 5.0,
            ball_dx: 0.0,
            ball_dy: 0.0,
            bumpers: Vec::new(),
            rails: Vec::new(),
            left_flipper: false,
            right_flipper: false,
            flipper_timer_l: 0,
            flipper_timer_r: 0,
            score: 0,
            high_score: 0,
            balls_left: 3,
            game_over: false,
            paused: false,
            launched: false,
            launch_power: 0.0,
            charging: false,
            tick: 0,
            combo: 0,
            combo_timer: 0,
            field_width: fw,
            field_height: fh,
        };
        p.init_table();
        p
    }

    fn init_table(&mut self) {
        let fw = self.field_width;
        let fh = self.field_height;

        self.bumpers.clear();
        self.rails.clear();

        // Top bumpers (triangle formation)
        let cx = fw / 2.0;
        self.bumpers.push(Bumper { x: cx, y: fh * 0.20, radius: 2.0, points: 100, hit_timer: 0 });
        self.bumpers.push(Bumper { x: cx - 6.0, y: fh * 0.30, radius: 2.0, points: 100, hit_timer: 0 });
        self.bumpers.push(Bumper { x: cx + 6.0, y: fh * 0.30, radius: 2.0, points: 100, hit_timer: 0 });

        // Middle bumpers
        self.bumpers.push(Bumper { x: cx - 4.0, y: fh * 0.45, radius: 1.5, points: 50, hit_timer: 0 });
        self.bumpers.push(Bumper { x: cx + 4.0, y: fh * 0.45, radius: 1.5, points: 50, hit_timer: 0 });
        self.bumpers.push(Bumper { x: cx, y: fh * 0.55, radius: 2.0, points: 75, hit_timer: 0 });

        // Side targets
        self.bumpers.push(Bumper { x: 4.0, y: fh * 0.35, radius: 1.0, points: 200, hit_timer: 0 });
        self.bumpers.push(Bumper { x: fw - 5.0, y: fh * 0.35, radius: 1.0, points: 200, hit_timer: 0 });

        // Rails (angled walls)
        // Top curve - guides ball from right plunger lane into play area
        self.rails.push(Rail { x1: fw - 5.0, y1: fh * 0.05, x2: fw * 0.5, y2: fh * 0.02 });
        self.rails.push(Rail { x1: fw * 0.5, y1: fh * 0.02, x2: fw * 0.3, y2: fh * 0.08 });
        // Left rail
        self.rails.push(Rail { x1: 2.0, y1: fh * 0.15, x2: 6.0, y2: fh * 0.05 });
        // Right rail  
        self.rails.push(Rail { x1: fw - 3.0, y1: fh * 0.15, x2: fw - 7.0, y2: fh * 0.05 });
        // Left guide to flippers
        self.rails.push(Rail { x1: 3.0, y1: fh * 0.70, x2: 8.0, y2: fh * 0.78 });
        // Right guide to flippers
        self.rails.push(Rail { x1: fw - 4.0, y1: fh * 0.70, x2: fw - 9.0, y2: fh * 0.78 });
    }

    fn flipper_left_y(&self) -> f32 {
        self.field_height * 0.82
    }

    fn flipper_right_y(&self) -> f32 {
        self.field_height * 0.82
    }

    fn flipper_left_x(&self) -> f32 {
        self.field_width * 0.20
    }

    fn flipper_right_x(&self) -> f32 {
        self.field_width * 0.55
    }

    fn update_physics(&mut self) {
        if !self.launched { return; }

        // Gravity
        self.ball_dy += GRAVITY;

        // Apply velocity
        self.ball_x += self.ball_dx;
        self.ball_y += self.ball_dy;

        // Damping
        self.ball_dx *= BALL_DAMPING;

        let fw = self.field_width;
        let fh = self.field_height;

        // Wall collisions
        if self.ball_x <= 2.0 {
            self.ball_x = 2.0;
            self.ball_dx = self.ball_dx.abs() * 0.8;
        }
        if self.ball_x >= fw - 3.0 {
            self.ball_x = fw - 3.0;
            self.ball_dx = -self.ball_dx.abs() * 0.8;
        }
        if self.ball_y <= 1.0 {
            self.ball_y = 1.0;
            self.ball_dy = self.ball_dy.abs() * 0.6;
        }

        // Flipper collisions
        let fl_x = self.flipper_left_x();
        let fl_y = self.flipper_left_y();
        let fr_x = self.flipper_right_x();
        let fr_y = self.flipper_right_y();
        let flipper_len = fw * 0.22;

        // Left flipper - wider collision zone for angled flipper
        if self.ball_x >= fl_x && self.ball_x <= fl_x + flipper_len {
            let hit_pos = (self.ball_x - fl_x) / flipper_len;
            let flipper_y_at_pos = if self.left_flipper {
                fl_y - hit_pos * 2.5 // Angled up
            } else {
                fl_y + hit_pos * 1.5 // Angled down
            };
            if (self.ball_y - flipper_y_at_pos).abs() < 1.5 {
                if self.left_flipper {
                    self.ball_dy = FLIPPER_FORCE * (1.0 - hit_pos * 0.2);
                    self.ball_dx = -(1.0 - hit_pos) * 0.6 + hit_pos * 0.8;
                } else {
                    self.ball_dy = -self.ball_dy.abs() * 0.3;
                }
            }
        }

        // Right flipper - wider collision zone for angled flipper
        if self.ball_x >= fr_x && self.ball_x <= fr_x + flipper_len {
            let hit_pos = (self.ball_x - fr_x) / flipper_len;
            let flipper_y_at_pos = if self.right_flipper {
                fr_y - (1.0 - hit_pos) * 2.5 // Angled up (mirrored)
            } else {
                fr_y + (1.0 - hit_pos) * 1.5 // Angled down (mirrored)
            };
            if (self.ball_y - flipper_y_at_pos).abs() < 1.5 {
                if self.right_flipper {
                    self.ball_dy = FLIPPER_FORCE * (1.0 - (1.0 - hit_pos) * 0.2);
                    self.ball_dx = hit_pos * 0.6 - (1.0 - hit_pos) * 0.8;
                } else {
                    self.ball_dy = -self.ball_dy.abs() * 0.3;
                }
            }
        }

        // Bumper collisions
        for bumper in &mut self.bumpers {
            let dx = self.ball_x - bumper.x;
            let dy = self.ball_y - bumper.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < bumper.radius + 0.8 {
                // Bounce away from bumper
                let nx = dx / dist.max(0.01);
                let ny = dy / dist.max(0.01);
                self.ball_dx = nx * BUMPER_FORCE;
                self.ball_dy = ny * BUMPER_FORCE;
                // Push ball out
                self.ball_x = bumper.x + nx * (bumper.radius + 1.0);
                self.ball_y = bumper.y + ny * (bumper.radius + 1.0);
                bumper.hit_timer = 6;
                // Scoring with combo
                self.combo += 1;
                self.combo_timer = 30;
                let combo_mult = self.combo.min(5);
                self.score += bumper.points * combo_mult;
            }
        }

        // Rail collisions (simplified - treat as horizontal bounce zones)
        for rail in &self.rails {
            let rx = (rail.x1 + rail.x2) / 2.0;
            let ry = (rail.y1 + rail.y2) / 2.0;
            let dx = self.ball_x - rx;
            let dy = self.ball_y - ry;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 2.5 {
                let nx = dx / dist.max(0.01);
                let ny = dy / dist.max(0.01);
                self.ball_dx += nx * 0.5;
                self.ball_dy += ny * 0.5;
                self.score += 10;
            }
        }

        // Ball lost (fell below flippers)
        if self.ball_y > fh + 2.0 {
            self.balls_left = self.balls_left.saturating_sub(1);
            if self.balls_left == 0 {
                self.game_over = true;
                if self.score > self.high_score {
                    self.high_score = self.score;
                }
            } else {
                // Reset ball to plunger
                self.ball_x = fw - 3.0;
                self.ball_y = fh - 5.0;
                self.ball_dx = 0.0;
                self.ball_dy = 0.0;
                self.launched = false;
                self.combo = 0;
            }
        }
    }

    fn render_field(&self, width: usize, height: usize) -> Vec<Line<'static>> {
        let w = width;
        let h = height;
        let sx = w as f32 / self.field_width;
        let sy = h as f32 / self.field_height;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(8, 15, 8))); w]; h];

        // Draw table border
        for y in 0..h {
            if y == 0 {
                for x in 0..w {
                    grid[y][x] = ('═', Style::default().fg(Color::Rgb(100, 80, 40)).bg(Color::Rgb(8, 15, 8)));
                }
                if w > 0 { grid[0][0] = ('╔', Style::default().fg(Color::Rgb(100, 80, 40)).bg(Color::Rgb(8, 15, 8))); }
                if w > 1 { grid[0][w-1] = ('╗', Style::default().fg(Color::Rgb(100, 80, 40)).bg(Color::Rgb(8, 15, 8))); }
            } else {
                grid[y][0] = ('║', Style::default().fg(Color::Rgb(100, 80, 40)).bg(Color::Rgb(8, 15, 8)));
                if w > 1 {
                    grid[y][w-1] = ('║', Style::default().fg(Color::Rgb(100, 80, 40)).bg(Color::Rgb(8, 15, 8)));
                }
            }
        }

        // Draw plunger lane
        let plunger_x = ((self.field_width - 2.0) * sx) as usize;
        for y in (h * 3 / 4)..h {
            if plunger_x < w {
                grid[y][plunger_x] = ('│', Style::default().fg(Color::Rgb(80, 80, 60)).bg(Color::Rgb(8, 15, 8)));
            }
        }
        // Plunger spring
        if !self.launched {
            let spring_y = h.saturating_sub(3);
            let power_bars = (self.launch_power * 4.0) as usize;
            for i in 0..power_bars.min(4) {
                let sy_pos = spring_y.saturating_sub(i);
                if plunger_x + 1 < w && sy_pos < h {
                    let color = match i {
                        0 => Color::Rgb(80, 200, 80),
                        1 => Color::Rgb(200, 200, 50),
                        2 => Color::Rgb(220, 140, 30),
                        _ => Color::Rgb(220, 50, 50),
                    };
                    grid[sy_pos][plunger_x + 1] = ('▮', Style::default().fg(color).bg(Color::Rgb(8, 15, 8)));
                }
            }
        }

        // Draw rails
        for rail in &self.rails {
            let x1 = (rail.x1 * sx) as i32;
            let y1 = (rail.y1 * sy) as i32;
            let x2 = (rail.x2 * sx) as i32;
            let y2 = (rail.y2 * sy) as i32;
            let steps = ((x2 - x1).abs().max((y2 - y1).abs()) + 1) as usize;
            for s in 0..=steps {
                let t = s as f32 / steps.max(1) as f32;
                let rx = (x1 as f32 + (x2 - x1) as f32 * t) as usize;
                let ry = (y1 as f32 + (y2 - y1) as f32 * t) as usize;
                if rx < w && ry < h {
                    grid[ry][rx] = ('◆', Style::default().fg(Color::Rgb(60, 100, 140)).bg(Color::Rgb(8, 15, 8)));
                }
            }
        }

        // Draw bumpers
        for bumper in &self.bumpers {
            let bx = (bumper.x * sx) as i32;
            let by = (bumper.y * sy) as i32;
            let r = (bumper.radius * sx.min(sy)) as i32;

            let (fg_color, ch) = if bumper.hit_timer > 0 {
                (Color::Rgb(255, 255, 100), '◉')
            } else if bumper.points >= 200 {
                (Color::Rgb(220, 50, 220), '◎')
            } else if bumper.points >= 100 {
                (Color::Rgb(220, 80, 80), '◎')
            } else {
                (Color::Rgb(80, 180, 220), '◎')
            };

            // Draw bumper body
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        let px = (bx + dx) as usize;
                        let py = (by + dy) as usize;
                        if px < w && py < h && px > 0 {
                            let inner_ch = if dx == 0 && dy == 0 {
                                ch
                            } else if dx.abs() + dy.abs() <= 1 {
                                '●'
                            } else {
                                '○'
                            };
                            grid[py][px] = (inner_ch, Style::default().fg(fg_color).bg(Color::Rgb(8, 15, 8)));
                        }
                    }
                }
            }
        }

        // Draw flippers
        let fl_x = (self.flipper_left_x() * sx) as usize;
        let fl_y = (self.flipper_left_y() * sy) as usize;
        let fr_x = (self.flipper_right_x() * sx) as usize;
        let fr_y = (self.flipper_right_y() * sy) as usize;
        let flipper_len = (self.field_width * 0.22 * sx) as usize;

        let fl_color = if self.left_flipper {
            Color::Rgb(255, 220, 80)
        } else {
            Color::Rgb(180, 150, 60)
        };
        let fr_color = if self.right_flipper {
            Color::Rgb(255, 220, 80)
        } else {
            Color::Rgb(180, 150, 60)
        };

        // Left flipper - swings up when active
        if self.left_flipper {
            // Angled up: pivot at left, tip rises
            let pivot_x = fl_x;
            let pivot_y = fl_y;
            for dx in 0..flipper_len {
                let rise = (dx as f32 / flipper_len as f32 * 2.5) as usize;
                let x = pivot_x + dx;
                let y = pivot_y.saturating_sub(rise);
                if x < w && y < h {
                    let ch = if dx == 0 { '●' } else if dx == flipper_len - 1 { '╱' } else { '─' };
                    grid[y][x] = (ch, Style::default().fg(fl_color).bg(Color::Rgb(8, 15, 8)).add_modifier(Modifier::BOLD));
                }
            }
        } else {
            // Resting: angled slightly down from pivot
            let pivot_x = fl_x;
            let pivot_y = fl_y;
            for dx in 0..flipper_len {
                let drop = (dx as f32 / flipper_len as f32 * 1.5) as usize;
                let x = pivot_x + dx;
                let y = pivot_y + drop;
                if x < w && y < h {
                    let ch = if dx == 0 { '●' } else if dx == flipper_len - 1 { '╲' } else { '─' };
                    grid[y][x] = (ch, Style::default().fg(fl_color).bg(Color::Rgb(8, 15, 8)).add_modifier(Modifier::BOLD));
                }
            }
        }
        // Right flipper - swings up when active (mirror)
        if self.right_flipper {
            let _pivot_x = fr_x + flipper_len.saturating_sub(1);
            let pivot_y = fr_y;
            for dx in 0..flipper_len {
                let rise = ((flipper_len - 1 - dx) as f32 / flipper_len as f32 * 2.5) as usize;
                let x = fr_x + dx;
                let y = pivot_y.saturating_sub(rise);
                if x < w && y < h {
                    let ch = if dx == flipper_len - 1 { '●' } else if dx == 0 { '╲' } else { '─' };
                    grid[y][x] = (ch, Style::default().fg(fr_color).bg(Color::Rgb(8, 15, 8)).add_modifier(Modifier::BOLD));
                }
            }
        } else {
            let pivot_y = fr_y;
            for dx in 0..flipper_len {
                let drop = ((flipper_len - 1 - dx) as f32 / flipper_len as f32 * 1.5) as usize;
                let x = fr_x + dx;
                let y = pivot_y + drop;
                if x < w && y < h {
                    let ch = if dx == flipper_len - 1 { '●' } else if dx == 0 { '╱' } else { '─' };
                    grid[y][x] = (ch, Style::default().fg(fr_color).bg(Color::Rgb(8, 15, 8)).add_modifier(Modifier::BOLD));
                }
            }
        }

        // Draw drain gap between flippers
        let drain_start = fl_x + flipper_len;
        let drain_end = fr_x;
        if fl_y < h {
            for x in drain_start..drain_end {
                if x < w {
                    grid[fl_y][x] = ('▿', Style::default().fg(Color::Rgb(100, 30, 30)).bg(Color::Rgb(8, 15, 8)));
                }
            }
        }

        // Draw ball
        let bx = (self.ball_x * sx) as usize;
        let by = (self.ball_y * sy) as usize;
        if bx < w && by < h {
            grid[by][bx] = ('●', Style::default()
                .fg(Color::Rgb(230, 230, 240))
                .bg(Color::Rgb(8, 15, 8))
                .add_modifier(Modifier::BOLD));
            // Ball glow
            for &(dx, dy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                let gx = bx as i32 + dx;
                let gy = by as i32 + dy;
                if gx >= 0 && (gx as usize) < w && gy >= 0 && (gy as usize) < h {
                    let gx = gx as usize;
                    let gy = gy as usize;
                    if grid[gy][gx].0 == ' ' {
                        grid[gy][gx] = ('·', Style::default()
                            .fg(Color::Rgb(60, 60, 80))
                            .bg(Color::Rgb(8, 15, 8)));
                    }
                }
            }
        }

        // Combo display
        if self.combo_timer > 0 && self.combo > 1 {
            let combo_text = format!("{}x COMBO!", self.combo);
            let cx = w / 2 - combo_text.len() / 2;
            let cy = h / 2;
            if cy < h {
                for (i, ch) in combo_text.chars().enumerate() {
                    let x = cx + i;
                    if x < w {
                        grid[cy][x] = (ch, Style::default()
                            .fg(Color::Rgb(255, 200, 50))
                            .bg(Color::Rgb(8, 15, 8))
                            .add_modifier(Modifier::BOLD));
                    }
                }
            }
        }

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

impl Game for Pinball {
    fn update(&mut self) {
        if self.game_over || self.paused { return; }
        self.tick += 1;

        // Update bumper hit timers
        for bumper in &mut self.bumpers {
            if bumper.hit_timer > 0 {
                bumper.hit_timer -= 1;
            }
        }

        // Update flipper timers
        if self.flipper_timer_l > 0 {
            self.flipper_timer_l -= 1;
            if self.flipper_timer_l == 0 { self.left_flipper = false; }
        }
        if self.flipper_timer_r > 0 {
            self.flipper_timer_r -= 1;
            if self.flipper_timer_r == 0 { self.right_flipper = false; }
        }

        // Update combo timer
        if self.combo_timer > 0 {
            self.combo_timer -= 1;
            if self.combo_timer == 0 { self.combo = 0; }
        }

        // Charge plunger
        if self.charging && !self.launched {
            self.launch_power = (self.launch_power + 0.03).min(1.0);
        }

        self.update_physics();
    }

    fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => self.reset(),
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if !self.game_over {
                    self.paused = !self.paused;
                }
            }
            _ => {
                if self.game_over {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        self.reset();
                    }
                    return;
                }
                if self.paused { return; }
                match key.code {
                    KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                        self.left_flipper = true;
                        self.flipper_timer_l = 6;
                    }
                    KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                        self.right_flipper = true;
                        self.flipper_timer_r = 6;
                    }
                    KeyCode::Char(' ') | KeyCode::Down => {
                        if !self.launched {
                            // Launch ball up the right lane
                            self.launched = true;
                            let power = self.launch_power.max(0.4);
                            self.ball_dy = -power * 3.5;
                            self.ball_dx = -1.2; // Strong leftward push into play area
                            self.launch_power = 0.0;
                            self.charging = false;
                        } else {
                            // Both flippers
                            self.left_flipper = true;
                            self.right_flipper = true;
                            self.flipper_timer_l = 6;
                            self.flipper_timer_r = 6;
                        }
                    }
                    KeyCode::Up => {
                        if !self.launched {
                            self.charging = true;
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
            .border_style(Style::default().fg(Color::Rgb(200, 160, 50)))
            .title(" 🎱 Pinball ")
            .title_style(Style::default().fg(Color::Rgb(255, 200, 80)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Update dimensions dynamically
        let new_fw = (inner.width as f32 * 0.6).max(20.0);
        let new_fh = inner.height.saturating_sub(2) as f32;
        if !self.launched && !self.game_over {
            if (new_fw - self.field_width).abs() > 2.0 || (new_fh - self.field_height).abs() > 2.0 {
                self.field_width = new_fw;
                self.field_height = new_fh;
                self.ball_x = new_fw - 3.0;
                self.ball_y = new_fh - 5.0;
                self.init_table();
            }
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(10),
                Constraint::Length(1),
            ])
            .split(inner);

        // Status bar
        let status = Line::from(vec![
            Span::styled(" 🎱 ", Style::default()),
            Span::styled(
                format!("Score: {} ", self.score),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Balls: {} ", "● ".repeat(self.balls_left as usize)),
                Style::default().fg(Color::Rgb(200, 200, 220)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("🏆 High: {} ", self.high_score),
                Style::default().fg(Color::Cyan),
            ),
            if self.combo > 1 {
                Span::styled(
                    format!(" │ 🔥 {}x Combo! ", self.combo),
                    Style::default().fg(Color::Rgb(255, 150, 50)).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("", Style::default())
            },
        ]);
        frame.render_widget(Paragraph::new(status), chunks[0]);

        // Game field - center the playfield
        let fw = chunks[1].width as usize;
        let fh = chunks[1].height as usize;
        let lines = self.render_field(fw, fh);
        frame.render_widget(Paragraph::new(lines), chunks[1]);

        // Help bar
        if self.game_over {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" 💀 GAME OVER! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(format!("Score: {} │ ", self.score), Style::default().fg(Color::Yellow)),
                Span::styled("Press ENTER to restart, Esc for menu", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.paused {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" ⏸ PAUSED - Press P to resume ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if !self.launched {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" Hold ↑ to charge, ", Style::default().fg(Color::DarkGray)),
                Span::styled("SPACE to launch ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("│ ←→ Flippers │ P Pause │ R Restart │ Esc Menu", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" ← Left Flipper ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("→ Right Flipper ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("SPACE Both ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("P Pause ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("Esc Menu", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(help, chunks[2]);
        }
    }

    fn reset(&mut self) {
        let hs = self.high_score;
        let fw = self.field_width;
        let fh = self.field_height;
        *self = Pinball::new();
        self.high_score = hs;
        self.field_width = fw;
        self.field_height = fh;
        self.ball_x = fw - 3.0;
        self.ball_y = fh - 5.0;
        self.init_table();
    }
}
