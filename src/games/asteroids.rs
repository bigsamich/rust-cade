use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

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
            AsteroidSize::Large => 3.0,
            AsteroidSize::Medium => 1.8,
            AsteroidSize::Small => 0.9,
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
        a.spawn_asteroids(4);
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
            let speed = 0.15 + self.rand_f32() * 0.25;
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

        // Clamp max speed
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

        // Shooting
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
            // Wrap bullets
            if bullet.x < 0.0 { bullet.x += self.field_width; }
            if bullet.x >= self.field_width { bullet.x -= self.field_width; }
            if bullet.y < 0.0 { bullet.y += self.field_height; }
            if bullet.y >= self.field_height { bullet.y -= self.field_height; }
            bullet.life = bullet.life.saturating_sub(1);
        }
        self.bullets.retain(|b| b.life > 0);
    }

    fn update_asteroids(&mut self) {
        for asteroid in &mut self.asteroids {
            asteroid.x += asteroid.vx;
            asteroid.y += asteroid.vy;
            if asteroid.x < -4.0 { asteroid.x += self.field_width + 8.0; }
            if asteroid.x >= self.field_width + 4.0 { asteroid.x -= self.field_width + 8.0; }
            if asteroid.y < -4.0 { asteroid.y += self.field_height + 8.0; }
            if asteroid.y >= self.field_height + 4.0 { asteroid.y -= self.field_height + 8.0; }
        }
    }

    fn check_collisions(&mut self) {
        // Bullet-asteroid collisions — collect hits first, then mutate
        let mut hits: Vec<(usize, usize, f32, f32, AsteroidSize)> = Vec::new(); // (bullet_idx, asteroid_idx, ax, ay, size)
        let mut bullets_to_remove: Vec<usize> = Vec::new();
        let mut asteroids_to_remove: Vec<usize> = Vec::new();

        for (bi, bullet) in self.bullets.iter().enumerate() {
            for (ai, asteroid) in self.asteroids.iter().enumerate() {
                if asteroids_to_remove.contains(&ai) { continue; }
                let dx = bullet.x - asteroid.x;
                let dy = bullet.y - asteroid.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < asteroid.size.radius() + 0.5 {
                    self.score += asteroid.size.points();
                    bullets_to_remove.push(bi);
                    asteroids_to_remove.push(ai);
                    hits.push((bi, ai, asteroid.x, asteroid.y, asteroid.size));
                    break;
                }
            }
        }

        // Now spawn split asteroids using mutable self for RNG
        let mut new_asteroids: Vec<Asteroid> = Vec::new();
        for &(_, _, ax, ay, size) in &hits {
            if let Some(new_size) = size.split() {
                let spread_angle = self.rand_f32() * std::f32::consts::TAU;
                let speed = 0.2 + self.rand_f32() * 0.3;
                let seed1 = (self.cheap_rand() % 256) as u8;
                let seed2 = (self.cheap_rand() % 256) as u8;
                new_asteroids.push(Asteroid {
                    x: ax,
                    y: ay,
                    vx: spread_angle.cos() * speed,
                    vy: spread_angle.sin() * speed,
                    size: new_size,
                    shape_seed: seed1,
                });
                new_asteroids.push(Asteroid {
                    x: ax,
                    y: ay,
                    vx: -spread_angle.cos() * speed,
                    vy: -spread_angle.sin() * speed,
                    size: new_size,
                    shape_seed: seed2,
                });
            }
        }

        // Remove hit bullets and asteroids (reverse order to maintain indices)
        bullets_to_remove.sort_unstable();
        bullets_to_remove.dedup();
        for &i in bullets_to_remove.iter().rev() {
            if i < self.bullets.len() {
                self.bullets.remove(i);
            }
        }
        asteroids_to_remove.sort_unstable();
        asteroids_to_remove.dedup();
        for &i in asteroids_to_remove.iter().rev() {
            if i < self.asteroids.len() {
                self.asteroids.remove(i);
            }
        }
        self.asteroids.extend(new_asteroids);

        // Ship-asteroid collisions (only if not invulnerable)
        if self.invuln_timer == 0 {
            for asteroid in &self.asteroids {
                let dx = self.ship_x - asteroid.x;
                let dy = self.ship_y - asteroid.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < asteroid.size.radius() + 1.0 {
                    self.lives = self.lives.saturating_sub(1);
                    if self.lives == 0 {
                        self.game_over = true;
                        if self.score > self.high_score {
                            self.high_score = self.score;
                        }
                    } else {
                        // Respawn in center
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

        // Check level complete
        if self.asteroids.is_empty() && !self.game_over {
            self.level += 1;
            let count = (3 + self.level).min(12) as usize;
            self.spawn_asteroids(count);
            self.invuln_timer = SHIP_INVULN_TICKS;
        }
    }

    /// Return the ship triangle points (nose, left wing, right wing) in field coords.
    fn ship_points(&self) -> [(f32, f32); 5] {
        let a = self.ship_angle;
        let nose_len = 3.0;
        let wing_len = 2.2;
        let notch_len = 1.2;
        let wing_angle = 2.5;
        let nose = (
            self.ship_x + a.cos() * nose_len,
            self.ship_y + a.sin() * nose_len,
        );
        let left = (
            self.ship_x + (a + std::f32::consts::PI - wing_angle / 2.0).cos() * wing_len,
            self.ship_y + (a + std::f32::consts::PI - wing_angle / 2.0).sin() * wing_len,
        );
        let right = (
            self.ship_x + (a + std::f32::consts::PI + wing_angle / 2.0).cos() * wing_len,
            self.ship_y + (a + std::f32::consts::PI + wing_angle / 2.0).sin() * wing_len,
        );
        // Rear notch (indent behind center for classic asteroids look)
        let notch = (
            self.ship_x + (a + std::f32::consts::PI).cos() * notch_len,
            self.ship_y + (a + std::f32::consts::PI).sin() * notch_len,
        );
        let center = (self.ship_x, self.ship_y);
        [nose, left, notch, right, center]
    }

    /// Bresenham's line rasterization.
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
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }
        cells
    }

    /// Convert a braille sub-pixel position to its bit mask.
    /// Each terminal cell is a 2-wide x 4-tall braille grid.
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

    /// Render the ship into the grid using braille characters for smooth sub-cell resolution.
    fn render_ship_braille(
        &self,
        grid: &mut [Vec<(char, Style)>],
        w: usize,
        h: usize,
        bg: Color,
    ) {
        if self.game_over { return; }
        let visible = self.invuln_timer == 0 || (self.tick % 4) < 2;
        if !visible { return; }

        // Braille pixel resolution: 2x horizontal, 4x vertical per terminal cell
        let bw = (w * 2) as i32;
        let bh = (h * 4) as i32;
        let bsx = bw as f32 / self.field_width;
        let bsy = bh as f32 / self.field_height;

        let [nose, left, notch, right, _center] = self.ship_points();

        let to_bp = |fx: f32, fy: f32| -> (i32, i32) {
            ((fx * bsx) as i32, (fy * bsy) as i32)
        };
        let (nx, ny) = to_bp(nose.0, nose.1);
        let (lx, ly) = to_bp(left.0, left.1);
        let (rx, ry) = to_bp(right.0, right.1);
        let (mx, my) = to_bp(notch.0, notch.1);

        // Accumulate braille bits per terminal cell
        let mut braille: std::collections::HashMap<(usize, usize), u8> = std::collections::HashMap::new();

        let mut set_dot = |bx: i32, by: i32| {
            if bx < 0 || by < 0 || bx >= bw || by >= bh { return; }
            let cx = bx as usize / 2;
            let cy = by as usize / 4;
            let sx = bx as usize % 2;
            let sy = by as usize % 4;
            *braille.entry((cx, cy)).or_insert(0) |= Self::braille_bit(sx, sy);
        };

        // Draw ship outline: nose -> left -> notch -> right -> nose
        for &(x0, y0, x1, y1) in &[
            (nx, ny, lx, ly),
            (lx, ly, mx, my),
            (mx, my, rx, ry),
            (rx, ry, nx, ny),
        ] {
            for (px, py) in Self::line_cells(x0, y0, x1, y1) {
                set_dot(px, py);
            }
        }

        let ship_color = if self.thrusting {
            Color::Rgb(100, 230, 255)
        } else {
            Color::Rgb(80, 255, 140)
        };

        // Write braille chars to grid
        for (&(cx, cy), &bits) in &braille {
            if cx < w && cy < h && bits != 0 {
                let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                grid[cy][cx] = (ch, Style::default().fg(ship_color).bg(bg).add_modifier(Modifier::BOLD));
            }
        }

        // Thrust exhaust — also in braille
        if self.thrusting {
            let mut flame_braille: std::collections::HashMap<(usize, usize), u8> = std::collections::HashMap::new();
            let a = self.ship_angle + std::f32::consts::PI;
            for i in 0..8 {
                let dist = 2.0 + i as f32 * 0.5;
                let spread = (i as f32 * 0.15) * if i % 2 == 0 { 1.0 } else { -1.0 };
                let fx = self.ship_x + (a + spread).cos() * dist;
                let fy = self.ship_y + (a + spread).sin() * dist;
                let bx = (fx * bsx) as i32;
                let by = (fy * bsy) as i32;
                if bx >= 0 && by >= 0 && bx < bw && by < bh {
                    let cx = bx as usize / 2;
                    let cy = by as usize / 4;
                    let sx = bx as usize % 2;
                    let sy = by as usize % 4;
                    *flame_braille.entry((cx, cy)).or_insert(0) |= Self::braille_bit(sx, sy);
                }
            }
            for (&(cx, cy), &bits) in &flame_braille {
                if cx < w && cy < h && bits != 0 {
                    // Don't overwrite ship cells
                    if braille.contains_key(&(cx, cy)) { continue; }
                    let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                    let flicker = if self.tick % 3 == 0 {
                        Color::Rgb(255, 200, 60)
                    } else {
                        Color::Rgb(255, 130, 30)
                    };
                    grid[cy][cx] = (ch, Style::default().fg(flicker).bg(bg));
                }
            }
        }
    }

    fn asteroid_char(size: AsteroidSize) -> char {
        match size {
            AsteroidSize::Large => '@',
            AsteroidSize::Medium => 'O',
            AsteroidSize::Small => 'o',
        }
    }

    fn asteroid_color(size: AsteroidSize, seed: u8) -> Color {
        let variant = seed % 3;
        match size {
            AsteroidSize::Large => match variant {
                0 => Color::Rgb(160, 140, 120),
                1 => Color::Rgb(140, 130, 110),
                _ => Color::Rgb(150, 135, 115),
            },
            AsteroidSize::Medium => match variant {
                0 => Color::Rgb(180, 160, 140),
                1 => Color::Rgb(170, 155, 130),
                _ => Color::Rgb(175, 158, 135),
            },
            AsteroidSize::Small => match variant {
                0 => Color::Rgb(200, 180, 160),
                1 => Color::Rgb(190, 175, 155),
                _ => Color::Rgb(195, 178, 158),
            },
        }
    }

    fn render_field(&self, width: usize, height: usize) -> Vec<Line<'static>> {
        let w = width;
        let h = height;

        let sx = w as f32 / self.field_width;
        let sy = h as f32 / self.field_height;

        let bg = Color::Rgb(5, 5, 15);
        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(bg)); w]; h];

        // Sprinkle some stars in background based on position
        for sy_i in 0..h {
            for sx_i in 0..w {
                let hash = ((sx_i * 7 + sy_i * 13 + 37) * 31) % 200;
                if hash < 2 {
                    let brightness = 40 + (hash as u8) * 20;
                    grid[sy_i][sx_i] = ('.', Style::default()
                        .fg(Color::Rgb(brightness, brightness, brightness + 10))
                        .bg(bg));
                } else if hash == 3 {
                    grid[sy_i][sx_i] = ('+', Style::default()
                        .fg(Color::Rgb(30, 30, 40))
                        .bg(bg));
                }
            }
        }

        // Draw asteroids
        for asteroid in &self.asteroids {
            let ax = (asteroid.x * sx) as usize;
            let ay = (asteroid.y * sy) as usize;
            let r = (asteroid.size.radius() * sx) as usize;
            let ch = Self::asteroid_char(asteroid.size);
            let color = Self::asteroid_color(asteroid.size, asteroid.shape_seed);

            match asteroid.size {
                AsteroidSize::Large => {
                    // Draw a larger shape
                    for dy in 0..=r.min(3) {
                        for dx in 0..=r.min(4) {
                            let gx = ax.wrapping_add(dx).wrapping_sub(r / 2);
                            let gy = ay.wrapping_add(dy).wrapping_sub(r / 2);
                            if gx < w && gy < h {
                                // Make it roughly circular
                                let ddx = dx as f32 - r as f32 / 2.0;
                                let ddy = dy as f32 - r as f32 / 2.0;
                                if (ddx * ddx + ddy * ddy).sqrt() <= r as f32 * 0.8 {
                                    let edge = (ddx * ddx + ddy * ddy).sqrt() > r as f32 * 0.5;
                                    let c = if edge { '.' } else { ch };
                                    grid[gy][gx] = (c, Style::default().fg(color).bg(bg));
                                }
                            }
                        }
                    }
                    // Always draw center
                    if ax < w && ay < h {
                        grid[ay][ax] = (ch, Style::default().fg(color).bg(bg).add_modifier(Modifier::BOLD));
                    }
                }
                AsteroidSize::Medium => {
                    for &(dx, dy) in &[(0i32, 0i32), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let gx = (ax as i32 + dx) as usize;
                        let gy = (ay as i32 + dy) as usize;
                        if gx < w && gy < h {
                            let c = if dx == 0 && dy == 0 { ch } else { '.' };
                            grid[gy][gx] = (c, Style::default().fg(color).bg(bg));
                        }
                    }
                    if ax < w && ay < h {
                        grid[ay][ax] = (ch, Style::default().fg(color).bg(bg).add_modifier(Modifier::BOLD));
                    }
                }
                AsteroidSize::Small => {
                    if ax < w && ay < h {
                        grid[ay][ax] = (ch, Style::default().fg(color).bg(bg).add_modifier(Modifier::BOLD));
                    }
                }
            }
        }

        // Draw bullets
        for bullet in &self.bullets {
            let bx = (bullet.x * sx) as usize;
            let by = (bullet.y * sy) as usize;
            if bx < w && by < h {
                let brightness = if bullet.life > BULLET_LIFETIME / 2 { 255 } else { 180 };
                grid[by][bx] = ('*', Style::default()
                    .fg(Color::Rgb(brightness, brightness, 100))
                    .bg(bg)
                    .add_modifier(Modifier::BOLD));
            }
        }

        // Draw ship using braille characters for smooth rotation
        self.render_ship_braille(&mut grid, w, h, bg);

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
        // Clear held states after processing (TUI has no key-up events)
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
        // Clear held-down states each frame since TUI gets key press events, not hold
        // We set them true on press and clear after one update cycle
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(100, 200, 255)))
            .title(" Asteroids ")
            .title_style(Style::default().fg(Color::Rgb(130, 220, 255)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Update dimensions dynamically
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
