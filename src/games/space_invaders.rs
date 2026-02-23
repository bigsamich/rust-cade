use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use std::collections::HashMap;

use crate::games::Game;

const PLAYER_SPEED: f32 = 1.5;
const PLAYER_BULLET_SPEED: f32 = 0.8;
const ALIEN_BULLET_SPEED: f32 = 0.4;
const MAX_PLAYER_BULLETS: usize = 3;
const MAX_ALIEN_BULLETS: usize = 5;
const ALIEN_COLS: usize = 11;
const ALIEN_ROWS: usize = 5;
const ALIEN_H_SPACING: f32 = 4.5;
const ALIEN_V_SPACING: f32 = 3.5;
const SHIELD_COUNT: usize = 4;
const SHIELD_WIDTH: f32 = 6.0;
const SHIELD_HEIGHT: f32 = 3.0;

#[derive(Clone, Copy, PartialEq)]
enum AlienKind {
    Top,    // small, 30 pts
    Mid,    // medium, 20 pts
    Bot,    // large, 10 pts
}

impl AlienKind {
    fn points(&self) -> u32 {
        match self {
            AlienKind::Top => 30,
            AlienKind::Mid => 20,
            AlienKind::Bot => 10,
        }
    }
}

#[derive(Clone)]
struct Alien {
    x: f32,
    y: f32,
    kind: AlienKind,
    alive: bool,
}

#[derive(Clone)]
struct Bullet {
    x: f32,
    y: f32,
    dy: f32,
}

#[derive(Clone)]
struct Shield {
    x: f32,
    y: f32,
    // 2D grid of braille-resolution pixels for damage
    pixels: Vec<Vec<bool>>,
    pw: usize,
    ph: usize,
}

impl Shield {
    fn new(x: f32, y: f32) -> Self {
        let pw = (SHIELD_WIDTH * 2.0) as usize; // braille-res width
        let ph = (SHIELD_HEIGHT * 4.0) as usize; // braille-res height
        let mut pixels = vec![vec![false; pw]; ph];
        // Fill arch shape
        let cx = pw as f32 / 2.0;
        for row in 0..ph {
            for col in 0..pw {
                let dx = col as f32 - cx;
                let top_curve = if row < ph / 3 {
                    // Curved top
                    let r = cx;
                    dx * dx + (row as f32 - ph as f32 / 3.0).powi(2) < r * r
                } else {
                    true
                };
                // Notch at bottom center
                let notch = row >= ph * 3 / 4
                    && col >= pw * 2 / 5
                    && col <= pw * 3 / 5;
                pixels[row][col] = top_curve && !notch;
            }
        }
        Shield { x, y, pixels, pw, ph }
    }

    fn damage_at(&mut self, fx: f32, fy: f32, radius: f32, bsx: f32, bsy: f32) -> bool {
        let local_x = (fx - self.x) * bsx;
        let local_y = (fy - self.y) * bsy;
        let r = radius * bsx.max(bsy);
        let mut hit = false;
        for dy in -(r as i32)..=(r as i32) {
            for dx in -(r as i32)..=(r as i32) {
                if (dx * dx + dy * dy) as f32 <= r * r {
                    let px = (local_x as i32 + dx) as usize;
                    let py = (local_y as i32 + dy) as usize;
                    if py < self.ph && px < self.pw && self.pixels[py][px] {
                        self.pixels[py][px] = false;
                        hit = true;
                    }
                }
            }
        }
        hit
    }

    fn hit_test(&self, fx: f32, fy: f32, bsx: f32, bsy: f32) -> bool {
        let local_x = (fx - self.x) * bsx;
        let local_y = (fy - self.y) * bsy;
        let px = local_x as usize;
        let py = local_y as usize;
        py < self.ph && px < self.pw && self.pixels[py][px]
    }
}

pub struct SpaceInvaders {
    player_x: f32,
    player_bullets: Vec<Bullet>,
    alien_bullets: Vec<Bullet>,
    aliens: Vec<Alien>,
    shields: Vec<Shield>,
    alien_dir: f32,       // 1.0 = right, -1.0 = left
    alien_speed: f32,
    alien_move_timer: u64,
    alien_move_interval: u64,
    alien_fire_timer: u64,
    score: u32,
    high_score: u32,
    lives: u32,
    level: u32,
    game_over: bool,
    paused: bool,
    tick: u64,
    field_width: f32,
    field_height: f32,
    rng_state: u32,
}

