use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::games::Game;

const NUM_LANES: usize = 13;

#[derive(Clone)]
struct Lane {
    lane_type: LaneType,
    speed: f32,
    objects: Vec<Obj>,
}

#[derive(Clone, Copy, PartialEq)]
enum LaneType {
    Safe,
    Road,
    Water,
    Goal,
}

#[derive(Clone)]
struct Obj {
    x: f32,
    width: i32,
}

pub struct Frogger {
    frog_x: i32,
    frog_y: usize,
    lanes: Vec<Lane>,
    score: u32,
    high_score: u32,
    lives: u32,
    game_over: bool,
    won: bool,
    paused: bool,
    tick: u64,
    goals_reached: [bool; 5],
    field_width: i32,
}

impl Frogger {
    pub fn new() -> Self {
        let fw = 80;
        let mut f = Self {
            frog_x: fw / 2,
            frog_y: 11,
            lanes: Vec::new(),
            score: 0,
            high_score: 0,
            lives: 3,
            game_over: false,
            won: false,
            paused: false,
            tick: 0,
            goals_reached: [false; 5],
            field_width: fw,
        };
        f.init_lanes();
        f
    }

    fn init_lanes(&mut self) {
        self.lanes.clear();
        let mut rng = rand::thread_rng();

        for i in 0..NUM_LANES {
            let lane = match i {
                0 => Lane { lane_type: LaneType::Goal, speed: 0.0, objects: vec![] },
                1 => self.make_water_lane(0.15, 10, &mut rng),
                2 => self.make_water_lane(-0.12, 8, &mut rng),
                3 => self.make_water_lane(0.18, 12, &mut rng),
                4 => self.make_water_lane(-0.10, 9, &mut rng),
                5 => Lane { lane_type: LaneType::Safe, speed: 0.0, objects: vec![] },
                6 => self.make_road_lane(-0.20, 5, &mut rng),
                7 => self.make_road_lane(0.15, 4, &mut rng),
                8 => self.make_road_lane(-0.25, 6, &mut rng),
                9 => self.make_road_lane(0.12, 4, &mut rng),
                10 => self.make_road_lane(-0.18, 5, &mut rng),
                11 | 12 => Lane { lane_type: LaneType::Safe, speed: 0.0, objects: vec![] },
                _ => Lane { lane_type: LaneType::Safe, speed: 0.0, objects: vec![] },
            };
            self.lanes.push(lane);
        }
    }

    fn make_water_lane(&self, speed: f32, log_width: i32, rng: &mut impl Rng) -> Lane {
        let mut objects = Vec::new();
        let mut x = rng.gen_range(0..10) as f32;
        let fw = self.field_width as f32;
        while x < fw + 30.0 {
            objects.push(Obj { x, width: log_width });
            x += (log_width as f32) + rng.gen_range(6.0..14.0);
        }
        Lane { lane_type: LaneType::Water, speed, objects }
    }

    fn make_road_lane(&self, speed: f32, car_width: i32, rng: &mut impl Rng) -> Lane {
        let mut objects = Vec::new();
        let mut x = rng.gen_range(0..10) as f32;
        let fw = self.field_width as f32;
        while x < fw + 30.0 {
            objects.push(Obj { x, width: car_width });
            x += (car_width as f32) + rng.gen_range(8.0..18.0);
        }
        Lane { lane_type: LaneType::Road, speed, objects }
    }

    fn goal_positions(&self) -> Vec<i32> {
        let fw = self.field_width;
        let spacing = fw / 6;
        vec![spacing, spacing * 2, spacing * 3, spacing * 4, spacing * 5]
    }

    fn check_collision(&mut self) {
        if self.frog_y >= self.lanes.len() { return; }
        let lane = &self.lanes[self.frog_y];
        let fx = self.frog_x;

        match lane.lane_type {
            LaneType::Goal => {
                let goals = self.goal_positions();
                let mut scored = false;
                for (i, &gx) in goals.iter().enumerate() {
                    if i < 5 && (fx - gx).abs() <= 2 && !self.goals_reached[i] {
                        self.goals_reached[i] = true;
                        self.score += 100;
                        scored = true;
                        break;
                    }
                }
                if scored {
                    if self.goals_reached.iter().all(|&g| g) {
                        self.won = true;
                        self.score += 500;
                    }
                    self.frog_x = self.field_width / 2;
                    self.frog_y = 11;
                } else {
                    self.lose_life();
                }
            }
            LaneType::Road => {
                for obj in &lane.objects {
                    let ox = obj.x as i32;
                    if fx >= ox && fx < ox + obj.width {
                        self.lose_life();
                        return;
                    }
                }
            }
            LaneType::Water => {
                let mut on_log = false;
                for obj in &lane.objects {
                    let ox = obj.x as i32;
                    if fx >= ox && fx < ox + obj.width {
                        on_log = true;
                        break;
                    }
                }
                if !on_log {
                    self.lose_life();
                }
            }
            LaneType::Safe => {}
        }
    }

