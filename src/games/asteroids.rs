use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use std::collections::HashMap;

use crate::games::Game;

const MAX_BULLETS: usize = 8;
const BULLET_LIFETIME: u64 = 40;
const THRUST_ACCEL: f32 = 0.06;
const FRICTION: f32 = 0.99;
const ROTATION_SPEED: f32 = 0.12;
const BULLET_SPEED: f32 = 1.2;
const SHIP_INVULN_TICKS: u64 = 60;
const FIRE_COOLDOWN: u64 = 5;

#[derive(Clone, Copy, PartialEq)]
enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl AsteroidSize {
    fn radius(&self) -> f32 {
        match self {
            AsteroidSize::Large => 5.0,
            AsteroidSize::Medium => 3.0,
            AsteroidSize::Small => 1.5,
        }
    }

    fn num_verts(&self) -> usize {
        match self {
            AsteroidSize::Large => 11,
            AsteroidSize::Medium => 9,
            AsteroidSize::Small => 7,
        }
    }

    fn points(&self) -> u32 {
        match self {
            AsteroidSize::Large => 20,
            AsteroidSize::Medium => 50,
            AsteroidSize::Small => 100,
        }
    }

    fn split(&self) -> Option<AsteroidSize> {
        match self {
            AsteroidSize::Large => Some(AsteroidSize::Medium),
            AsteroidSize::Medium => Some(AsteroidSize::Small),
            AsteroidSize::Small => None,
        }
    }

    fn color(&self, seed: u8) -> Color {
        let v = seed % 3;
        match self {
            AsteroidSize::Large => match v {
                0 => Color::Rgb(170, 150, 120),
                1 => Color::Rgb(150, 140, 110),
                _ => Color::Rgb(160, 145, 115),
            },
            AsteroidSize::Medium => match v {
                0 => Color::Rgb(190, 170, 140),
                1 => Color::Rgb(180, 165, 135),
                _ => Color::Rgb(185, 168, 138),
            },
            AsteroidSize::Small => match v {
                0 => Color::Rgb(210, 190, 160),
                1 => Color::Rgb(200, 185, 155),
                _ => Color::Rgb(205, 188, 158),
            },
        }
    }
}

#[derive(Clone)]
struct Asteroid {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: AsteroidSize,
    shape_seed: u8,
}

#[derive(Clone)]
struct Bullet {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: u64,
}

pub struct Asteroids {
    ship_x: f32,
    ship_y: f32,
    ship_vx: f32,
    ship_vy: f32,
    ship_angle: f32,
    thrusting: bool,
    rotating_left: bool,
    rotating_right: bool,
    shooting: bool,
    invuln_timer: u64,
    fire_cooldown: u64,
    asteroids: Vec<Asteroid>,
    bullets: Vec<Bullet>,
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

impl Asteroids {
    pub fn new() -> Self {
        let fw = 80.0;
        let fh = 30.0;
        let mut a = Self {
            ship_x: fw / 2.0,
            ship_y: fh / 2.0,
            ship_vx: 0.0,
            ship_vy: 0.0,
            ship_angle: -std::f32::consts::FRAC_PI_2,
            thrusting: false,
            rotating_left: false,
            rotating_right: false,
            shooting: false,
            invuln_timer: SHIP_INVULN_TICKS,
            fire_cooldown: 0,
            asteroids: Vec::new(),
            bullets: Vec::new(),
            score: 0,
            high_score: 0,
            lives: 3,
            level: 1,
            game_over: false,
            paused: false,
            tick: 0,
            field_width: fw,
            field_height: fh,
            rng_state: 42,
        };
        a.spawn_asteroids(2);
        a
    }

    fn cheap_rand(&mut self) -> u32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state
    }

    fn rand_f32(&mut self) -> f32 {
        (self.cheap_rand() % 10000) as f32 / 10000.0
    }

    fn spawn_asteroids(&mut self, count: usize) {
        for _ in 0..count {
            let edge = self.cheap_rand() % 4;
            let (x, y) = match edge {
                0 => (self.rand_f32() * self.field_width, 0.0),
                1 => (self.rand_f32() * self.field_width, self.field_height),
                2 => (0.0, self.rand_f32() * self.field_height),
                _ => (self.field_width, self.rand_f32() * self.field_height),
            };
            let angle = self.rand_f32() * std::f32::consts::TAU;
            let base_speed = 0.1 + self.level as f32 * 0.02;
            let speed = base_speed + self.rand_f32() * 0.15;
            let seed = (self.cheap_rand() % 256) as u8;
            self.asteroids.push(Asteroid {
                x,
                y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                size: AsteroidSize::Large,
                shape_seed: seed,
            });
        }
    }

