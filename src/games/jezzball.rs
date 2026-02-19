use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::games::Game;

const MAX_BALLS: usize = 8;

#[derive(Clone)]
struct Ball {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum CellState {
    Empty,
    Filled,
    WallGrowing,
    WallComplete,
}

#[derive(Clone, Copy, PartialEq)]
enum WallDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
struct GrowingWall {
    origin_x: usize,
    origin_y: usize,
    direction: WallDirection,
    head_a: i32, // grows in positive direction
    head_b: i32, // grows in negative direction
    done_a: bool,
    done_b: bool,
}

pub struct JezzBall {
    field_width: usize,
    field_height: usize,
    grid: Vec<Vec<CellState>>,
    balls: Vec<Ball>,
    cursor_x: usize,
    cursor_y: usize,
    wall_dir: WallDirection,
    growing_walls: Vec<GrowingWall>,
    score: u32,
    high_score: u32,
    level: u32,
    lives: u32,
    game_over: bool,
    won_level: bool,
    paused: bool,
    tick: u64,
    total_empty: usize,
    target_percent: f32,
}

impl JezzBall {
    pub fn new() -> Self {
        let fw = 60;
        let fh = 24;
        let grid = vec![vec![CellState::Empty; fw]; fh];
        let total_empty = fw * fh;

        let mut s = Self {
            field_width: fw,
            field_height: fh,
            grid,
            balls: Vec::new(),
            cursor_x: fw / 2,
            cursor_y: fh / 2,
            wall_dir: WallDirection::Horizontal,
            growing_walls: Vec::new(),
            score: 0,
            high_score: 0,
            level: 1,
            lives: 3,
            game_over: false,
            won_level: false,
            paused: false,
            tick: 0,
            total_empty,
            target_percent: 75.0,
        };
        s.spawn_balls(2);
        s
    }

    fn spawn_balls(&mut self, count: usize) {
        self.balls.clear();
        for i in 0..count.min(MAX_BALLS) {
            let angle = std::f32::consts::PI * (0.3 + 0.5 * i as f32);
            let speed = 0.4;
            self.balls.push(Ball {
                x: self.field_width as f32 * 0.3 + (i as f32 * 7.0) % (self.field_width as f32 * 0.4),
                y: self.field_height as f32 * 0.3 + (i as f32 * 5.0) % (self.field_height as f32 * 0.4),
                dx: speed * angle.cos(),
                dy: speed * angle.sin(),
            });
        }
        // Ensure balls aren't on filled cells
        for ball in &mut self.balls {
            let bx = (ball.x as usize).min(self.field_width - 1);
            let by = (ball.y as usize).min(self.field_height - 1);
            if self.grid[by][bx] != CellState::Empty {
                ball.x = self.field_width as f32 / 2.0;
                ball.y = self.field_height as f32 / 2.0;
            }
        }
    }

    fn filled_percent(&self) -> f32 {
        let filled = self.grid.iter().flatten().filter(|c| {
            matches!(c, CellState::Filled | CellState::WallComplete)
        }).count();
        if self.total_empty == 0 {
            return 100.0;
        }
        (filled as f32 / self.total_empty as f32) * 100.0
    }