    fn lose_life(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            self.game_over = true;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
        }
        self.frog_x = self.field_width / 2;
        self.frog_y = 11;
    }

    fn move_frog_with_log(&mut self) {
        if self.frog_y < self.lanes.len() {
            let lane = &self.lanes[self.frog_y];
            if lane.lane_type == LaneType::Water {
                let speed = lane.speed;
                self.frog_x = (self.frog_x as f32 + speed).round() as i32;
                if self.frog_x < 0 || self.frog_x >= self.field_width {
                    self.lose_life();
                }
            }
        }
    }

    fn render_lane(&self, lane_idx: usize, width: usize) -> Line<'static> {
        let lane = &self.lanes[lane_idx];
        let w = width;
        let mut chars: Vec<(char, Style)> = vec![(' ', Style::default()); w];

        match lane.lane_type {
            LaneType::Safe => {
                // Grass with texture
                for (x, c) in chars.iter_mut().enumerate() {
                    let hash = (x.wrapping_mul(7) + lane_idx * 13) % 5;
                    let (ch, green) = match hash {
                        0 => ('"', 90),
                        1 => ('.', 70),
                        2 => (',', 80),
                        _ => (' ', 60),
                    };
                    *c = (ch, Style::default().fg(Color::Rgb(30, green as u8, 20)).bg(Color::Rgb(15, 45, 10)));
                }
            }
            LaneType::Goal => {
                // Water with goal pads
                let water_style = Style::default().fg(Color::Rgb(40, 80, 180)).bg(Color::Rgb(10, 30, 100));
                for (x, c) in chars.iter_mut().enumerate() {
                    let ch = if (x + self.tick as usize / 3) % 3 == 0 { '~' } else { '≈' };
                    *c = (ch, water_style);
                }
                let goals = self.goal_positions();
                for (i, &gx) in goals.iter().enumerate() {
                    if i >= 5 { break; }
                    for dx in -2..=2 {
                        let x = gx + dx;
                        if x >= 0 && (x as usize) < w {
                            if self.goals_reached[i] {
                                chars[x as usize] = ('★', Style::default()
                                    .fg(Color::Rgb(50, 220, 50))
                                    .bg(Color::Rgb(15, 60, 15))
                                    .add_modifier(Modifier::BOLD));
                            } else {
                                let ch = if dx == -2 || dx == 2 { '┃' } else if dx == 0 { '▼' } else { '─' };
                                chars[x as usize] = (ch, Style::default()
                                    .fg(Color::Rgb(200, 200, 50))
                                    .bg(Color::Rgb(40, 40, 15)));
                            }
                        }
                    }
                }
            }
            LaneType::Road => {
                // Asphalt road with markings
                let road_bg = Color::Rgb(35, 35, 40);
                for (_x, c) in chars.iter_mut().enumerate() {
                    *c = (' ', Style::default().bg(road_bg));
                }
                // Lane markings
                let marking_offset = if lane.speed > 0.0 { self.tick as usize / 2 } else { 1000usize.wrapping_sub(self.tick as usize / 2) };
                for x in 0..w {
                    if (x + marking_offset) % 8 < 3 {
                        chars[x] = ('─', Style::default().fg(Color::Rgb(120, 120, 40)).bg(road_bg));
                    }
                }
                // Draw vehicles
                let car_colors = [
                    Color::Rgb(220, 50, 50),    // Red
                    Color::Rgb(50, 120, 220),   // Blue
                    Color::Rgb(220, 180, 30),   // Yellow
                    Color::Rgb(180, 50, 200),   // Purple
                    Color::Rgb(220, 120, 30),   // Orange
                ];
                let color = car_colors[lane_idx % car_colors.len()];
                for obj in &lane.objects {
                    let ox = obj.x as i32;
                    for dx in 0..obj.width {
                        let x = ox + dx;
                        if x >= 0 && (x as usize) < w {
                            let (ch, fg) = if dx == 0 {
                                if lane.speed > 0.0 { ('▶', color) } else { ('◀', color) }
                            } else if dx == obj.width - 1 {
                                ('█', Color::Rgb(60, 60, 60))
                            } else if dx == 1 {
                                ('█', color)
                            } else {
                                ('▓', color)
                            };
                            chars[x as usize] = (ch, Style::default().fg(fg).bg(road_bg));
                        }
                    }
                }
            }
            LaneType::Water => {
                // Animated water
                let water_bg = Color::Rgb(10, 30, 100);
                for (x, c) in chars.iter_mut().enumerate() {
                    let phase = (x + self.tick as usize / 2 + lane_idx * 3) % 4;
                    let ch = match phase {
                        0 => '~',
                        1 => '≈',
                        2 => '~',
                        _ => '∽',
                    };
                    *c = (ch, Style::default().fg(Color::Rgb(50, 100, 200)).bg(water_bg));
                }
                // Draw logs
                let log_color = Color::Rgb(140, 90, 40);
                let log_dark = Color::Rgb(100, 65, 25);
                for obj in &lane.objects {
                    let ox = obj.x as i32;
                    for dx in 0..obj.width {
                        let x = ox + dx;
                        if x >= 0 && (x as usize) < w {
                            let (ch, fg) = if dx == 0 {
                                ('╣', log_dark)
                            } else if dx == obj.width - 1 {
                                ('╠', log_dark)
                            } else if dx % 3 == 0 {
                                ('█', log_dark)
                            } else {
                                ('▓', log_color)
                            };
                            chars[x as usize] = (ch, Style::default().fg(fg).bg(Color::Rgb(80, 50, 20)));
                        }
                    }
                }
            }
        }

        // Draw frog
        if lane_idx == self.frog_y {
            let fx = self.frog_x;
            if fx >= 0 && (fx as usize) < w {
                let frog_style = Style::default()
                    .fg(Color::Rgb(255, 255, 255))
                    .bg(Color::Rgb(30, 180, 30))
                    .add_modifier(Modifier::BOLD);
                // Draw a 3-char wide frog
                if fx > 0 && ((fx - 1) as usize) < w {
                    chars[(fx - 1) as usize] = ('(', frog_style);
                }
                chars[fx as usize] = ('▲', frog_style);
                if ((fx + 1) as usize) < w {
                    chars[(fx + 1) as usize] = (')', frog_style);
                }
            }
        }

        let spans: Vec<Span<'static>> = chars
            .into_iter()
            .map(|(ch, style)| Span::styled(String::from(ch), style))
            .collect();
        Line::from(spans)
    }
}