    fn wrap_coord(&self, x: f32, y: f32) -> (f32, f32) {
        let mut nx = x;
        let mut ny = y;
        if nx < 0.0 { nx += self.field_width; }
        if nx >= self.field_width { nx -= self.field_width; }
        if ny < 0.0 { ny += self.field_height; }
        if ny >= self.field_height { ny -= self.field_height; }
        (nx, ny)
    }

    fn update_ship(&mut self) {
        if self.rotating_left {
            self.ship_angle -= ROTATION_SPEED;
        }
        if self.rotating_right {
            self.ship_angle += ROTATION_SPEED;
        }

        if self.thrusting {
            self.ship_vx += self.ship_angle.cos() * THRUST_ACCEL;
            self.ship_vy += self.ship_angle.sin() * THRUST_ACCEL;
        }

        let speed = (self.ship_vx * self.ship_vx + self.ship_vy * self.ship_vy).sqrt();
        if speed > 1.5 {
            self.ship_vx = self.ship_vx / speed * 1.5;
            self.ship_vy = self.ship_vy / speed * 1.5;
        }

        self.ship_vx *= FRICTION;
        self.ship_vy *= FRICTION;

        self.ship_x += self.ship_vx;
        self.ship_y += self.ship_vy;

        let (nx, ny) = self.wrap_coord(self.ship_x, self.ship_y);
        self.ship_x = nx;
        self.ship_y = ny;

        if self.invuln_timer > 0 {
            self.invuln_timer -= 1;
        }

        if self.fire_cooldown > 0 {
            self.fire_cooldown -= 1;
        }
        if self.shooting && self.fire_cooldown == 0 && self.bullets.len() < MAX_BULLETS {
            self.bullets.push(Bullet {
                x: self.ship_x + self.ship_angle.cos() * 1.5,
                y: self.ship_y + self.ship_angle.sin() * 1.5,
                vx: self.ship_angle.cos() * BULLET_SPEED + self.ship_vx * 0.3,
                vy: self.ship_angle.sin() * BULLET_SPEED + self.ship_vy * 0.3,
                life: BULLET_LIFETIME,
            });
            self.fire_cooldown = FIRE_COOLDOWN;
        }
    }

    fn update_bullets(&mut self) {
        for bullet in &mut self.bullets {
            bullet.x += bullet.vx;
            bullet.y += bullet.vy;
            if bullet.x < 0.0 { bullet.x += self.field_width; }
            if bullet.x >= self.field_width { bullet.x -= self.field_width; }
            if bullet.y < 0.0 { bullet.y += self.field_height; }
            if bullet.y >= self.field_height { bullet.y -= self.field_height; }
            bullet.life = bullet.life.saturating_sub(1);
        }
        self.bullets.retain(|b| b.life > 0);
    }

    fn update_asteroids(&mut self) {
        let margin = 8.0;
        for asteroid in &mut self.asteroids {
            asteroid.x += asteroid.vx;
            asteroid.y += asteroid.vy;
            if asteroid.x < -margin { asteroid.x += self.field_width + margin * 2.0; }
            if asteroid.x >= self.field_width + margin { asteroid.x -= self.field_width + margin * 2.0; }
            if asteroid.y < -margin { asteroid.y += self.field_height + margin * 2.0; }
            if asteroid.y >= self.field_height + margin { asteroid.y -= self.field_height + margin * 2.0; }
        }
    }