impl SpaceInvaders {
    pub fn new() -> Self {
        let fw = 80.0;
        let fh = 35.0;
        let mut s = Self {
            player_x: fw / 2.0,
            player_bullets: Vec::new(),
            alien_bullets: Vec::new(),
            aliens: Vec::new(),
            shields: Vec::new(),
            alien_dir: 1.0,
            alien_speed: 0.8,
            alien_move_timer: 0,
            alien_move_interval: 30,
            alien_fire_timer: 0,
            score: 0,
            high_score: 0,
            lives: 3,
            level: 1,
            game_over: false,
            paused: false,
            tick: 0,
            field_width: fw,
            field_height: fh,
            rng_state: 12345,
        };
        s.init_aliens();
        s.init_shields();
        s
    }

    fn cheap_rand(&mut self) -> u32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state
    }

    fn init_aliens(&mut self) {
        self.aliens.clear();
        let start_x = (self.field_width - (ALIEN_COLS as f32 - 1.0) * ALIEN_H_SPACING) / 2.0;
        let start_y = 3.0;
        for row in 0..ALIEN_ROWS {
            let kind = match row {
                0 => AlienKind::Top,
                1 | 2 => AlienKind::Mid,
                _ => AlienKind::Bot,
            };
            for col in 0..ALIEN_COLS {
                self.aliens.push(Alien {
                    x: start_x + col as f32 * ALIEN_H_SPACING,
                    y: start_y + row as f32 * ALIEN_V_SPACING,
                    kind,
                    alive: true,
                });
            }
        }
    }

    fn init_shields(&mut self) {
        self.shields.clear();
        let shield_y = self.field_height - 8.0;
        let total_w = SHIELD_COUNT as f32 * SHIELD_WIDTH + (SHIELD_COUNT as f32 - 1.0) * 8.0;
        let start_x = (self.field_width - total_w) / 2.0;
        for i in 0..SHIELD_COUNT {
            let sx = start_x + i as f32 * (SHIELD_WIDTH + 8.0);
            self.shields.push(Shield::new(sx, shield_y));
        }
    }

    fn player_y(&self) -> f32 {
        self.field_height - 2.5
    }

    fn update_bullets(&mut self) {
        // Move player bullets up
        for b in &mut self.player_bullets {
            b.y += b.dy;
        }
        self.player_bullets.retain(|b| b.y > -1.0);

        // Move alien bullets down
        for b in &mut self.alien_bullets {
            b.y += b.dy;
        }
        self.alien_bullets.retain(|b| b.y < self.field_height + 1.0);
    }

    fn update_aliens(&mut self) {
        self.alien_move_timer += 1;

        // Speed up as fewer aliens remain
        let alive = self.aliens.iter().filter(|a| a.alive).count();
        let total = ALIEN_ROWS * ALIEN_COLS;
        self.alien_move_interval = if alive <= 1 {
            3
        } else if alive <= total / 8 {
            5
        } else if alive <= total / 4 {
            8
        } else if alive <= total / 2 {
            14
        } else {
            (22u64).saturating_sub(self.level as u64 * 2).max(6)
        };

        if self.alien_move_timer >= self.alien_move_interval {
            self.alien_move_timer = 0;

            // Check edges
            let mut hit_edge = false;
            for alien in &self.aliens {
                if !alien.alive { continue; }
                let next_x = alien.x + self.alien_dir * self.alien_speed;
                if next_x < 2.0 || next_x > self.field_width - 2.0 {
                    hit_edge = true;
                    break;
                }
            }

            if hit_edge {
                // Move down and reverse
                for alien in &mut self.aliens {
                    if alien.alive {
                        alien.y += 1.2;
                    }
                }
                self.alien_dir = -self.alien_dir;
            } else {
                // Move horizontally
                for alien in &mut self.aliens {
                    if alien.alive {
                        alien.x += self.alien_dir * self.alien_speed;
                    }
                }
            }
        }

        // Alien shooting
        self.alien_fire_timer += 1;
        let fire_interval = (60u64).saturating_sub(self.level as u64 * 5).max(15);
        if self.alien_fire_timer >= fire_interval && self.alien_bullets.len() < MAX_ALIEN_BULLETS {
            self.alien_fire_timer = 0;
            // Pick a random alive alien from the bottom of each column
            let mut bottom_aliens: Vec<usize> = Vec::new();
            for col in 0..ALIEN_COLS {
                let mut lowest: Option<usize> = None;
                for (i, alien) in self.aliens.iter().enumerate() {
                    if !alien.alive { continue; }
                    let acol = ((alien.x - 5.0) / ALIEN_H_SPACING).round() as usize;
                    if acol == col {
                        if lowest.is_none() || alien.y > self.aliens[lowest.unwrap()].y {
                            lowest = Some(i);
                        }
                    }
                }
                if let Some(idx) = lowest {
                    bottom_aliens.push(idx);
                }
            }
            if !bottom_aliens.is_empty() {
                let pick = self.cheap_rand() as usize % bottom_aliens.len();
                let alien = &self.aliens[bottom_aliens[pick]];
                let speed = ALIEN_BULLET_SPEED + self.level as f32 * 0.03;
                self.alien_bullets.push(Bullet {
                    x: alien.x,
                    y: alien.y + 1.0,
                    dy: speed,
                });
            }
        }
    }

    fn check_collisions(&mut self) {
        let bsx = 2.0; // braille scale for shield damage
        let bsy = 4.0;

        // Player bullets vs aliens
        let mut bullets_remove = Vec::new();
        for (bi, bullet) in self.player_bullets.iter().enumerate() {
            for alien in &mut self.aliens {
                if !alien.alive { continue; }
                let dx = (bullet.x - alien.x).abs();
                let dy = (bullet.y - alien.y).abs();
                if dx < 2.0 && dy < 1.5 {
                    alien.alive = false;
                    self.score += alien.kind.points();
                    bullets_remove.push(bi);
                    break;
                }
            }
        }
        bullets_remove.sort_unstable();
        bullets_remove.dedup();
        for &i in bullets_remove.iter().rev() {
            if i < self.player_bullets.len() { self.player_bullets.remove(i); }
        }

        // Player bullets vs shields
        let mut bullets_remove = Vec::new();
        for (bi, bullet) in self.player_bullets.iter().enumerate() {
            for shield in &mut self.shields {
                if bullet.x >= shield.x && bullet.x <= shield.x + SHIELD_WIDTH
                    && bullet.y >= shield.y && bullet.y <= shield.y + SHIELD_HEIGHT
                {
                    if shield.hit_test(bullet.x, bullet.y, bsx, bsy) {
                        shield.damage_at(bullet.x, bullet.y, 0.8, bsx, bsy);
                        bullets_remove.push(bi);
                        break;
                    }
                }
            }
        }
        bullets_remove.sort_unstable();
        bullets_remove.dedup();
        for &i in bullets_remove.iter().rev() {
            if i < self.player_bullets.len() { self.player_bullets.remove(i); }
        }

        // Alien bullets vs shields
        let mut bullets_remove = Vec::new();
        for (bi, bullet) in self.alien_bullets.iter().enumerate() {
            for shield in &mut self.shields {
                if bullet.x >= shield.x && bullet.x <= shield.x + SHIELD_WIDTH
                    && bullet.y >= shield.y && bullet.y <= shield.y + SHIELD_HEIGHT
                {
                    if shield.hit_test(bullet.x, bullet.y, bsx, bsy) {
                        shield.damage_at(bullet.x, bullet.y, 0.8, bsx, bsy);
                        bullets_remove.push(bi);
                        break;
                    }
                }
            }
        }
        bullets_remove.sort_unstable();
        bullets_remove.dedup();
        for &i in bullets_remove.iter().rev() {
            if i < self.alien_bullets.len() { self.alien_bullets.remove(i); }
        }

        // Alien bullets vs player
        let py = self.player_y();
        let mut bullets_remove = Vec::new();
        for (bi, bullet) in self.alien_bullets.iter().enumerate() {
            let dx = (bullet.x - self.player_x).abs();
            let dy = (bullet.y - py).abs();
            if dx < 2.5 && dy < 1.2 {
                bullets_remove.push(bi);
                self.lives = self.lives.saturating_sub(1);
                if self.lives == 0 {
                    self.game_over = true;
                    if self.score > self.high_score {
                        self.high_score = self.score;
                    }
                }
            }
        }
        for &i in bullets_remove.iter().rev() {
            if i < self.alien_bullets.len() { self.alien_bullets.remove(i); }
        }

        // Aliens reaching bottom
        for alien in &self.aliens {
            if alien.alive && alien.y >= self.field_height - 4.0 {
                self.game_over = true;
                if self.score > self.high_score {
                    self.high_score = self.score;
                }
                break;
            }
        }

        // All aliens dead = win level
        if self.aliens.iter().all(|a| !a.alive) {
            self.level += 1;
            self.init_aliens();
            self.init_shields();
            self.player_bullets.clear();
            self.alien_bullets.clear();
        }
    }

    // ── Braille rendering ──────────────────────────────────────────────

    fn braille_bit(sub_x: usize, sub_y: usize) -> u8 {
        match (sub_x, sub_y) {
            (0, 0) => 0x01,
            (0, 1) => 0x02,
            (0, 2) => 0x04,
            (0, 3) => 0x40,
            (1, 0) => 0x08,
            (1, 1) => 0x10,
            (1, 2) => 0x20,
            (1, 3) => 0x80,
            _ => 0,
        }
    }

    fn set_dot(map: &mut HashMap<(usize, usize), u8>, bx: i32, by: i32, bw: i32, bh: i32) {
        if bx < 0 || by < 0 || bx >= bw || by >= bh { return; }
        let cx = bx as usize / 2;
        let cy = by as usize / 4;
        let sx = bx as usize % 2;
        let sy = by as usize % 4;
        *map.entry((cx, cy)).or_insert(0) |= Self::braille_bit(sx, sy);
    }

    fn write_layer(
        grid: &mut [Vec<(char, Style)>],
        map: &HashMap<(usize, usize), u8>,
        w: usize, h: usize,
        color: Color, bg: Color, bold: bool,
    ) {
        for (&(cx, cy), &bits) in map {
            if cx < w && cy < h && bits != 0 {
                let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                let mut style = Style::default().fg(color).bg(bg);
                if bold { style = style.add_modifier(Modifier::BOLD); }
                grid[cy][cx] = (ch, style);
            }
        }
    }

    fn render_alien_sprite(map: &mut HashMap<(usize, usize), u8>, cx: i32, cy: i32, kind: AlienKind, frame: bool, bw: i32, bh: i32) {
        // All sprites are defined on a ~7x5 braille-pixel grid centered at (cx, cy)
        match kind {
            AlienKind::Top => {
                // Small squid shape
                let pixels: &[(i32, i32)] = if frame {
                    &[
                        (0,-2),
                        (-1,-1),(0,-1),(1,-1),
                        (-2,0),(-1,0),(0,0),(1,0),(2,0),
                        (-2,1),(0,1),(2,1),
                        (-1,2),(1,2),
                    ]
                } else {
                    &[
                        (0,-2),
                        (-1,-1),(0,-1),(1,-1),
                        (-2,0),(-1,0),(0,0),(1,0),(2,0),
                        (-2,1),(0,1),(2,1),
                        (-3,2),(3,2),
                    ]
                };
                for &(dx, dy) in pixels {
                    Self::set_dot(map, cx + dx, cy + dy, bw, bh);
                }
            }
            AlienKind::Mid => {
                // Crab shape
                let pixels: &[(i32, i32)] = if frame {
                    &[
                        (-1,-2),(1,-2),
                        (-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),
                        (-3,0),(-2,0),(-1,0),(0,0),(1,0),(2,0),(3,0),
                        (-3,1),(-1,1),(0,1),(1,1),(3,1),
                        (-3,2),(-2,2),(2,2),(3,2),
                    ]
                } else {
                    &[
                        (-1,-2),(1,-2),
                        (-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),
                        (-3,0),(-2,0),(-1,0),(0,0),(1,0),(2,0),(3,0),
                        (-3,1),(-1,1),(0,1),(1,1),(3,1),
                        (-2,2),(-1,2),(1,2),(2,2),
                    ]
                };
                for &(dx, dy) in pixels {
                    Self::set_dot(map, cx + dx, cy + dy, bw, bh);
                }
            }
            AlienKind::Bot => {
                // Octopus shape
                let pixels: &[(i32, i32)] = if frame {
                    &[
                        (-2,-2),(-1,-2),(0,-2),(1,-2),(2,-2),
                        (-3,-1),(-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),(3,-1),
                        (-3,0),(-2,0),(0,0),(2,0),(3,0),
                        (-3,1),(-1,1),(0,1),(1,1),(3,1),
                        (-2,2),(2,2),
                    ]
                } else {
                    &[
                        (-2,-2),(-1,-2),(0,-2),(1,-2),(2,-2),
                        (-3,-1),(-2,-1),(-1,-1),(0,-1),(1,-1),(2,-1),(3,-1),
                        (-3,0),(-2,0),(0,0),(2,0),(3,0),
                        (-3,1),(-1,1),(0,1),(1,1),(3,1),
                        (-3,2),(3,2),
                    ]
                };
                for &(dx, dy) in pixels {
                    Self::set_dot(map, cx + dx, cy + dy, bw, bh);
                }
            }
        }
    }

    fn render_player_ship(map: &mut HashMap<(usize, usize), u8>, cx: i32, cy: i32, bw: i32, bh: i32) {
        // Cannon / ship shape
        let pixels: &[(i32, i32)] = &[
            (0,-3),
            (-1,-2),(0,-2),(1,-2),
            (-1,-1),(0,-1),(1,-1),
            (-3,0),(-2,0),(-1,0),(0,0),(1,0),(2,0),(3,0),
            (-4,1),(-3,1),(-2,1),(-1,1),(0,1),(1,1),(2,1),(3,1),(4,1),
            (-4,2),(-3,2),(-2,2),(-1,2),(0,2),(1,2),(2,2),(3,2),(4,2),
        ];
        for &(dx, dy) in pixels {
            Self::set_dot(map, cx + dx, cy + dy, bw, bh);
        }
    }

    fn render_field(&self, width: usize, height: usize) -> Vec<Line<'static>> {
        let w = width;
        let h = height;
        let bw = (w * 2) as i32;
        let bh = (h * 4) as i32;
        let bsx = bw as f32 / self.field_width;
        let bsy = bh as f32 / self.field_height;

        let bg = Color::Rgb(0, 0, 5);
        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(bg)); w]; h];

        let anim_frame = (self.tick / 15) % 2 == 0;

        // ── Aliens ─────────────────────────────────────────────────────
        for alien in &self.aliens {
            if !alien.alive { continue; }
            let mut amap: HashMap<(usize, usize), u8> = HashMap::new();
            let cx = (alien.x * bsx) as i32;
            let cy = (alien.y * bsy) as i32;
            Self::render_alien_sprite(&mut amap, cx, cy, alien.kind, anim_frame, bw, bh);

            let color = match alien.kind {
                AlienKind::Top => Color::Rgb(255, 80, 80),
                AlienKind::Mid => Color::Rgb(80, 255, 150),
                AlienKind::Bot => Color::Rgb(200, 180, 255),
            };
            Self::write_layer(&mut grid, &amap, w, h, color, bg, false);
        }

        // ── Shields ────────────────────────────────────────────────────
        for shield in &self.shields {
            let mut smap: HashMap<(usize, usize), u8> = HashMap::new();
            let sx0 = (shield.x * bsx) as i32;
            let sy0 = (shield.y * bsy) as i32;
            for row in 0..shield.ph {
                for col in 0..shield.pw {
                    if shield.pixels[row][col] {
                        Self::set_dot(&mut smap, sx0 + col as i32, sy0 + row as i32, bw, bh);
                    }
                }
            }
            Self::write_layer(&mut grid, &smap, w, h, Color::Rgb(40, 200, 40), bg, false);
        }

        // ── Player bullets ─────────────────────────────────────────────
        for bullet in &self.player_bullets {
            let mut bmap: HashMap<(usize, usize), u8> = HashMap::new();
            let bx = (bullet.x * bsx) as i32;
            let by = (bullet.y * bsy) as i32;
            for dy in 0..3 {
                Self::set_dot(&mut bmap, bx, by + dy, bw, bh);
            }
            Self::write_layer(&mut grid, &bmap, w, h, Color::Rgb(255, 255, 200), bg, true);
        }

        // ── Alien bullets ──────────────────────────────────────────────
        for bullet in &self.alien_bullets {
            let mut bmap: HashMap<(usize, usize), u8> = HashMap::new();
            let bx = (bullet.x * bsx) as i32;
            let by = (bullet.y * bsy) as i32;
            // Zigzag bolt shape
            let zigzag = if (self.tick / 4) % 2 == 0 {
                [(0,0),(1,1),(0,2),(-1,3),(0,4)]
            } else {
                [(0,0),(-1,1),(0,2),(1,3),(0,4)]
            };
            for &(dx, dy) in &zigzag {
                Self::set_dot(&mut bmap, bx + dx, by + dy, bw, bh);
            }
            Self::write_layer(&mut grid, &bmap, w, h, Color::Rgb(255, 100, 100), bg, true);
        }

        // ── Player ship ────────────────────────────────────────────────
        if !self.game_over {
            let mut pmap: HashMap<(usize, usize), u8> = HashMap::new();
            let px = (self.player_x * bsx) as i32;
            let py = (self.player_y() * bsy) as i32;
            Self::render_player_ship(&mut pmap, px, py, bw, bh);
            Self::write_layer(&mut grid, &pmap, w, h, Color::Rgb(80, 255, 80), bg, true);
        }

        // ── Ground line ────────────────────────────────────────────────
        let ground_y = h.saturating_sub(1);
        if ground_y < h {
            for x in 0..w {
                grid[ground_y][x] = ('\u{2500}', Style::default().fg(Color::Rgb(40, 80, 40)).bg(bg));
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

impl Game for SpaceInvaders {
    fn update(&mut self) {
        if self.game_over || self.paused { return; }
        self.tick += 1;
        self.update_bullets();
        self.update_aliens();
        self.check_collisions();
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
                    KeyCode::Left => {
                        self.player_x = (self.player_x - PLAYER_SPEED).max(3.0);
                    }
                    KeyCode::Right => {
                        self.player_x = (self.player_x + PLAYER_SPEED).min(self.field_width - 3.0);
                    }
                    KeyCode::Char(' ') | KeyCode::Up => {
                        if self.player_bullets.len() < MAX_PLAYER_BULLETS {
                            self.player_bullets.push(Bullet {
                                x: self.player_x,
                                y: self.player_y() - 2.0,
                                dy: -PLAYER_BULLET_SPEED,
                            });
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
            .border_style(Style::default().fg(Color::Rgb(80, 255, 80)))
            .title(" Space Invaders ")
            .title_style(Style::default().fg(Color::Rgb(100, 255, 100)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let new_fw = inner.width as f32;
        let new_fh = (inner.height.saturating_sub(2)) as f32;
        if (new_fw - self.field_width).abs() > 1.0 || (new_fh - self.field_height).abs() > 1.0 {
            let ratio_x = new_fw / self.field_width;
            let ratio_y = new_fh / self.field_height;
            self.player_x *= ratio_x;
            for a in &mut self.aliens { a.x *= ratio_x; a.y *= ratio_y; }
            for b in &mut self.player_bullets { b.x *= ratio_x; b.y *= ratio_y; }
            for b in &mut self.alien_bullets { b.x *= ratio_x; b.y *= ratio_y; }
            for s in &mut self.shields { s.x *= ratio_x; s.y *= ratio_y; }
            self.field_width = new_fw;
            self.field_height = new_fh;
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
        let lives_str = "\u{2666} ".repeat(self.lives as usize);
        let alive = self.aliens.iter().filter(|a| a.alive).count();
        let status = Line::from(vec![
            Span::styled(" \u{1f47e} ", Style::default()),
            Span::styled(
                format!("Score: {} ", self.score),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Lives: {}", lives_str),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("High: {} ", self.high_score),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Wave: {} ", self.level),
                Style::default().fg(Color::Green),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Aliens: {} ", alive),
                Style::default().fg(Color::Rgb(255, 80, 80)),
            ),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[0]);

        let fw = chunks[1].width as usize;
        let fh = chunks[1].height as usize;
        if fw > 0 && fh > 0 {
            let lines = self.render_field(fw, fh);
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }

        if self.game_over {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" GAME OVER! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("Press ENTER to restart, Esc for menu", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else if self.paused {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" PAUSED - Press P to resume ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            frame.render_widget(msg, chunks[2]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" \u{2190}\u{2192} Move ", Style::default().fg(Color::DarkGray)),
                Span::styled("| ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("Space Shoot ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("| ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("P Pause ", Style::default().fg(Color::DarkGray)),
                Span::styled("| ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("R Restart ", Style::default().fg(Color::DarkGray)),
                Span::styled("| ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("Esc Menu", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(help, chunks[2]);
        }
    }

    fn get_score(&self) -> u32 { self.score }
    fn is_game_over(&self) -> bool { self.game_over }

    fn reset(&mut self) {
        let hs = self.high_score;
        let fw = self.field_width;
        let fh = self.field_height;
        *self = SpaceInvaders::new();
        self.high_score = hs;
        self.field_width = fw;
        self.field_height = fh;
        self.player_x = fw / 2.0;
        self.init_aliens();
        self.init_shields();
    }
}