    fn move_balls(&mut self) -> bool {
        let mut wall_hit = false;

        for ball_idx in 0..self.balls.len() {
            let ball = &mut self.balls[ball_idx];
            let new_x = ball.x + ball.dx;
            let new_y = ball.y + ball.dy;

            // Boundary collisions
            if new_x < 0.0 || new_x >= self.field_width as f32 - 0.01 {
                ball.dx = -ball.dx;
            }
            if new_y < 0.0 || new_y >= self.field_height as f32 - 0.01 {
                ball.dy = -ball.dy;
            }

            ball.x = (ball.x + ball.dx).clamp(0.0, self.field_width as f32 - 0.01);
            ball.y = (ball.y + ball.dy).clamp(0.0, self.field_height as f32 - 0.01);

            let gx = ball.x as usize;
            let gy = ball.y as usize;

            if gx < self.field_width && gy < self.field_height {
                match self.grid[gy][gx] {
                    CellState::WallGrowing => {
                        // Ball hit a growing wall - destroy it and lose a life
                        wall_hit = true;
                        ball.dx = -ball.dx;
                        ball.dy = -ball.dy;
                        ball.x = (ball.x + ball.dx * 2.0).clamp(0.0, self.field_width as f32 - 0.01);
                        ball.y = (ball.y + ball.dy * 2.0).clamp(0.0, self.field_height as f32 - 0.01);
                    }
                    CellState::Filled | CellState::WallComplete => {
                        // Bounce off completed walls
                        // Check which direction to bounce
                        let prev_x = (ball.x - ball.dx) as usize;
                        let prev_y = (ball.y - ball.dy) as usize;

                        let check_x = gx.min(self.field_width - 1);
                        let check_py = prev_y.min(self.field_height - 1);
                        let check_px = prev_x.min(self.field_width - 1);

                        if check_py < self.field_height && self.grid[check_py][check_x] != CellState::Empty {
                            ball.dx = -ball.dx;
                        }
                        if check_px < self.field_width && gy < self.field_height && self.grid[gy][check_px] != CellState::Empty {
                            ball.dy = -ball.dy;
                        }
                        if check_py < self.field_height && check_px < self.field_width
                            && self.grid[check_py][check_x] == CellState::Empty
                            && self.grid[gy][check_px] == CellState::Empty
                        {
                            ball.dx = -ball.dx;
                            ball.dy = -ball.dy;
                        }

                        ball.x = (ball.x + ball.dx * 2.0).clamp(0.0, self.field_width as f32 - 0.01);
                        ball.y = (ball.y + ball.dy * 2.0).clamp(0.0, self.field_height as f32 - 0.01);
                    }
                    CellState::Empty => {}
                }
            }
        }

        wall_hit
    }

    fn grow_walls(&mut self) {
        let mut walls_to_remove = Vec::new();

        for (i, wall) in self.growing_walls.iter_mut().enumerate() {
            // Grow head_a (positive direction)
            if !wall.done_a {
                wall.head_a += 1;
                let (cx, cy) = match wall.direction {
                    WallDirection::Horizontal => (wall.origin_x as i32 + wall.head_a, wall.origin_y as i32),
                    WallDirection::Vertical => (wall.origin_x as i32, wall.origin_y as i32 + wall.head_a),
                };
                if cx < 0 || cx >= self.field_width as i32 || cy < 0 || cy >= self.field_height as i32 {
                    wall.done_a = true;
                } else {
                    let ux = cx as usize;
                    let uy = cy as usize;
                    match self.grid[uy][ux] {
                        CellState::Filled | CellState::WallComplete => {
                            wall.done_a = true;
                        }
                        _ => {
                            self.grid[uy][ux] = CellState::WallGrowing;
                        }
                    }
                }
            }

            // Grow head_b (negative direction)
            if !wall.done_b {
                wall.head_b -= 1;
                let (cx, cy) = match wall.direction {
                    WallDirection::Horizontal => (wall.origin_x as i32 + wall.head_b, wall.origin_y as i32),
                    WallDirection::Vertical => (wall.origin_x as i32, wall.origin_y as i32 + wall.head_b),
                };
                if cx < 0 || cx >= self.field_width as i32 || cy < 0 || cy >= self.field_height as i32 {
                    wall.done_b = true;
                } else {
                    let ux = cx as usize;
                    let uy = cy as usize;
                    match self.grid[uy][ux] {
                        CellState::Filled | CellState::WallComplete => {
                            wall.done_b = true;
                        }
                        _ => {
                            self.grid[uy][ux] = CellState::WallGrowing;
                        }
                    }
                }
            }

            if wall.done_a && wall.done_b {
                walls_to_remove.push(i);
            }
        }

        // Complete finished walls
        for &i in walls_to_remove.iter().rev() {
            let wall = &self.growing_walls[i];
            // Convert growing cells to complete
            match wall.direction {
                WallDirection::Horizontal => {
                    let y = wall.origin_y;
                    for x in 0..self.field_width {
                        if y < self.field_height && self.grid[y][x] == CellState::WallGrowing {
                            self.grid[y][x] = CellState::WallComplete;
                        }
                    }
                }
                WallDirection::Vertical => {
                    let x = wall.origin_x;
                    for y in 0..self.field_height {
                        if x < self.field_width && self.grid[y][x] == CellState::WallGrowing {
                            self.grid[y][x] = CellState::WallComplete;
                        }
                    }
                }
            }
            self.growing_walls.remove(i);

            // After completing a wall, fill regions that don't contain balls
            self.fill_empty_regions();

            // Award score for filling
            self.score += 10;
        }
    }