    fn check_collisions(&mut self) {
        let mut hits: Vec<(usize, usize, f32, f32, AsteroidSize)> = Vec::new();
        let mut bullets_to_remove: Vec<usize> = Vec::new();
        let mut asteroids_to_remove: Vec<usize> = Vec::new();

        for (bi, bullet) in self.bullets.iter().enumerate() {
            for (ai, asteroid) in self.asteroids.iter().enumerate() {
                if asteroids_to_remove.contains(&ai) { continue; }
                let dx = bullet.x - asteroid.x;
                let dy = bullet.y - asteroid.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < asteroid.size.radius() {
                    self.score += asteroid.size.points();
                    bullets_to_remove.push(bi);
                    asteroids_to_remove.push(ai);
                    hits.push((bi, ai, asteroid.x, asteroid.y, asteroid.size));
                    break;
                }
            }
        }

        let mut new_asteroids: Vec<Asteroid> = Vec::new();
        for &(_, _, ax, ay, size) in &hits {
            if let Some(new_size) = size.split() {
                let spread_angle = self.rand_f32() * std::f32::consts::TAU;
                let speed = 0.2 + self.rand_f32() * 0.3 + self.level as f32 * 0.02;
                let seed1 = (self.cheap_rand() % 256) as u8;
                let seed2 = (self.cheap_rand() % 256) as u8;
                new_asteroids.push(Asteroid {
                    x: ax, y: ay,
                    vx: spread_angle.cos() * speed,
                    vy: spread_angle.sin() * speed,
                    size: new_size,
                    shape_seed: seed1,
                });
                new_asteroids.push(Asteroid {
                    x: ax, y: ay,
                    vx: -spread_angle.cos() * speed,
                    vy: -spread_angle.sin() * speed,
                    size: new_size,
                    shape_seed: seed2,
                });
            }
        }

        bullets_to_remove.sort_unstable();
        bullets_to_remove.dedup();
        for &i in bullets_to_remove.iter().rev() {
            if i < self.bullets.len() { self.bullets.remove(i); }
        }
        asteroids_to_remove.sort_unstable();
        asteroids_to_remove.dedup();
        for &i in asteroids_to_remove.iter().rev() {
            if i < self.asteroids.len() { self.asteroids.remove(i); }
        }
        self.asteroids.extend(new_asteroids);

        // Ship-asteroid collisions
        if self.invuln_timer == 0 {
            for asteroid in &self.asteroids {
                let dx = self.ship_x - asteroid.x;
                let dy = self.ship_y - asteroid.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < asteroid.size.radius() + 1.2 {
                    self.lives = self.lives.saturating_sub(1);
                    if self.lives == 0 {
                        self.game_over = true;
                        if self.score > self.high_score {
                            self.high_score = self.score;
                        }
                    } else {
                        self.ship_x = self.field_width / 2.0;
                        self.ship_y = self.field_height / 2.0;
                        self.ship_vx = 0.0;
                        self.ship_vy = 0.0;
                        self.invuln_timer = SHIP_INVULN_TICKS;
                    }
                    break;
                }
            }
        }

        // Level complete
        if self.asteroids.is_empty() && !self.game_over {
            self.level += 1;
            let count = (1 + self.level).min(8) as usize;
            self.spawn_asteroids(count);
            self.invuln_timer = SHIP_INVULN_TICKS;
        }
    }

    // ── Braille rendering helpers ──────────────────────────────────────

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

