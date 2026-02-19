use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::games::Game;

const BRICK_ROWS: usize = 6;
const BRICKS_PER_ROW: usize = 12;

#[derive(Clone)]
struct Brick {
    x: f32,
    y: f32,
    width: f32,
    alive: bool,
    color: Color,
    points: u32,
}

pub struct Breakout {
    paddle_x: f32,
    paddle_width: f32,
    ball_x: f32,
    ball_y: f32,
    ball_dx: f32,
    ball_dy: f32,
    ball_speed: f32,
    bricks: Vec<Brick>,
    score: u32,
    high_score: u32,
    lives: u32,
    game_over: bool,
    won: bool,
    paused: bool,
    launched: bool,
    tick: u64,
    // Dynamic dimensions
    field_width: f32,
    field_height: f32,
    paddle_y: f32,
}

impl Breakout {
    pub fn new() -> Self {
        let fw = 70.0;
        let fh = 28.0;
        let pw = 12.0;
        let py = fh - 3.0;
        let mut b = Self {
            paddle_x: fw / 2.0 - pw / 2.0,
            paddle_width: pw,
            ball_x: fw / 2.0,
            ball_y: py - 1.0,
            ball_dx: 0.35,
            ball_dy: -0.35,
            ball_speed: 0.35,
            bricks: Vec::new(),
            score: 0,
            high_score: 0,
            lives: 3,
            game_over: false,
            won: false,
            paused: false,
            launched: false,
            tick: 0,
            field_width: fw,
            field_height: fh,
            paddle_y: py,
        };
        b.init_bricks();
        b
    }

    fn init_bricks(&mut self) {
        self.bricks.clear();
        let colors = [
            Color::Rgb(220, 50, 50),   // Red
            Color::Rgb(220, 130, 30),  // Orange
            Color::Rgb(220, 200, 30),  // Yellow
            Color::Rgb(50, 200, 50),   // Green
            Color::Rgb(50, 130, 220),  // Blue
            Color::Rgb(150, 50, 220),  // Purple
        ];
        let points = [60, 50, 40, 30, 20, 10];
        let brick_width = self.field_width / BRICKS_PER_ROW as f32;
        
        for row in 0..BRICK_ROWS {
            for col in 0..BRICKS_PER_ROW {
                self.bricks.push(Brick {
                    x: col as f32 * brick_width,
                    y: 2.0 + row as f32 * 1.5,
                    width: brick_width,
                    alive: true,
                    color: colors[row % colors.len()],
                    points: points[row % points.len()],
                });
            }
        }
    }

    fn reset_ball(&mut self) {
        self.ball_x = self.paddle_x + self.paddle_width / 2.0;
        self.ball_y = self.paddle_y - 1.0;
        self.ball_dx = self.ball_speed;
        self.ball_dy = -self.ball_speed;
        self.launched = false;
    }

    fn move_ball(&mut self) {
        if !self.launched {
            self.ball_x = self.paddle_x + self.paddle_width / 2.0;
            self.ball_y = self.paddle_y - 1.0;
            return;
        }

        self.ball_x += self.ball_dx;
        self.ball_y += self.ball_dy;

        // Wall collisions
        if self.ball_x <= 0.5 {
            self.ball_x = 0.5;
            self.ball_dx = self.ball_dx.abs();
        }
        if self.ball_x >= self.field_width - 1.5 {
            self.ball_x = self.field_width - 1.5;
            self.ball_dx = -self.ball_dx.abs();
        }
        if self.ball_y <= 0.5 {
            self.ball_y = 0.5;
            self.ball_dy = self.ball_dy.abs();
        }

        // Ball falls below paddle
        if self.ball_y >= self.field_height {
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.game_over = true;
                if self.score > self.high_score {
                    self.high_score = self.score;
                }
            } else {
                self.reset_ball();
            }
            return;
        }

        // Paddle collision
        if self.ball_dy > 0.0
            && self.ball_y >= self.paddle_y - 0.5
            && self.ball_y <= self.paddle_y + 1.0
            && self.ball_x >= self.paddle_x - 0.5
            && self.ball_x <= self.paddle_x + self.paddle_width + 0.5
        {
            self.ball_dy = -self.ball_dy.abs();
            let hit_pos = (self.ball_x - self.paddle_x) / self.paddle_width;
            self.ball_dx = self.ball_speed * (hit_pos - 0.5) * 3.0;
            if self.ball_dy.abs() < 0.15 {
                self.ball_dy = -0.15;
            }
            // Prevent ball from going too horizontal
            if self.ball_dx.abs() > self.ball_speed * 1.5 {
                self.ball_dx = self.ball_dx.signum() * self.ball_speed * 1.5;
            }
        }