impl Game for Frogger {
    fn update(&mut self) {
        if self.game_over || self.won || self.paused { return; }
        self.tick += 1;

        for lane in &mut self.lanes {
            for obj in &mut lane.objects {
                obj.x += lane.speed;
                let fw = 200.0; // generous wrap range
                if lane.speed > 0.0 && obj.x > fw {
                    obj.x = -(obj.width as f32) - 2.0;
                } else if lane.speed < 0.0 && obj.x + obj.width as f32 + 2.0 < -fw + 150.0 {
                    obj.x = fw;
                }
            }
        }

        self.move_frog_with_log();
        self.check_collision();
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
                    KeyCode::Up => {
                        if self.frog_y > 0 {
                            self.frog_y -= 1;
                            self.score += 10;
                            self.check_collision();
                        }
                    }
                    KeyCode::Down => {
                        if self.frog_y < 11 {
                            self.frog_y += 1;
                        }
                    }
                    KeyCode::Left => {
                        self.frog_x = (self.frog_x - 2).max(1);
                    }
                    KeyCode::Right => {
                        self.frog_x = (self.frog_x + 2).min(self.field_width - 2);
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
            .border_style(Style::default().fg(Color::Rgb(50, 180, 50)))
            .title(" 🐸 Frogger ")
            .title_style(Style::default().fg(Color::Rgb(80, 220, 80)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Update field width dynamically
        let new_fw = inner.width as i32;
        if new_fw != self.field_width && (!self.game_over && !self.won) {
            // Adjust frog position proportionally
            let ratio = new_fw as f32 / self.field_width as f32;
            self.frog_x = (self.frog_x as f32 * ratio) as i32;
            self.field_width = new_fw;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(NUM_LANES as u16),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        // Status bar
        let status = Line::from(vec![
            Span::styled(" 🐸 ", Style::default()),
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
                format!("Goals: {}/5 ", self.goals_reached.iter().filter(|&&g| g).count()),
                Style::default().fg(Color::Green),
            ),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[0]);

        // Game field
        let field_width = chunks[1].width as usize;
        let mut lines: Vec<Line> = Vec::new();
        for i in 0..NUM_LANES {
            if i < self.lanes.len() {
                lines.push(self.render_lane(i, field_width));
            }
        }
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
                Span::styled("Press ENTER to play again", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.paused {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" ⏸ PAUSED - Press P to resume ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" ↑↓←→ Move ", Style::default().fg(Color::DarkGray)),
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
        *self = Frogger::new();
        self.high_score = hs;
        self.field_width = fw;
        self.frog_x = fw / 2;
    }
}