    fn line_cells(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let mut cells = Vec::new();
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx = x0;
        let mut cy = y0;
        loop {
            cells.push((cx, cy));
            if cx == x1 && cy == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; cx += sx; }
            if e2 <= dx { err += dx; cy += sy; }
        }
        cells
    }

    /// Generate irregular polygon vertices for an asteroid using its seed.
    fn asteroid_verts(cx: f32, cy: f32, size: AsteroidSize, seed: u8) -> Vec<(f32, f32)> {
        let n = size.num_verts();
        let r = size.radius();
        let mut verts = Vec::with_capacity(n);
        // Use seed to create per-vertex radius variation
        let mut s = seed as u32;
        for i in 0..n {
            let angle = (i as f32 / n as f32) * std::f32::consts::TAU;
            // Simple hash for variation per vertex
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            let variation = 0.7 + ((s >> 16) % 300) as f32 / 1000.0; // 0.7 - 1.0
            let vr = r * variation;
            verts.push((cx + angle.cos() * vr, cy + angle.sin() * vr));
        }
        verts
    }

    /// Set a braille dot in the map, with bounds checking.
    fn set_braille_dot(map: &mut HashMap<(usize, usize), u8>, bx: i32, by: i32, bw: i32, bh: i32) {
        if bx < 0 || by < 0 || bx >= bw || by >= bh { return; }
        let cx = bx as usize / 2;
        let cy = by as usize / 4;
        let sx = bx as usize % 2;
        let sy = by as usize % 4;
        *map.entry((cx, cy)).or_insert(0) |= Self::braille_bit(sx, sy);
    }

    /// Write a braille map layer onto the grid with a given color.
    fn write_braille_layer(
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
                // Merge with existing braille in this cell
                let existing = &grid[cy][cx];
                if existing.0 as u32 >= 0x2800 && (existing.0 as u32) < 0x2900 {
                    // Same-color merge: combine bits
                    let old_bits = existing.0 as u32 - 0x2800;
                    let merged = old_bits as u8 | bits;
                    let merged_ch = char::from_u32(0x2800 + merged as u32).unwrap_or(ch);
                    grid[cy][cx] = (merged_ch, style);
                } else {
                    grid[cy][cx] = (ch, style);
                }
            }
        }
    }

    /// Ship triangle: nose, left wing, notch, right wing.
    fn ship_points(&self) -> [(f32, f32); 4] {
        let a = self.ship_angle;
        let nose_len = 3.0;
        let wing_len = 2.2;
        let notch_len = 1.2;
        let wing_angle = 2.5;
        [
            (self.ship_x + a.cos() * nose_len,
             self.ship_y + a.sin() * nose_len),
            (self.ship_x + (a + std::f32::consts::PI - wing_angle / 2.0).cos() * wing_len,
             self.ship_y + (a + std::f32::consts::PI - wing_angle / 2.0).sin() * wing_len),
            (self.ship_x + (a + std::f32::consts::PI).cos() * notch_len,
             self.ship_y + (a + std::f32::consts::PI).sin() * notch_len),
            (self.ship_x + (a + std::f32::consts::PI + wing_angle / 2.0).cos() * wing_len,
             self.ship_y + (a + std::f32::consts::PI + wing_angle / 2.0).sin() * wing_len),
        ]
    }

    // ── Main render ────────────────────────────────────────────────────

    fn render_field(&self, width: usize, height: usize) -> Vec<Line<'static>> {
        let w = width;
        let h = height;
        let bw = (w * 2) as i32;
        let bh = (h * 4) as i32;
        let bsx = bw as f32 / self.field_width;
        let bsy = bh as f32 / self.field_height;

        let bg = Color::Rgb(5, 5, 15);
        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(bg)); w]; h];

        // Sparse background stars (regular chars, not braille)
        for yi in 0..h {
            for xi in 0..w {
                let hash = ((xi * 7 + yi * 13 + 37) * 31) % 250;
                if hash < 2 {
                    let b = 35 + (hash as u8) * 15;
                    grid[yi][xi] = ('.', Style::default().fg(Color::Rgb(b, b, b + 8)).bg(bg));
                }
            }
        }

        // ── Asteroids (braille polygons) ───────────────────────────────
        for asteroid in &self.asteroids {
            let verts = Self::asteroid_verts(asteroid.x, asteroid.y, asteroid.size, asteroid.shape_seed);
            let color = asteroid.size.color(asteroid.shape_seed);
            let mut amap: HashMap<(usize, usize), u8> = HashMap::new();

            // Draw polygon outline
            let n = verts.len();
            for i in 0..n {
                let (x0, y0) = verts[i];
                let (x1, y1) = verts[(i + 1) % n];
                let bx0 = (x0 * bsx) as i32;
                let by0 = (y0 * bsy) as i32;
                let bx1 = (x1 * bsx) as i32;
                let by1 = (y1 * bsy) as i32;
                for (px, py) in Self::line_cells(bx0, by0, bx1, by1) {
                    Self::set_braille_dot(&mut amap, px, py, bw, bh);
                }
            }

            Self::write_braille_layer(&mut grid, &amap, w, h, color, bg, false);
        }

        // ── Bullets (braille dots with short trail) ────────────────────
        for bullet in &self.bullets {
            let mut bmap: HashMap<(usize, usize), u8> = HashMap::new();
            let brightness = if bullet.life > BULLET_LIFETIME / 2 { 255 } else { 180 };
            let color = Color::Rgb(brightness, brightness, 80);

            // Head dot (2x2 braille pixels for visibility)
            let bx = (bullet.x * bsx) as i32;
            let by = (bullet.y * bsy) as i32;
            for &(dx, dy) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
                Self::set_braille_dot(&mut bmap, bx + dx, by + dy, bw, bh);
            }

            // Trail dot
            let tx = ((bullet.x - bullet.vx * 1.5) * bsx) as i32;
            let ty = ((bullet.y - bullet.vy * 1.5) * bsy) as i32;
            Self::set_braille_dot(&mut bmap, tx, ty, bw, bh);

            Self::write_braille_layer(&mut grid, &bmap, w, h, color, bg, true);
        }

        // ── Ship (braille triangle) ────────────────────────────────────
        if !self.game_over {
            let visible = self.invuln_timer == 0 || (self.tick % 4) < 2;
            if visible {
                let pts = self.ship_points();
                let ship_color = if self.thrusting {
                    Color::Rgb(100, 230, 255)
                } else {
                    Color::Rgb(80, 255, 140)
                };

                let mut smap: HashMap<(usize, usize), u8> = HashMap::new();
                let to_bp = |fx: f32, fy: f32| -> (i32, i32) {
                    ((fx * bsx) as i32, (fy * bsy) as i32)
                };
                let bp: Vec<(i32, i32)> = pts.iter().map(|&(x, y)| to_bp(x, y)).collect();

                // Outline: nose->left->notch->right->nose
                let edges = [(0,1), (1,2), (2,3), (3,0)];
                for &(a, b) in &edges {
                    for (px, py) in Self::line_cells(bp[a].0, bp[a].1, bp[b].0, bp[b].1) {
                        Self::set_braille_dot(&mut smap, px, py, bw, bh);
                    }
                }

                Self::write_braille_layer(&mut grid, &smap, w, h, ship_color, bg, true);

                // Thrust flame
                if self.thrusting {
                    let mut fmap: HashMap<(usize, usize), u8> = HashMap::new();
                    let fa = self.ship_angle + std::f32::consts::PI;
                    for i in 0..10 {
                        let dist = 2.0 + i as f32 * 0.5;
                        let spread = (i as f32 * 0.18) * if i % 2 == 0 { 1.0 } else { -1.0 };
                        let fx = self.ship_x + (fa + spread).cos() * dist;
                        let fy = self.ship_y + (fa + spread).sin() * dist;
                        let fbx = (fx * bsx) as i32;
                        let fby = (fy * bsy) as i32;
                        Self::set_braille_dot(&mut fmap, fbx, fby, bw, bh);
                        // Extra width dot
                        let perp = fa + std::f32::consts::FRAC_PI_2;
                        let px2 = fx + perp.cos() * 0.3;
                        let py2 = fy + perp.sin() * 0.3;
                        Self::set_braille_dot(&mut fmap, (px2 * bsx) as i32, (py2 * bsy) as i32, bw, bh);
                    }
                    let flicker = if self.tick % 3 == 0 {
                        Color::Rgb(255, 200, 60)
                    } else {
                        Color::Rgb(255, 130, 30)
                    };
                    // Don't overwrite ship cells
                    for key in smap.keys() {
                        fmap.remove(key);
                    }
                    Self::write_braille_layer(&mut grid, &fmap, w, h, flicker, bg, false);
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

impl Game for Asteroids {
    fn update(&mut self) {
        if self.game_over || self.paused {
            self.thrusting = false;
            self.rotating_left = false;
            self.rotating_right = false;
            self.shooting = false;
            return;
        }
        self.tick += 1;
        self.update_ship();
        self.update_bullets();
        self.update_asteroids();
        self.check_collisions();
        self.thrusting = false;
        self.rotating_left = false;
        self.rotating_right = false;
        self.shooting = false;
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
                    KeyCode::Left => self.rotating_left = true,
                    KeyCode::Right => self.rotating_right = true,
                    KeyCode::Up => self.thrusting = true,
                    KeyCode::Char(' ') => self.shooting = true,
                    _ => {}
                }
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(100, 200, 255)))
            .title(" Asteroids ")
            .title_style(Style::default().fg(Color::Rgb(130, 220, 255)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let new_fw = inner.width as f32;
        let new_fh = (inner.height.saturating_sub(2)) as f32;
        if (new_fw - self.field_width).abs() > 1.0 || (new_fh - self.field_height).abs() > 1.0 {
            let ratio_x = new_fw / self.field_width;
            let ratio_y = new_fh / self.field_height;
            self.ship_x *= ratio_x;
            self.ship_y *= ratio_y;
            for a in &mut self.asteroids {
                a.x *= ratio_x;
                a.y *= ratio_y;
            }
            for b in &mut self.bullets {
                b.x *= ratio_x;
                b.y *= ratio_y;
            }
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
        let status = Line::from(vec![
            Span::styled(" \u{2604} ", Style::default()),
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
                format!("Level: {} ", self.level),
                Style::default().fg(Color::Green),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Rocks: {} ", self.asteroids.len()),
                Style::default().fg(Color::Rgb(160, 140, 120)),
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

        // Help bar
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
                Span::styled(" \u{2190}\u{2192} Rotate ", Style::default().fg(Color::DarkGray)),
                Span::styled("| ", Style::default().fg(Color::Rgb(60, 60, 60))),
                Span::styled("\u{2191} Thrust ", Style::default().fg(Color::DarkGray)),
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
        *self = Asteroids::new();
        self.high_score = hs;
        self.field_width = fw;
        self.field_height = fh;
        self.ship_x = fw / 2.0;
        self.ship_y = fh / 2.0;
    }
}