        // Brick collisions
        let mut hit_idx = None;
        for (i, brick) in self.bricks.iter().enumerate() {
            if !brick.alive { continue; }
            if self.ball_x >= brick.x - 0.5
                && self.ball_x < brick.x + brick.width + 0.5
                && self.ball_y >= brick.y - 0.5
                && self.ball_y < brick.y + 1.5
            {
                hit_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = hit_idx {
            let brick = &self.bricks[idx];
            let cx = brick.x + brick.width / 2.0;
            let cy = brick.y + 0.75;
            let dx = self.ball_x - cx;
            let dy = self.ball_y - cy;
            if dx.abs() / brick.width > dy.abs() / 1.5 {
                self.ball_dx = -self.ball_dx;
            } else {
                self.ball_dy = -self.ball_dy;
            }
            self.score += self.bricks[idx].points;
            self.bricks[idx].alive = false;

            if self.bricks.iter().all(|b| !b.alive) {
                self.won = true;
                if self.score > self.high_score {
                    self.high_score = self.score;
                }
            }
            self.ball_speed = (self.ball_speed + 0.003).min(0.7);
        }
    }

    fn render_field(&self, width: usize, height: usize) -> Vec<Line<'static>> {
        let w = width;
        let h = height;

        // Scale factors
        let sx = w as f32 / self.field_width;
        let sy = h as f32 / self.field_height;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(10, 10, 20))); w]; h];

        // Draw walls
        for y in 0..h {
            grid[y][0] = ('│', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(10, 10, 20)));
            if w > 1 {
                grid[y][w - 1] = ('│', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(10, 10, 20)));
            }
        }
        for x in 0..w {
            grid[0][x] = ('─', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(10, 10, 20)));
        }
        if w > 0 && h > 0 {
            grid[0][0] = ('╭', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(10, 10, 20)));
            grid[0][w - 1] = ('╮', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(10, 10, 20)));
        }

        // Draw bricks
        for brick in &self.bricks {
            if !brick.alive { continue; }
            let bx_start = (brick.x * sx) as usize;
            let bx_end = ((brick.x + brick.width) * sx) as usize;
            let by = (brick.y * sy) as usize;
            
            if by < h {
                for bx in bx_start..bx_end.min(w) {
                    if bx < w {
                        let ch = if bx == bx_start {
                            '▐'
                        } else if bx + 1 >= bx_end.min(w) {
                            '▌'
                        } else {
                            '█'
                        };
                        grid[by][bx] = (ch, Style::default().fg(brick.color).bg(Color::Rgb(10, 10, 20)));
                    }
                }
                // Shadow row below
                let shadow_y = by + 1;
                if shadow_y < h {
                    for bx in bx_start..bx_end.min(w) {
                        if bx < w && grid[shadow_y][bx].0 == ' ' {
                            grid[shadow_y][bx] = ('░', Style::default()
                                .fg(Color::Rgb(30, 30, 40))
                                .bg(Color::Rgb(10, 10, 20)));
                        }
                    }
                }
            }
        }

        // Draw paddle
        let px_start = (self.paddle_x * sx) as usize;
        let px_end = ((self.paddle_x + self.paddle_width) * sx) as usize;
        let py = (self.paddle_y * sy) as usize;
        if py < h {
            for px in px_start..px_end.min(w) {
                if px < w {
                    let ch = if px == px_start {
                        '╣'
                    } else if px + 1 >= px_end.min(w) {
                        '╠'
                    } else if px == px_start + 1 || px + 2 >= px_end.min(w) {
                        '▓'
                    } else {
                        '═'
                    };
                    grid[py][px] = (ch, Style::default()
                        .fg(Color::Rgb(180, 200, 255))
                        .bg(Color::Rgb(30, 50, 120))
                        .add_modifier(Modifier::BOLD));
                }
            }
        }

        // Draw ball
        let bx = (self.ball_x * sx) as usize;
        let by = (self.ball_y * sy) as usize;
        if bx < w && by < h {
            grid[by][bx] = ('●', Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(Color::Rgb(10, 10, 20))
                .add_modifier(Modifier::BOLD));
            // Ball trail
            let trail_x = (self.ball_x - self.ball_dx * 2.0) * sx;
            let trail_y = (self.ball_y - self.ball_dy * 2.0) * sy;
            let tx = trail_x as usize;
            let ty = trail_y as usize;
            if tx < w && ty < h && (tx != bx || ty != by) {
                grid[ty][tx] = ('·', Style::default()
                    .fg(Color::Rgb(100, 100, 120))
                    .bg(Color::Rgb(10, 10, 20)));
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

impl Game for Breakout {
    fn update(&mut self) {
        if self.game_over || self.won || self.paused { return; }
        self.tick += 1;
        self.move_ball();
    }

    fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => self.reset(),
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if !self.game_over && !self.won {
                    self.paused = !self.paused;
                }
            }
            _ => {
                if self.game_over || self.won {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        self.reset();
                    }
                    return;
                }
                if self.paused { return; }
                match key.code {
                    KeyCode::Left => {
                        self.paddle_x = (self.paddle_x - 2.0).max(0.5);
                        if !self.launched {
                            self.ball_x = self.paddle_x + self.paddle_width / 2.0;
                        }
                    }
                    KeyCode::Right => {
                        self.paddle_x = (self.paddle_x + 2.0).min(self.field_width - self.paddle_width - 0.5);
                        if !self.launched {
                            self.ball_x = self.paddle_x + self.paddle_width / 2.0;
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Up => {
                        if !self.launched {
                            self.launched = true;
                            self.ball_dy = -self.ball_speed;
                            self.ball_dx = self.ball_speed * 0.7;
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
            .border_style(Style::default().fg(Color::Rgb(220, 80, 80)))
            .title(" 🧱 Breakout ")
            .title_style(Style::default().fg(Color::Rgb(255, 100, 100)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Update dimensions dynamically
        let new_fw = inner.width as f32;
        let new_fh = (inner.height.saturating_sub(2)) as f32;
        if !self.launched && !self.game_over && !self.won {
            if (new_fw - self.field_width).abs() > 1.0 || (new_fh - self.field_height).abs() > 1.0 {
                let ratio_x = new_fw / self.field_width;
                let ratio_y = new_fh / self.field_height;
                self.paddle_x *= ratio_x;
                self.ball_x *= ratio_x;
                self.ball_y *= ratio_y;
                self.paddle_y = new_fh - 3.0;
                self.field_width = new_fw;
                self.field_height = new_fh;
                self.paddle_width = (new_fw / 6.0).max(6.0);
                // Reinit bricks for new dimensions
                self.init_bricks();
            }
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(8),
                Constraint::Length(1),
            ])
            .split(inner);

        // Status bar
        let bricks_left = self.bricks.iter().filter(|b| b.alive).count();
        let total_bricks = BRICK_ROWS * BRICKS_PER_ROW;
        let status = Line::from(vec![
            Span::styled(" 🧱 ", Style::default()),
            Span::styled(
                format!("Score: {} ", self.score),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Lives: {} ", "♥ ".repeat(self.lives as usize)),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("🏆 High: {} ", self.high_score),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Bricks: {}/{} ", bricks_left, total_bricks),
                Style::default().fg(Color::Green),
            ),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[0]);

        // Game field
        let fw = chunks[1].width as usize;
        let fh = chunks[1].height as usize;
        let lines = self.render_field(fw, fh);
        frame.render_widget(Paragraph::new(lines), chunks[1]);

        // Help bar
        if self.game_over {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" 💀 GAME OVER! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("Press ENTER to restart, Esc for menu", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.won {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" 🎉 YOU WIN! ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(format!("Score: {} │ Press ENTER to play again", self.score), Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.paused {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" ⏸ PAUSED - Press P to resume ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if !self.launched {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" ←→ Move Paddle ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("SPACE Launch ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("P Pause ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("R Restart ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("Esc Menu", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(help, chunks[2]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" ←→ Move Paddle ", Style::default().fg(Color::DarkGray)),
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
        let fh = self.field_height;
        *self = Breakout::new();
        self.high_score = hs;
        self.field_width = fw;
        self.field_height = fh;
        self.paddle_y = fh - 3.0;
        self.paddle_width = (fw / 6.0).max(6.0);
        self.paddle_x = fw / 2.0 - self.paddle_width / 2.0;
        self.ball_x = fw / 2.0;
        self.ball_y = self.paddle_y - 1.0;
        self.init_bricks();
    }
}