    fn destroy_growing_walls(&mut self) {
        // Remove all growing wall cells from the grid
        for y in 0..self.field_height {
            for x in 0..self.field_width {
                if self.grid[y][x] == CellState::WallGrowing {
                    self.grid[y][x] = CellState::Empty;
                }
            }
        }
        self.growing_walls.clear();
    }

    fn fill_empty_regions(&mut self) {
        // Flood fill to find regions, then fill any region that doesn't contain a ball
        let w = self.field_width;
        let h = self.field_height;
        let mut visited = vec![vec![false; w]; h];
        let mut regions: Vec<Vec<(usize, usize)>> = Vec::new();

        for sy in 0..h {
            for sx in 0..w {
                if visited[sy][sx] || self.grid[sy][sx] != CellState::Empty {
                    continue;
                }
                // BFS flood fill
                let mut region = Vec::new();
                let mut queue = std::collections::VecDeque::new();
                queue.push_back((sx, sy));
                visited[sy][sx] = true;

                while let Some((cx, cy)) = queue.pop_front() {
                    region.push((cx, cy));
                    for (nx, ny) in [
                        (cx.wrapping_sub(1), cy),
                        (cx + 1, cy),
                        (cx, cy.wrapping_sub(1)),
                        (cx, cy + 1),
                    ] {
                        if nx < w && ny < h && !visited[ny][nx] && self.grid[ny][nx] == CellState::Empty {
                            visited[ny][nx] = true;
                            queue.push_back((nx, ny));
                        }
                    }
                }
                regions.push(region);
            }
        }

        // Check each region for balls
        for region in &regions {
            let has_ball = self.balls.iter().any(|ball| {
                let bx = ball.x as usize;
                let by = ball.y as usize;
                region.iter().any(|&(rx, ry)| rx == bx && ry == by)
            });

            if !has_ball && !region.is_empty() {
                // Fill this region
                let region_size = region.len();
                for &(rx, ry) in region {
                    self.grid[ry][rx] = CellState::Filled;
                }
                self.score += region_size as u32;
            }
        }
    }

    fn launch_wall(&mut self) {
        let cx = self.cursor_x;
        let cy = self.cursor_y;

        if cx >= self.field_width || cy >= self.field_height {
            return;
        }
        if self.grid[cy][cx] != CellState::Empty {
            return;
        }

        // Place origin
        self.grid[cy][cx] = CellState::WallGrowing;

        self.growing_walls.push(GrowingWall {
            origin_x: cx,
            origin_y: cy,
            direction: self.wall_dir,
            head_a: 0,
            head_b: 0,
            done_a: false,
            done_b: false,
        });
    }

    fn advance_level(&mut self) {
        self.level += 1;
        let num_balls = (self.level as usize + 1).min(MAX_BALLS);
        self.grid = vec![vec![CellState::Empty; self.field_width]; self.field_height];
        self.growing_walls.clear();
        self.won_level = false;
        self.spawn_balls(num_balls);
    }

    fn render_field(&self, width: usize, height: usize) -> Vec<Line<'static>> {
        let w = width.min(self.field_width);
        let h = height.min(self.field_height);

        let sx = w as f32 / self.field_width as f32;
        let sy = h as f32 / self.field_height as f32;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(5, 5, 15))); w]; h];

        // Draw grid cells
        for gy in 0..self.field_height {
            for gx in 0..self.field_width {
                let px = (gx as f32 * sx) as usize;
                let py = (gy as f32 * sy) as usize;
                if px >= w || py >= h {
                    continue;
                }
                match self.grid[gy][gx] {
                    CellState::Empty => {}
                    CellState::Filled => {
                        grid[py][px] = ('█', Style::default()
                            .fg(Color::Rgb(30, 60, 120))
                            .bg(Color::Rgb(15, 30, 60)));
                    }
                    CellState::WallGrowing => {
                        let blink = if self.tick % 4 < 2 { '▓' } else { '░' };
                        grid[py][px] = (blink, Style::default()
                            .fg(Color::Rgb(255, 200, 50))
                            .bg(Color::Rgb(80, 60, 10)));
                    }
                    CellState::WallComplete => {
                        grid[py][px] = ('▓', Style::default()
                            .fg(Color::Rgb(50, 100, 180))
                            .bg(Color::Rgb(20, 40, 80)));
                    }
                }
            }
        }

        // Draw balls
        let ball_colors = [
            Color::Rgb(255, 80, 80),
            Color::Rgb(80, 255, 80),
            Color::Rgb(80, 80, 255),
            Color::Rgb(255, 255, 80),
            Color::Rgb(255, 80, 255),
            Color::Rgb(80, 255, 255),
            Color::Rgb(255, 160, 80),
            Color::Rgb(200, 200, 200),
        ];
        for (i, ball) in self.balls.iter().enumerate() {
            let px = (ball.x * sx) as usize;
            let py = (ball.y * sy) as usize;
            if px < w && py < h {
                let color = ball_colors[i % ball_colors.len()];
                grid[py][px] = ('●', Style::default()
                    .fg(color)
                    .bg(Color::Rgb(5, 5, 15))
                    .add_modifier(Modifier::BOLD));
            }
        }

        // Draw cursor
        let cpx = (self.cursor_x as f32 * sx) as usize;
        let cpy = (self.cursor_y as f32 * sy) as usize;
        if cpx < w && cpy < h {
            let cursor_char = match self.wall_dir {
                WallDirection::Horizontal => '─',
                WallDirection::Vertical => '│',
            };
            let blink = self.tick % 6 < 4;
            let cursor_color = if blink {
                Color::Rgb(255, 255, 255)
            } else {
                Color::Rgb(150, 150, 150)
            };
            grid[cpy][cpx] = (cursor_char, Style::default()
                .fg(cursor_color)
                .bg(Color::Rgb(40, 40, 60))
                .add_modifier(Modifier::BOLD));

            // Draw direction indicator
            match self.wall_dir {
                WallDirection::Horizontal => {
                    if cpx > 0 {
                        grid[cpy][cpx - 1] = ('╶', Style::default()
                            .fg(Color::Rgb(100, 100, 140))
                            .bg(Color::Rgb(5, 5, 15)));
                    }
                    if cpx + 1 < w {
                        grid[cpy][cpx + 1] = ('╴', Style::default()
                            .fg(Color::Rgb(100, 100, 140))
                            .bg(Color::Rgb(5, 5, 15)));
                    }
                }
                WallDirection::Vertical => {
                    if cpy > 0 {
                        grid[cpy - 1][cpx] = ('╵', Style::default()
                            .fg(Color::Rgb(100, 100, 140))
                            .bg(Color::Rgb(5, 5, 15)));
                    }
                    if cpy + 1 < h {
                        grid[cpy + 1][cpx] = ('╷', Style::default()
                            .fg(Color::Rgb(100, 100, 140))
                            .bg(Color::Rgb(5, 5, 15)));
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

impl Game for JezzBall {
    fn update(&mut self) {
        if self.game_over || self.won_level || self.paused {
            return;
        }
        self.tick += 1;

        // Move balls and check for wall hits
        let wall_hit = self.move_balls();
        if wall_hit {
            self.destroy_growing_walls();
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.game_over = true;
                if self.score > self.high_score {
                    self.high_score = self.score;
                }
            }
        }

        // Grow walls
        if self.tick % 2 == 0 {
            self.grow_walls();
        }

        // Check win condition
        if self.filled_percent() >= self.target_percent {
            self.won_level = true;
            self.score += 100 * self.level;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
        }
    }

    fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => self.reset(),
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if !self.game_over && !self.won_level {
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
                if self.won_level {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        self.advance_level();
                    }
                    return;
                }
                if self.paused {
                    return;
                }
                match key.code {
                    KeyCode::Left => {
                        if self.cursor_x > 0 {
                            self.cursor_x -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.cursor_x < self.field_width - 1 {
                            self.cursor_x += 1;
                        }
                    }
                    KeyCode::Up => {
                        if self.cursor_y > 0 {
                            self.cursor_y -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.cursor_y < self.field_height - 1 {
                            self.cursor_y += 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        self.launch_wall();
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        // Toggle wall direction
                        self.wall_dir = match self.wall_dir {
                            WallDirection::Horizontal => WallDirection::Vertical,
                            WallDirection::Vertical => WallDirection::Horizontal,
                        };
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
            .border_style(Style::default().fg(Color::Rgb(80, 150, 220)))
            .title(" 🟦 JezzBall ")
            .title_style(Style::default().fg(Color::Rgb(100, 180, 255)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Resize grid if needed
        let new_fw = inner.width as usize;
        let new_fh = inner.height.saturating_sub(2) as usize;
        if new_fw > 4 && new_fh > 4 && !self.game_over && !self.won_level && self.growing_walls.is_empty() {
            if new_fw != self.field_width || new_fh != self.field_height {
                self.field_width = new_fw;
                self.field_height = new_fh;
                self.grid = vec![vec![CellState::Empty; new_fw]; new_fh];
                self.total_empty = new_fw * new_fh;
                self.cursor_x = self.cursor_x.min(new_fw.saturating_sub(1));
                self.cursor_y = self.cursor_y.min(new_fh.saturating_sub(1));
                for ball in &mut self.balls {
                    ball.x = ball.x.min(new_fw as f32 - 0.01);
                    ball.y = ball.y.min(new_fh as f32 - 0.01);
                }
            }
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(4),
                Constraint::Length(1),
            ])
            .split(inner);

        // Status bar
        let dir_label = match self.wall_dir {
            WallDirection::Horizontal => "Horiz ─",
            WallDirection::Vertical => "Vert │",
        };
        let pct = self.filled_percent();
        let status = Line::from(vec![
            Span::styled(" 🟦 ", Style::default()),
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
                format!("Level: {} ", self.level),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Filled: {:.0}%/{:.0}% ", pct, self.target_percent),
                Style::default().fg(if pct >= self.target_percent * 0.8 { Color::Green } else { Color::White }),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("🏆 {} ", self.high_score),
                Style::default().fg(Color::Rgb(180, 140, 50)),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Dir: {} ", dir_label),
                Style::default().fg(Color::Rgb(150, 200, 255)),
            ),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[0]);

        // Game field
        let fw = chunks[1].width as usize;
        let fh = chunks[1].height as usize;
        if fw > 0 && fh > 0 {
            let lines = self.render_field(fw, fh);
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }

        // Help/status bar
        if self.game_over {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" 💀 GAME OVER! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("Score: {} │ Press ENTER to restart, Esc for menu", self.score),
                    Style::default().fg(Color::Gray),
                ),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.won_level {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" 🎉 LEVEL COMPLETE! ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("Score: {} │ Press ENTER for next level", self.score),
                    Style::default().fg(Color::Gray),
                ),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.paused {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" ⏸ PAUSED - Press P to resume ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" ←↑↓→ Move ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("SPACE Launch Wall ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("D Toggle Dir ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("P Pause ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("R Reset ", Style::default().fg(Color::DarkGray)),
                Span::styled("│ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("Esc Menu", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(help, chunks[2]);
        }
    }

    fn reset(&mut self) {
        let hs = self.high_score;
        *self = JezzBall::new();
        self.high_score = hs;
    }
}
