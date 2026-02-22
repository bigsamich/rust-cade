use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::games::Game;

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum BumpAxis {
    X,
    Y,
}

#[allow(dead_code)]
impl BumpAxis {
    fn label(&self) -> &str {
        match self {
            BumpAxis::X => "X",
            BumpAxis::Y => "Y",
        }
    }

    fn color(&self) -> Color {
        match self {
            BumpAxis::X => Color::Rgb(255, 180, 120),
            BumpAxis::Y => Color::Rgb(200, 120, 255),
        }
    }

    fn toggle(&self) -> BumpAxis {
        match self {
            BumpAxis::X => BumpAxis::Y,
            BumpAxis::Y => BumpAxis::X,
        }
    }
}

#[derive(Clone)]
struct BumpConfig {
    size: usize,            // 3, 4, or 5
    start_section: usize,   // first section of the bump
    axis: BumpAxis,         // which axis is currently being adjusted
}

impl BumpConfig {
    fn new(size: usize, start_section: usize) -> Self {
        Self {
            size,
            start_section,
            axis: BumpAxis::X,
        }
    }

    /// Get the sign-pattern coefficients for a closed orbit bump.
    /// These sum to zero so the net angle kick cancels out.
    fn coefficients(&self) -> Vec<f32> {
        match self.size {
            3 => vec![1.0, -2.0, 1.0],
            4 => vec![1.0, -1.0, -1.0, 1.0],
            5 => vec![1.0, -2.0, 2.0, -2.0, 1.0],
            _ => vec![],
        }
    }

    /// Return the list of (section_index, coefficient) pairs for this bump
    fn section_coefficients(&self) -> Vec<(usize, f32)> {
        self.coefficients()
            .iter()
            .enumerate()
            .map(|(i, &c)| ((self.start_section + i) % NUM_SECTIONS, c))
            .collect()
    }

    /// Get the magnet index for trim in a given section based on current axis
    #[allow(dead_code)]
    fn trim_index_in_section(&self, section: usize) -> usize {
        match self.axis {
            BumpAxis::X => section * MAGNETS_PER_SECTION + 5, // HTrim
            BumpAxis::Y => section * MAGNETS_PER_SECTION + 4, // VTrim
        }
    }

    /// Check if a section is part of this bump
    fn contains_section(&self, sec: usize) -> bool {
        for i in 0..self.size {
            if (self.start_section + i) % NUM_SECTIONS == sec {
                return true;
            }
        }
        false
    }

    /// Get the coefficient for a given section (if it's part of the bump)
    fn coeff_for_section(&self, sec: usize) -> Option<f32> {
        let coeffs = self.coefficients();
        for i in 0..self.size {
            if (self.start_section + i) % NUM_SECTIONS == sec {
                return Some(coeffs[i]);
            }
        }
        None
    }
}

#[derive(Clone)]
struct Restriction {
    section: usize,       // which section (0-based)
    axis: char,           // 'x' or 'y'
    positive_blocked: bool, // true = blocks positive side (val > 0), false = blocks negative (val < 0)
}

impl Restriction {
    fn label(&self) -> String {
        let sign = if self.positive_blocked { "≤0" } else { "≥0" };
        format!("{}{}",self.axis, sign)
    }

    fn check(&self, x: f32, y: f32) -> bool {
        let val = if self.axis == 'x' { x } else { y };
        if self.positive_blocked { val > 0.0 } else { val < 0.0 }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Difficulty {
    Easy,
    Hard,
}

impl Difficulty {
    fn label(&self) -> &str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Hard => "Hard",
        }
    }

    fn color(&self) -> Color {
        match self {
            Difficulty::Easy => Color::Rgb(80, 255, 80),
            Difficulty::Hard => Color::Rgb(255, 60, 60),
        }
    }

    fn toggle(&self) -> Difficulty {
        match self {
            Difficulty::Easy => Difficulty::Hard,
            Difficulty::Hard => Difficulty::Easy,
        }
    }

    /// Constant beam size growth rate per element step (simulates phase instability)
    fn size_growth_rate(&self) -> f32 {
        match self {
            Difficulty::Easy => 0.0,
            Difficulty::Hard => 0.05,
        }
    }
}

const NUM_SECTIONS: usize = 24;
const APERTURE: f32 = 50.0; // max beam size before instant loss (hard wall)
const LOSS_ZONE: f32 = 25.0; // beam edges past this start accumulating losses
const MAX_LOSSES: f32 = 100.0; // game over when losses reach this
const MAGNETS_PER_SECTION: usize = 6;
const TOTAL_MAGNETS: usize = NUM_SECTIONS * MAGNETS_PER_SECTION;
// Design dipole strength: each section needs 15° of bend (360/24), split across 2 dipoles
const DESIGN_DIPOLE_POWER: f32 = 0.1309; // ~7.5° in radians per dipole
const GOAL_TURNS: u32 = 5;
const MAX_HISTORY: usize = 60;
const NUM_RAMPS: usize = 10;
const MAX_RAMP_DELTA: f32 = 0.5;

#[derive(Clone, Copy, PartialEq)]
enum MagnetType {
    FocusQuad,  // QF - focuses horizontally
    Dipole1,    // D1 - bends beam
    DefocusQuad,// QD - defocuses horizontally
    Dipole2,    // D2 - bends beam
    VTrim,      // VT - vertical trim dipole
    HTrim,      // HT - horizontal trim dipole
}

impl MagnetType {
    fn label(&self) -> &str {
        match self {
            MagnetType::FocusQuad => "QF",
            MagnetType::Dipole1 => "D1",
            MagnetType::DefocusQuad => "QD",
            MagnetType::Dipole2 => "D2",
            MagnetType::VTrim => "VT",
            MagnetType::HTrim => "HT",
        }
    }

    fn color(&self) -> Color {
        match self {
            MagnetType::FocusQuad => Color::Rgb(80, 180, 255),
            MagnetType::Dipole1 => Color::Rgb(255, 120, 80),
            MagnetType::DefocusQuad => Color::Rgb(80, 255, 140),
            MagnetType::Dipole2 => Color::Rgb(255, 200, 80),
            MagnetType::VTrim => Color::Rgb(200, 120, 255),
            MagnetType::HTrim => Color::Rgb(255, 180, 120),
        }
    }
}

#[derive(Clone)]
struct Magnet {
    mag_type: MagnetType,
    power: f32,       // current supply value
    _section: usize,  // which section (0-23)
}

pub struct BeamGame {
    magnets: Vec<Magnet>,
    selected: usize,           // currently selected magnet index
    beam_running: bool,
    beam_position: f32,        // horizontal position (should stay near 0)
    beam_angle: f32,           // horizontal angle
    beam_size: f32,            // beam envelope size
    beam_y_position: f32,      // vertical position (should stay near 0)
    beam_y_angle: f32,         // vertical angle
    beam_y_size: f32,          // vertical beam envelope size
    beam_section: usize,       // current section the beam is in
    beam_element: usize,       // current element within section (0-3)
    beam_progress: f32,        // fractional progress through current element
    beam_lost: bool,
    beam_losses: f32,         // accumulated losses from beam in loss zone
    beam_completed: bool,
    turns_completed: u32,
    best_turns: u32,
    tick: u64,
    paused: bool,
    // Track beam trail for display
    trail: Vec<(usize, f32, f32)>, // (section, position, size) at each section boundary
    adjust_speed: f32,
    // Position history for sparkline
    pos_history: Vec<f32>,
    size_history: Vec<f32>,
    y_pos_history: Vec<f32>,
    y_size_history: Vec<f32>,
    // Aperture restrictions
    restrictions: Vec<Restriction>,
    // Difficulty
    difficulty: Difficulty,
    // Message flash
    message: Option<(String, u32, Color)>, // (text, ticks_remaining, color)
    // Bump mode: closed orbit bump using N consecutive trim magnets
    bump: Option<BumpConfig>,
    // Power supply ramp: 10 settings per magnet, one per turn (keys 0-9)
    ramp_powers: Vec<[f32; 10]>,  // Per-magnet power at each ramp point
    selected_ramp: usize,          // Which ramp point is being edited (0-9)
    // Closed orbit target: beam should return to this point each turn
    target_x: f32,
    target_y: f32,
    turn_positions: Vec<(f32, f32)>, // beam (x, y) at end of each turn
}

impl BeamGame {
    pub fn new() -> Self {
        let mut magnets = Vec::new();
        for sec in 0..NUM_SECTIONS {
            // All magnets start at 0 - player must:
            // 1) Set dipoles to ~0.131 (DESIGN_DIPOLE_POWER) to bend beam around the ring
            // 2) Tune quads for focusing stability
            // 3) Use trims for fine orbit corrections
            // With all magnets at 0 the beam goes straight and immediately hits the wall!
            magnets.push(Magnet { mag_type: MagnetType::FocusQuad, power: 0.0, _section: sec });
            magnets.push(Magnet { mag_type: MagnetType::Dipole1, power: 0.0, _section: sec });
            magnets.push(Magnet { mag_type: MagnetType::DefocusQuad, power: 0.0, _section: sec });
            magnets.push(Magnet { mag_type: MagnetType::Dipole2, power: 0.0, _section: sec });
            magnets.push(Magnet { mag_type: MagnetType::VTrim, power: 0.0, _section: sec });
            magnets.push(Magnet { mag_type: MagnetType::HTrim, power: 0.0, _section: sec });
        }

        // Generate 4 random restrictions: 2 horizontal, 2 vertical on distinct sections
        let mut rng = rand::thread_rng();
        let mut restriction_sections: Vec<usize> = Vec::new();
        while restriction_sections.len() < 4 {
            let s = rng.gen_range(0..NUM_SECTIONS);
            if !restriction_sections.contains(&s) {
                restriction_sections.push(s);
            }
        }
        let restrictions = vec![
            Restriction { section: restriction_sections[0], axis: 'x', positive_blocked: rng.gen_bool(0.5) },
            Restriction { section: restriction_sections[1], axis: 'x', positive_blocked: rng.gen_bool(0.5) },
            Restriction { section: restriction_sections[2], axis: 'y', positive_blocked: rng.gen_bool(0.5) },
            Restriction { section: restriction_sections[3], axis: 'y', positive_blocked: rng.gen_bool(0.5) },
        ];

        // Initialize ramp powers: all 10 ramp points start with same initial power
        let ramp_powers: Vec<[f32; 10]> = magnets.iter()
            .map(|m| [m.power; 10])
            .collect();

        Self {
            magnets,
            selected: 0,
            beam_running: false,
            beam_position: 0.0,
            beam_angle: 0.0,
            beam_size: 10.0,
            beam_y_position: 0.0,
            beam_y_angle: 0.0,
            beam_y_size: 10.0,
            beam_section: 0,
            beam_element: 0,
            beam_progress: 0.0,
            beam_lost: false,
            beam_losses: 0.0,
            beam_completed: false,
            turns_completed: 0,
            best_turns: 0,
            tick: 0,
            paused: false,
            trail: Vec::new(),
            adjust_speed: 0.01,
            pos_history: Vec::new(),
            size_history: Vec::new(),
            y_pos_history: Vec::new(),
            y_size_history: Vec::new(),
            restrictions,
            difficulty: Difficulty::Easy,
            message: None,
            bump: None,
            ramp_powers,
            selected_ramp: 0,
            target_x: rng.gen_range(-5.0..5.0),
            target_y: rng.gen_range(-5.0..5.0),
            turn_positions: Vec::new(),
        }
    }

    fn apply_element(&mut self) {
        let mag_idx = self.beam_section * MAGNETS_PER_SECTION + self.beam_element;
        if mag_idx >= self.magnets.len() { return; }
        let magnet = &self.magnets[mag_idx];

        match magnet.mag_type {
            MagnetType::FocusQuad => {
                // Thin lens focusing in X: x' -= k*x, size decreases
                let k = magnet.power;
                self.beam_angle -= k * self.beam_position;
                self.beam_size = (self.beam_size * (1.0 - k.abs() * 0.5)).max(1.0);
                // Opposite in Y: defocusing
                self.beam_y_angle += k * self.beam_y_position;
                self.beam_y_size = (self.beam_y_size * (1.0 + k.abs() * 0.3)).min(APERTURE * 2.0);
            }
            MagnetType::Dipole1 | MagnetType::Dipole2 => {
                // Dipole must supply the design bend to keep beam on circular orbit.
                // The beam "wants" to go straight; only the dipole bending force curves it.
                // Error from design = how much the orbit deviates from the ideal circle.
                let bend_error = magnet.power - DESIGN_DIPOLE_POWER;
                self.beam_angle += bend_error;
                // Drift effect: position changes with angle
                self.beam_position += self.beam_angle * 2.0;
                // Y gets a small drift from its own angle
                self.beam_y_position += self.beam_y_angle * 0.5;
            }
            MagnetType::DefocusQuad => {
                // Thin lens defocusing in X: x' += k*x, size increases
                let k = magnet.power;
                self.beam_angle += k * self.beam_position;
                self.beam_size = (self.beam_size * (1.0 + k.abs() * 0.3)).min(APERTURE * 2.0);
                // Opposite in Y: focusing
                self.beam_y_angle -= k * self.beam_y_position;
                self.beam_y_size = (self.beam_y_size * (1.0 - k.abs() * 0.5)).max(1.0);
            }
            MagnetType::VTrim => {
                // Vertical trim dipole: kicks the vertical angle
                self.beam_y_angle += magnet.power;
                // Drift from the vertical kick
                self.beam_y_position += self.beam_y_angle * 1.0;
            }
            MagnetType::HTrim => {
                // Horizontal trim dipole: kicks the horizontal angle
                self.beam_angle += magnet.power;
                // Drift from the horizontal kick
                self.beam_position += self.beam_angle * 1.0;
            }
        }

        // Small drift between elements
        self.beam_position += self.beam_angle * 0.5;
        self.beam_y_position += self.beam_y_angle * 0.3;

        // Phase instability: constant beam size growth in Hard mode
        let growth = self.difficulty.size_growth_rate();
        if growth > 0.0 {
            self.beam_size += growth;
            self.beam_y_size += growth;
        }
    }

    fn advance_beam(&mut self) {
        self.beam_progress += 0.40;

        if self.beam_progress >= 1.0 {
            self.beam_progress = 0.0;
            self.apply_element();

            // Hard wall: instant loss if position exceeds aperture
            if self.beam_position.abs() > APERTURE || self.beam_y_position.abs() > APERTURE {
                self.beam_lost = true;
                self.message = Some(("Hit aperture wall!".to_string(), 60, Color::Rgb(255, 60, 60)));
                return;
            }

            // Loss zone: accumulate losses when beam edges extend past ±LOSS_ZONE
            let x_edge_pos = self.beam_position + self.beam_size * 0.5;
            let x_edge_neg = self.beam_position - self.beam_size * 0.5;
            let y_edge_pos = self.beam_y_position + self.beam_y_size * 0.5;
            let y_edge_neg = self.beam_y_position - self.beam_y_size * 0.5;

            let mut loss_this_step = 0.0_f32;
            if x_edge_pos > LOSS_ZONE { loss_this_step += (x_edge_pos - LOSS_ZONE) * 0.5; }
            if x_edge_neg < -LOSS_ZONE { loss_this_step += (-x_edge_neg - LOSS_ZONE) * 0.5; }
            if y_edge_pos > LOSS_ZONE { loss_this_step += (y_edge_pos - LOSS_ZONE) * 0.5; }
            if y_edge_neg < -LOSS_ZONE { loss_this_step += (-y_edge_neg - LOSS_ZONE) * 0.5; }

            if loss_this_step > 0.0 {
                self.beam_losses += loss_this_step;
            }

            if self.beam_losses >= MAX_LOSSES {
                self.beam_lost = true;
                self.message = Some((
                    format!("Beam losses exceeded {:.0}!", MAX_LOSSES),
                    60,
                    Color::Rgb(255, 100, 100),
                ));
                return;
            }

            // Check dynamic aperture restrictions
            for r in &self.restrictions {
                if self.beam_section == r.section && r.check(self.beam_position, self.beam_y_position) {
                    self.beam_lost = true;
                    self.message = Some((
                        format!("Hit section {} restriction! ({})", r.section + 1, r.label()),
                        60,
                        Color::Rgb(255, 100, 100),
                    ));
                    return;
                }
            }

            // Advance to next element
            self.beam_element += 1;
            if self.beam_element >= MAGNETS_PER_SECTION {
                self.beam_element = 0;
                // Record trail
                self.trail.push((self.beam_section, self.beam_position, self.beam_size));
                if self.trail.len() > NUM_SECTIONS * 3 {
                    self.trail.remove(0);
                }
                self.beam_section += 1;
                if self.beam_section >= NUM_SECTIONS {
                    self.beam_section = 0;
                    // Record beam position at turn boundary for bullseye plot
                    self.turn_positions.push((self.beam_position, self.beam_y_position));
                    if self.turn_positions.len() > 20 {
                        self.turn_positions.remove(0);
                    }
                    self.turns_completed += 1;
                    if self.turns_completed > self.best_turns {
                        self.best_turns = self.turns_completed;
                    }
                    if self.turns_completed >= GOAL_TURNS {
                        self.beam_completed = true;
                    }
                }
            }
        }
    }

    fn selected_section(&self) -> usize {
        self.selected / MAGNETS_PER_SECTION
    }

    fn selected_element(&self) -> usize {
        self.selected % MAGNETS_PER_SECTION
    }

    /// Copy current section's magnet settings (including all ramp points) to all other sections
    fn copy_to_all_sections(&mut self) {
        let src_sec = self.selected_section();
        let src_base = src_sec * MAGNETS_PER_SECTION;
        let powers: Vec<f32> = (0..MAGNETS_PER_SECTION)
            .map(|e| self.magnets[src_base + e].power)
            .collect();
        let ramps: Vec<[f32; 10]> = (0..MAGNETS_PER_SECTION)
            .map(|e| self.ramp_powers[src_base + e])
            .collect();
        for sec in 0..NUM_SECTIONS {
            if sec == src_sec { continue; }
            let base = sec * MAGNETS_PER_SECTION;
            for e in 0..MAGNETS_PER_SECTION {
                self.magnets[base + e].power = powers[e];
                self.ramp_powers[base + e] = ramps[e];
            }
        }
        self.message = Some((
            format!("Copied section {} to all (all ramps)!", src_sec + 1),
            45,
            Color::Rgb(80, 255, 180),
        ));
    }

    /// Jump to next section (keep same element position)
    fn next_section(&mut self) {
        let elem = self.selected_element();
        let sec = (self.selected_section() + 1) % NUM_SECTIONS;
        self.selected = sec * MAGNETS_PER_SECTION + elem;
    }

    /// Jump to previous section (keep same element position)
    fn prev_section(&mut self) {
        let elem = self.selected_element();
        let sec = if self.selected_section() == 0 { NUM_SECTIONS - 1 } else { self.selected_section() - 1 };
        self.selected = sec * MAGNETS_PER_SECTION + elem;
    }

    /// Get a stability indicator (how centered and small the beam is in both planes)
    fn stability_score(&self) -> f32 {
        if self.pos_history.is_empty() { return 0.0; }
        let avg_pos: f32 = self.pos_history.iter().map(|p| p.abs()).sum::<f32>() / self.pos_history.len() as f32;
        let avg_size: f32 = self.size_history.iter().sum::<f32>() / self.size_history.len().max(1) as f32;
        let avg_y_pos: f32 = self.y_pos_history.iter().map(|p| p.abs()).sum::<f32>() / self.y_pos_history.len().max(1) as f32;
        let avg_y_size: f32 = self.y_size_history.iter().sum::<f32>() / self.y_size_history.len().max(1) as f32;
        let x_pos_score = (1.0 - avg_pos / APERTURE).max(0.0);
        let x_size_score = (1.0 - avg_size / APERTURE).max(0.0);
        let y_pos_score = (1.0 - avg_y_pos / APERTURE).max(0.0);
        let y_size_score = (1.0 - avg_y_size / APERTURE).max(0.0);
        let x_score = x_pos_score * 0.6 + x_size_score * 0.4;
        let y_score = y_pos_score * 0.6 + y_size_score * 0.4;
        ((x_score + y_score) * 0.5) * 100.0
    }

    /// Get the ramp power for a magnet at a given turn number.
    /// Each turn 0-8 maps directly to ramp points 0-8. Turn 9+ uses ramp 8.
    fn ramp_power_for_turn(&self, magnet_idx: usize, turn: u32) -> f32 {
        let idx = (turn as usize).min(NUM_RAMPS - 1);
        self.ramp_powers[magnet_idx][idx]
    }

    /// Clamp a ramp value so it's within ±MAX_RAMP_DELTA of its neighbors.
    fn clamp_ramp_value(&self, magnet_idx: usize, ramp_idx: usize, value: f32) -> f32 {
        let mut v = value;
        if ramp_idx > 0 {
            let prev = self.ramp_powers[magnet_idx][ramp_idx - 1];
            v = v.clamp(prev - MAX_RAMP_DELTA, prev + MAX_RAMP_DELTA);
        }
        if ramp_idx < NUM_RAMPS - 1 {
            let next = self.ramp_powers[magnet_idx][ramp_idx + 1];
            v = v.clamp(next - MAX_RAMP_DELTA, next + MAX_RAMP_DELTA);
        }
        v
    }

    /// Sync all magnets' display power from ramp_powers at the selected ramp point.
    fn sync_display_from_ramp(&mut self) {
        let ramp_idx = self.selected_ramp;
        for i in 0..TOTAL_MAGNETS {
            self.magnets[i].power = self.ramp_powers[i][ramp_idx];
        }
    }

    /// Update all magnets' effective power from ramp based on current turn.
    fn sync_interpolated_powers(&mut self) {
        let turn = self.turns_completed;
        for i in 0..TOTAL_MAGNETS {
            self.magnets[i].power = self.ramp_power_for_turn(i, turn);
        }
    }

    /// Adjust a single magnet's ramp power at the selected ramp point with constraint enforcement.
    fn adjust_ramp_power(&mut self, magnet_idx: usize, delta: f32) {
        let ramp_idx = self.selected_ramp;
        let new_val = self.ramp_powers[magnet_idx][ramp_idx] + delta;
        let clamped = self.clamp_ramp_value(magnet_idx, ramp_idx, new_val);
        self.ramp_powers[magnet_idx][ramp_idx] = clamped;
        self.magnets[magnet_idx].power = clamped;
    }
}

impl Game for BeamGame {
    fn update(&mut self) {
        // Always tick message timer
        if let Some((_, ref mut ticks, _)) = self.message {
            if *ticks > 0 {
                *ticks -= 1;
            } else {
                self.message = None;
            }
        }
        if self.paused || self.beam_lost || self.beam_completed { return; }
        self.tick += 1;
        if self.beam_running {
            // Update magnet powers from ramp interpolation based on current turn
            self.sync_interpolated_powers();
            // Run multiple simulation steps per frame for faster gameplay
            for _ in 0..3 {
                if self.beam_lost || self.beam_completed { break; }
                self.advance_beam();
            }
            // Record history every few ticks
            if self.tick % 3 == 0 {
                self.pos_history.push(self.beam_position);
                self.size_history.push(self.beam_size);
                self.y_pos_history.push(self.beam_y_position);
                self.y_size_history.push(self.beam_y_size);
                if self.pos_history.len() > MAX_HISTORY {
                    self.pos_history.remove(0);
                }
                if self.size_history.len() > MAX_HISTORY {
                    self.size_history.remove(0);
                }
                if self.y_pos_history.len() > MAX_HISTORY {
                    self.y_pos_history.remove(0);
                }
                if self.y_size_history.len() > MAX_HISTORY {
                    self.y_size_history.remove(0);
                }
            }
        }
    }

    fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => self.reset(),
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if !self.beam_lost && !self.beam_completed {
                    self.paused = !self.paused;
                }
            }
            _ => {
                if self.beam_lost || self.beam_completed {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        self.reset();
                    }
                    return;
                }
                if self.paused { return; }
                match key.code {
                    KeyCode::Char(' ') => {
                        if !self.beam_running {
                            self.beam_running = true;
                            self.beam_position = self.target_x;
                            self.beam_angle = 0.0;
                            self.beam_size = 10.0;
                            self.beam_y_position = self.target_y;
                            self.beam_y_angle = 0.0;
                            self.beam_y_size = 10.0;
                            self.beam_section = 0;
                            self.beam_element = 0;
                            self.beam_progress = 0.0;
                            self.beam_losses = 0.0;
                            self.trail.clear();
                            self.pos_history.clear();
                            self.size_history.clear();
                            self.y_pos_history.clear();
                            self.y_size_history.clear();
                        }
                    }
                    KeyCode::Up => {
                        if self.bump.is_some() {
                            // In bump mode: adjust X+Y bump trims up
                            if let Some(ref bump) = self.bump {
                                let sec_coeffs = bump.section_coefficients();
                                let speed = self.adjust_speed;
                                for (sec, coeff) in &sec_coeffs {
                                    let ht_idx = sec * MAGNETS_PER_SECTION + 5;
                                    self.adjust_ramp_power(ht_idx, speed * coeff);
                                    let vt_idx = sec * MAGNETS_PER_SECTION + 4;
                                    self.adjust_ramp_power(vt_idx, speed * coeff);
                                }
                            }
                        } else {
                            // Select previous magnet
                            if self.selected == 0 {
                                self.selected = TOTAL_MAGNETS - 1;
                            } else {
                                self.selected -= 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if self.bump.is_some() {
                            // In bump mode: adjust X+Y bump trims down
                            if let Some(ref bump) = self.bump {
                                let sec_coeffs = bump.section_coefficients();
                                let speed = self.adjust_speed;
                                for (sec, coeff) in &sec_coeffs {
                                    let ht_idx = sec * MAGNETS_PER_SECTION + 5;
                                    self.adjust_ramp_power(ht_idx, -(speed * coeff));
                                    let vt_idx = sec * MAGNETS_PER_SECTION + 4;
                                    self.adjust_ramp_power(vt_idx, -(speed * coeff));
                                }
                            }
                        } else {
                            // Select next magnet
                            self.selected = (self.selected + 1) % TOTAL_MAGNETS;
                        }
                    }
                    KeyCode::Left => {
                        if self.bump.is_some() {
                            // In bump mode: shift bump start section backward
                            if let Some(ref mut bump) = self.bump {
                                bump.start_section = if bump.start_section == 0 {
                                    NUM_SECTIONS - 1
                                } else {
                                    bump.start_section - 1
                                };
                                self.message = Some((
                                    format!("{}-Bump {} start: sec {}",
                                        bump.size, bump.axis.label(),
                                        bump.start_section + 1),
                                    30,
                                    Color::Rgb(120, 220, 255),
                                ));
                            }
                        } else {
                            // Decrease power
                            let sel = self.selected;
                            let spd = self.adjust_speed;
                            self.adjust_ramp_power(sel, -spd);
                        }
                    }
                    KeyCode::Right => {
                        if self.bump.is_some() {
                            // In bump mode: shift bump start section forward
                            if let Some(ref mut bump) = self.bump {
                                bump.start_section = (bump.start_section + 1) % NUM_SECTIONS;
                                self.message = Some((
                                    format!("{}-Bump {} start: sec {}",
                                        bump.size, bump.axis.label(),
                                        bump.start_section + 1),
                                    30,
                                    Color::Rgb(120, 220, 255),
                                ));
                            }
                        } else {
                            // Increase power
                            let sel = self.selected;
                            let spd = self.adjust_speed;
                            self.adjust_ramp_power(sel, spd);
                        }
                    }
                    // Bump mode: W/S to adjust only X trims
                    KeyCode::Char('w') | KeyCode::Char('W') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                let ht_idx = sec * MAGNETS_PER_SECTION + 5;
                                self.adjust_ramp_power(ht_idx, speed * coeff);
                            }
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                let ht_idx = sec * MAGNETS_PER_SECTION + 5;
                                self.adjust_ramp_power(ht_idx, -(speed * coeff));
                            }
                        }
                    }
                    // Bump mode: E/Q to adjust only Y trims
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                let vt_idx = sec * MAGNETS_PER_SECTION + 4;
                                self.adjust_ramp_power(vt_idx, speed * coeff);
                            }
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                let vt_idx = sec * MAGNETS_PER_SECTION + 4;
                                self.adjust_ramp_power(vt_idx, -(speed * coeff));
                            }
                        }
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        self.adjust_speed = (self.adjust_speed * 2.0).min(1.0);
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        self.adjust_speed = (self.adjust_speed * 0.5).max(0.001);
                    }
                    // Copy current section settings to all sections
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.copy_to_all_sections();
                    }
                    // Jump to next/previous section (when not in bump mode)
                    KeyCode::Char(']') => {
                        if self.bump.is_none() {
                            self.next_section();
                        }
                    }
                    KeyCode::Char('[') => {
                        if self.bump.is_none() {
                            self.prev_section();
                        }
                    }
                    // Zero the selected magnet's ramp value (Z key, or zero bump trims in bump mode)
                    KeyCode::Char('z') | KeyCode::Char('Z') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let ramp_idx = self.selected_ramp;
                            for (sec, _) in &sec_coeffs {
                                let ht_idx = sec * MAGNETS_PER_SECTION + 5;
                                let vt_idx = sec * MAGNETS_PER_SECTION + 4;
                                let ht_clamped = self.clamp_ramp_value(ht_idx, ramp_idx, 0.0);
                                self.ramp_powers[ht_idx][ramp_idx] = ht_clamped;
                                self.magnets[ht_idx].power = ht_clamped;
                                let vt_clamped = self.clamp_ramp_value(vt_idx, ramp_idx, 0.0);
                                self.ramp_powers[vt_idx][ramp_idx] = vt_clamped;
                                self.magnets[vt_idx].power = vt_clamped;
                            }
                            self.message = Some((
                                format!("Zeroed bump trims (Ramp{})", self.selected_ramp),
                                30, Color::Rgb(255, 200, 80),
                            ));
                        } else {
                            let sel = self.selected;
                            let ramp_idx = self.selected_ramp;
                            let clamped = self.clamp_ramp_value(sel, ramp_idx, 0.0);
                            self.ramp_powers[sel][ramp_idx] = clamped;
                            self.magnets[sel].power = clamped;
                        }
                    }
                    // Ramp point selection: keys 0-9 select which ramp point to edit
                    KeyCode::Char(c @ '0'..='9') => {
                        let ramp_idx = (c as usize) - ('0' as usize);
                        self.selected_ramp = ramp_idx;
                        self.sync_display_from_ramp();
                        self.message = Some((
                            format!("Ramp{}", ramp_idx),
                            30, Color::Rgb(120, 200, 255),
                        ));
                    }
                    // Cycle bump modes: B cycles off -> 3 -> 4 -> 5 -> off
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        if let Some(ref bump) = self.bump {
                            let start = bump.start_section;
                            match bump.size {
                                3 => {
                                    self.bump = Some(BumpConfig::new(4, start));
                                    self.message = Some((
                                        format!("4-Bump mode (sec {}-{})", start + 1, (start + 3) % NUM_SECTIONS + 1),
                                        45, Color::Rgb(80, 255, 200),
                                    ));
                                }
                                4 => {
                                    self.bump = Some(BumpConfig::new(5, start));
                                    self.message = Some((
                                        format!("5-Bump mode (sec {}-{})", start + 1, (start + 4) % NUM_SECTIONS + 1),
                                        45, Color::Rgb(80, 255, 200),
                                    ));
                                }
                                _ => {
                                    self.bump = None;
                                    self.message = Some((
                                        "Bump mode OFF".to_string(), 30,
                                        Color::Rgb(140, 140, 160),
                                    ));
                                }
                            }
                        } else {
                            let start = self.selected_section();
                            self.bump = Some(BumpConfig::new(3, start));
                            self.message = Some((
                                format!("3-Bump mode (sec {}-{})", start + 1, (start + 2) % NUM_SECTIONS + 1),
                                45, Color::Rgb(80, 255, 200),
                            ));
                        }
                    }
                    // Toggle difficulty (only before beam starts)
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if !self.beam_running {
                            self.difficulty = self.difficulty.toggle();
                            self.message = Some((
                                format!("Difficulty: {}", self.difficulty.label()),
                                45,
                                self.difficulty.color(),
                            ));
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
            .border_style(Style::default().fg(Color::Rgb(100, 180, 255)))
            .title(" ⚛ Beam ")
            .title_style(Style::default().fg(Color::Rgb(120, 200, 255)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Status
                Constraint::Length(2),  // Beam X display bar
                Constraint::Length(2),  // Beam Y display bar
                Constraint::Min(8),    // Middle: magnet control + ring
                Constraint::Length(1),  // Help
            ])
            .split(inner);

        // Split middle area: magnet control on left, ring visualization on right
        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // Magnet control panel
                Constraint::Min(20),   // Ring visualization
            ])
            .split(chunks[3]);

        // Status bar
        let stability = self.stability_score();
        let stab_color = if stability > 80.0 { Color::Rgb(80, 255, 80) }
            else if stability > 50.0 { Color::Yellow }
            else if stability > 20.0 { Color::Rgb(255, 160, 50) }
            else { Color::Rgb(255, 60, 60) };
        let mut status_spans = vec![
            Span::styled(
                format!("[{}] ", self.difficulty.label()),
                Style::default().fg(self.difficulty.color()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Turns: {}/{} ", self.turns_completed, GOAL_TURNS),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Best: {} ", self.best_turns),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Size: {:.1}/{:.1} ", self.beam_size, self.beam_y_size),
                Style::default().fg(if self.beam_size > 30.0 || self.beam_y_size > 30.0 { Color::Red } else { Color::Green }),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("Loss: {:.0}/{:.0} ", self.beam_losses, MAX_LOSSES),
                Style::default().fg(
                    if self.beam_losses > 75.0 { Color::Rgb(255, 60, 60) }
                    else if self.beam_losses > 40.0 { Color::Rgb(255, 200, 50) }
                    else { Color::Rgb(100, 100, 140) }
                ).add_modifier(if self.beam_losses > 40.0 { Modifier::BOLD } else { Modifier::empty() }),
            ),
        ];
        status_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        status_spans.push(Span::styled(
            format!("Stability: {:.0}% ", stability),
            Style::default().fg(stab_color).add_modifier(Modifier::BOLD),
        ));
        // Score: sum of absolute power across all magnets
        let score: f32 = self.magnets.iter().map(|m| m.power.abs()).sum();
        status_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        status_spans.push(Span::styled(
            format!("Score: {:.3} ", score),
            Style::default().fg(Color::Rgb(255, 200, 80)).add_modifier(Modifier::BOLD),
        ));
        // Show flash message if active
        if let Some((ref msg, ticks, color)) = self.message {
            if ticks > 0 {
                status_spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                status_spans.push(Span::styled(
                    format!(" {} ", msg),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
            }
        }
        let status = Line::from(status_spans);
        frame.render_widget(Paragraph::new(status), chunks[0]);

        // Beam position visualization bar
        let bar_w = chunks[1].width as usize;
        let mut bar_lines: Vec<Line> = Vec::new();

        // Aperture bar
        let center = bar_w / 2;
        let scale = center as f32 / APERTURE;
        let mut bar_chars: Vec<(char, Style)> = vec![(' ', Style::default().bg(Color::Rgb(15, 15, 25))); bar_w];

        // Draw aperture limits (hard wall)
        let left_ap = center.saturating_sub((APERTURE * scale) as usize);
        let right_ap = (center + (APERTURE * scale) as usize).min(bar_w - 1);
        if left_ap < bar_w { bar_chars[left_ap] = ('│', Style::default().fg(Color::Red).bg(Color::Rgb(15, 15, 25))); }
        if right_ap < bar_w { bar_chars[right_ap] = ('│', Style::default().fg(Color::Red).bg(Color::Rgb(15, 15, 25))); }

        // Draw loss zone markers at ±25
        let left_lz = center.saturating_sub((LOSS_ZONE * scale) as usize);
        let right_lz = (center + (LOSS_ZONE * scale) as usize).min(bar_w - 1);
        if left_lz < bar_w && bar_chars[left_lz].0 == ' ' {
            bar_chars[left_lz] = ('┆', Style::default().fg(Color::Rgb(255, 200, 50)).bg(Color::Rgb(15, 15, 25)));
        }
        if right_lz < bar_w && bar_chars[right_lz].0 == ' ' {
            bar_chars[right_lz] = ('┆', Style::default().fg(Color::Rgb(255, 200, 50)).bg(Color::Rgb(15, 15, 25)));
        }

        // Draw beam with smooth gradient using density characters
        if self.beam_running && !self.beam_lost {
            let beam_center = (center as f32 + self.beam_position * scale) as usize;
            let beam_half = (self.beam_size * scale * 0.5) as usize;
            let bstart = beam_center.saturating_sub(beam_half);
            let bend = (beam_center + beam_half).min(bar_w);
            for x in bstart..bend {
                if x < bar_w {
                    let dist = (x as f32 - beam_center as f32).abs();
                    let intensity = 1.0 - dist / (beam_half as f32 + 1.0);
                    // Use density characters for smooth beam edges
                    let ch = if intensity > 0.8 { '█' }
                        else if intensity > 0.6 { '▓' }
                        else if intensity > 0.35 { '▒' }
                        else { '░' };
                    let g = (80.0 + intensity * 175.0) as u8;
                    let b = (120.0 + intensity * 135.0) as u8;
                    let r = (10.0 + intensity * 60.0) as u8;
                    bar_chars[x] = (ch, Style::default().fg(Color::Rgb(r, g, b)).bg(Color::Rgb(15, 15, 25)));
                }
            }
            if beam_center < bar_w {
                bar_chars[beam_center] = ('█', Style::default().fg(Color::Rgb(200, 255, 255)).bg(Color::Rgb(15, 15, 25)));
            }
        }

        // Center mark
        bar_chars[center] = if bar_chars[center].0 == ' ' {
            ('┊', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(15, 15, 25)))
        } else {
            bar_chars[center]
        };

        let spans: Vec<Span> = bar_chars.iter().map(|(ch, s)| Span::styled(String::from(*ch), *s)).collect();
        let x_color = if self.beam_position.abs() > 30.0 { Color::Red } else { Color::Green };
        bar_lines.push(Line::from(vec![
            Span::styled(" Beam X: ", Style::default().fg(Color::Rgb(100, 100, 140))),
            Span::styled(
                format!("{:+.1}", self.beam_position),
                Style::default().fg(x_color).add_modifier(Modifier::BOLD),
            ),
        ]));
        bar_lines.push(Line::from(spans));
        frame.render_widget(Paragraph::new(bar_lines), chunks[1]);

        // Beam Y position visualization bar
        let y_bar_w = chunks[2].width as usize;
        let mut y_bar_lines: Vec<Line> = Vec::new();
        let y_center = y_bar_w / 2;
        let y_scale = y_center as f32 / APERTURE;
        let mut y_bar_chars: Vec<(char, Style)> = vec![(' ', Style::default().bg(Color::Rgb(15, 15, 25))); y_bar_w];

        // Draw aperture limits (hard wall)
        let y_left_ap = y_center.saturating_sub((APERTURE * y_scale) as usize);
        let y_right_ap = (y_center + (APERTURE * y_scale) as usize).min(y_bar_w - 1);
        if y_left_ap < y_bar_w { y_bar_chars[y_left_ap] = ('│', Style::default().fg(Color::Red).bg(Color::Rgb(15, 15, 25))); }
        if y_right_ap < y_bar_w { y_bar_chars[y_right_ap] = ('│', Style::default().fg(Color::Red).bg(Color::Rgb(15, 15, 25))); }

        // Draw loss zone markers at ±25
        let y_left_lz = y_center.saturating_sub((LOSS_ZONE * y_scale) as usize);
        let y_right_lz = (y_center + (LOSS_ZONE * y_scale) as usize).min(y_bar_w - 1);
        if y_left_lz < y_bar_w && y_bar_chars[y_left_lz].0 == ' ' {
            y_bar_chars[y_left_lz] = ('┆', Style::default().fg(Color::Rgb(255, 200, 50)).bg(Color::Rgb(15, 15, 25)));
        }
        if y_right_lz < y_bar_w && y_bar_chars[y_right_lz].0 == ' ' {
            y_bar_chars[y_right_lz] = ('┆', Style::default().fg(Color::Rgb(255, 200, 50)).bg(Color::Rgb(15, 15, 25)));
        }

        // Draw beam Y with smooth gradient using density characters
        if self.beam_running && !self.beam_lost {
            let beam_y_center = (y_center as f32 + self.beam_y_position * y_scale) as usize;
            let beam_y_half = (self.beam_y_size * y_scale * 0.5) as usize;
            let ybstart = beam_y_center.saturating_sub(beam_y_half);
            let ybend = (beam_y_center + beam_y_half).min(y_bar_w);
            for x in ybstart..ybend {
                if x < y_bar_w {
                    let dist = (x as f32 - beam_y_center as f32).abs();
                    let intensity = 1.0 - dist / (beam_y_half as f32 + 1.0);
                    let ch = if intensity > 0.8 { '█' }
                        else if intensity > 0.6 { '▓' }
                        else if intensity > 0.35 { '▒' }
                        else { '░' };
                    let r = (60.0 + intensity * 140.0) as u8;
                    let b = (100.0 + intensity * 155.0) as u8;
                    y_bar_chars[x] = (ch, Style::default().fg(Color::Rgb(r, 20, b)).bg(Color::Rgb(15, 15, 25)));
                }
            }
            if beam_y_center < y_bar_w {
                y_bar_chars[beam_y_center] = ('█', Style::default().fg(Color::Rgb(255, 200, 255)).bg(Color::Rgb(15, 15, 25)));
            }
        }

        // Center mark
        y_bar_chars[y_center] = if y_bar_chars[y_center].0 == ' ' {
            ('┊', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(15, 15, 25)))
        } else {
            y_bar_chars[y_center]
        };

        let y_spans: Vec<Span> = y_bar_chars.iter().map(|(ch, s)| Span::styled(String::from(*ch), *s)).collect();
        let y_color = if self.beam_y_position.abs() > 30.0 { Color::Red } else { Color::Rgb(120, 200, 255) };
        y_bar_lines.push(Line::from(vec![
            Span::styled(" Beam Y: ", Style::default().fg(Color::Rgb(140, 100, 160))),
            Span::styled(
                format!("{:+.1}", self.beam_y_position),
                Style::default().fg(y_color).add_modifier(Modifier::BOLD),
            ),
        ]));
        y_bar_lines.push(Line::from(y_spans));
        frame.render_widget(Paragraph::new(y_bar_lines), chunks[2]);

        // Ring visualization - show all 24 sections as a compact ring layout
        let ring_w = middle[1].width as usize;
        let ring_h = middle[1].height as usize;
        let cx = ring_w as f32 / 2.0;
        let cy = ring_h as f32 / 2.0;
        let rx = (ring_w as f32 * 0.35).min(cx - 4.0);
        let ry = (ring_h as f32 * 0.38).min(cy - 2.0);

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default()); ring_w]; ring_h];

        // Draw connecting arcs between sections using fine characters
        let connect_steps = 2; // fewer dots for compact ring
        for sec in 0..NUM_SECTIONS {
            let a1 = (sec as f32 / NUM_SECTIONS as f32) * std::f32::consts::PI * 2.0 - std::f32::consts::FRAC_PI_2;
            let a2 = ((sec + 1) as f32 / NUM_SECTIONS as f32) * std::f32::consts::PI * 2.0 - std::f32::consts::FRAC_PI_2;
            for step in 1..=connect_steps {
                let t = step as f32 / (connect_steps + 1) as f32;
                let a = a1 + (a2 - a1) * t;
                let dx = (cx + rx * a.cos()) as usize;
                let dy = (cy + ry * a.sin()) as usize;
                if dx < ring_w && dy < ring_h && grid[dy][dx].0 == ' ' {
                    // Use different connector chars based on angle for a nicer arc
                    let tangent = a.cos().abs();
                    let ch = if tangent > 0.7 { '─' } else if tangent < 0.3 { '│' } else { '·' };
                    grid[dy][dx] = (ch, Style::default().fg(Color::Rgb(30, 40, 55)));
                }
            }
        }

        // Draw ring sections
        for sec in 0..NUM_SECTIONS {
            let angle = (sec as f32 / NUM_SECTIONS as f32) * std::f32::consts::PI * 2.0 - std::f32::consts::FRAC_PI_2;
            let x = cx + rx * angle.cos();
            let y = cy + ry * angle.sin();
            let ix = x as usize;
            let iy = y as usize;

            if ix >= ring_w || iy >= ring_h { continue; }

            // Determine section display
            let is_beam_here = self.beam_running && !self.beam_lost && self.beam_section == sec;
            let is_selected = self.selected_section() == sec;
            let is_bump_section = self.bump.as_ref().map_or(false, |b| b.contains_section(sec));

            // Check trail
            let trail_entry = self.trail.iter().rev().find(|(s, _, _)| *s == sec);

            let (ch, style) = if is_beam_here {
                ('◉', Style::default().fg(Color::Rgb(100, 255, 255)).add_modifier(Modifier::BOLD))
            } else if let Some((_, pos, _size)) = trail_entry {
                let intensity = if pos.abs() < 10.0 { 200 } else if pos.abs() < 30.0 { 140 } else { 80 };
                ('●', Style::default().fg(Color::Rgb(30, intensity as u8, (intensity + 30).min(255) as u8)))
            } else if is_bump_section {
                // Highlight bump sections with coefficient indicator
                let coeff = self.bump.as_ref().and_then(|b| b.coeff_for_section(sec)).unwrap_or(0.0);
                let ch = if coeff > 0.0 { '⊕' } else { '⊖' };
                let color = if coeff > 0.0 {
                    Color::Rgb(80, 255, 180) // green for positive
                } else {
                    Color::Rgb(255, 140, 80) // orange for negative
                };
                (ch, Style::default().fg(color).add_modifier(Modifier::BOLD))
            } else if is_selected {
                ('◈', Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD))
            } else if self.restrictions.iter().any(|r| r.section == sec) {
                // Section has aperture restriction - show warning marker
                ('◆', Style::default().fg(Color::Rgb(255, 80, 80)))
            } else {
                ('○', Style::default().fg(Color::Rgb(60, 80, 100)))
            };

            grid[iy][ix] = (ch, style);

            // Section number label (offset outward, tighter for compact ring)
            let label_angle = angle;
            let lx = (cx + (rx + 2.0) * label_angle.cos()) as usize;
            let ly = (cy + (ry + 1.0) * label_angle.sin()) as usize;
            if lx < ring_w && ly < ring_h {
                // Section number + restriction info for restricted sections
                let sec_restrictions: Vec<String> = self.restrictions.iter()
                    .filter(|r| r.section == sec)
                    .map(|r| r.label())
                    .collect();
                let has_restriction = !sec_restrictions.is_empty();
                let label = if has_restriction {
                    format!("{}:{}", sec + 1, sec_restrictions.join(","))
                } else {
                    format!("{}", sec + 1)
                };
                for (i, c) in label.chars().enumerate() {
                    let nx = lx + i;
                    if nx < ring_w {
                        let col = if has_restriction {
                            Color::Rgb(255, 100, 100)
                        } else if is_selected {
                            Color::Rgb(255, 220, 80)
                        } else {
                            Color::Rgb(60, 60, 80)
                        };
                        grid[ly][nx] = (c, Style::default().fg(col));
                    }
                }
            }
        }

        // Center text
        let center_text = if self.beam_completed {
            "✓ STABLE!"
        } else if self.beam_lost {
            "✗ LOST"
        } else if self.paused {
            "PAUSED"
        } else if !self.beam_running {
            "READY"
        } else {
            "RUNNING"
        };
        let ctx = (cx as usize).saturating_sub(center_text.len() / 2);
        let cty = cy as usize;
        let ct_color = if self.beam_completed { Color::Rgb(80, 255, 80) }
            else if self.beam_lost { Color::Rgb(255, 80, 80) }
            else if self.paused { Color::Rgb(255, 200, 50) }
            else if self.beam_running { Color::Rgb(80, 200, 255) }
            else { Color::Rgb(140, 140, 160) };
        for (i, c) in center_text.chars().enumerate() {
            let x = ctx + i;
            if x < ring_w && cty < ring_h {
                grid[cty][x] = (c, Style::default().fg(ct_color).add_modifier(Modifier::BOLD));
            }
        }

        // Draw position history sparkline below center text
        if !self.pos_history.is_empty() && cty + 1 < ring_h {
            let sparkline_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
            let spark_w = (rx as usize).min(self.pos_history.len()).min(20);
            let start_idx = self.pos_history.len().saturating_sub(spark_w);
            let spark_x = (cx as usize).saturating_sub(spark_w / 2);
            for (i, &val) in self.pos_history[start_idx..].iter().enumerate() {
                let x = spark_x + i;
                if x < ring_w && cty + 1 < ring_h {
                    let norm = (val.abs() / APERTURE).min(1.0);
                    let idx = (norm * 7.0) as usize;
                    let color = if norm < 0.2 { Color::Rgb(50, 200, 100) }
                        else if norm < 0.5 { Color::Rgb(200, 200, 50) }
                        else { Color::Rgb(200, 60, 60) };
                    grid[cty + 1][x] = (sparkline_chars[idx], Style::default().fg(color));
                }
            }
        }

        let lines: Vec<Line> = grid.into_iter()
            .map(|row| {
                Line::from(row.into_iter()
                    .map(|(ch, s)| Span::styled(String::from(ch), s))
                    .collect::<Vec<_>>())
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), middle[1]);

        // Split left panel: bullseye on top, magnet control on bottom
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(8),    // Bullseye closed orbit plot
                Constraint::Length(14), // Magnet control panel
            ])
            .split(middle[0]);

        // Magnet detail panel - show magnet info or bump info (LEFT of ring, top)
        let sec = self.selected_section();
        let elem = self.selected_element();
        let sec_base = sec * MAGNETS_PER_SECTION;

        if let Some(ref bump) = self.bump {
            // BUMP MODE detail panel (vertical layout)
            let sec_coeffs = bump.section_coefficients();
            let mut panel_lines: Vec<Line> = Vec::new();

            // Bump header
            panel_lines.push(Line::from(vec![
                Span::styled(
                    format!(" ⊕⊖ {}-BUMP", bump.size),
                    Style::default().fg(Color::Rgb(80, 255, 200)).add_modifier(Modifier::BOLD),
                ),
            ]));

            // Section coefficients stacked vertically
            for (s, c) in &sec_coeffs {
                let sign_str = if *c > 0.0 { "+" } else { "−" };
                let color = if *c > 0.0 {
                    Color::Rgb(80, 255, 180)
                } else {
                    Color::Rgb(255, 140, 80)
                };
                panel_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}S{}(×{:.0})", sign_str, s + 1, c.abs()),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                ]));
            }

            // Blank separator
            panel_lines.push(Line::from(Span::styled("", Style::default())));

            // Show HT and VT values for each bump section, stacked
            for (s, _) in &sec_coeffs {
                let ht_idx = s * MAGNETS_PER_SECTION + 5;
                let vt_idx = s * MAGNETS_PER_SECTION + 4;
                panel_lines.push(Line::from(vec![
                    Span::styled(format!("  S{} ", s + 1), Style::default().fg(Color::Rgb(160, 160, 180))),
                    Span::styled("HT", Style::default().fg(Color::Rgb(255, 180, 120))),
                    Span::styled(format!("{:+.3}", self.magnets[ht_idx].power), Style::default().fg(Color::Rgb(255, 200, 140))),
                ]));
                panel_lines.push(Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled("VT", Style::default().fg(Color::Rgb(200, 120, 255))),
                    Span::styled(format!("{:+.3}", self.magnets[vt_idx].power), Style::default().fg(Color::Rgb(220, 160, 255))),
                ]));
            }

            // Blank separator
            panel_lines.push(Line::from(Span::styled("", Style::default())));

            // Controls summary (stacked for narrow panel)
            panel_lines.push(Line::from(vec![
                Span::styled(" ↑↓", Style::default().fg(Color::Rgb(255, 255, 100)).add_modifier(Modifier::BOLD)),
                Span::styled(" X+Y ", Style::default().fg(Color::Rgb(140, 140, 160))),
                Span::styled("W/S", Style::default().fg(Color::Rgb(255, 180, 120)).add_modifier(Modifier::BOLD)),
                Span::styled(" X", Style::default().fg(Color::Rgb(140, 140, 160))),
            ]));
            panel_lines.push(Line::from(vec![
                Span::styled(" E/Q", Style::default().fg(Color::Rgb(200, 120, 255)).add_modifier(Modifier::BOLD)),
                Span::styled(" Y ", Style::default().fg(Color::Rgb(140, 140, 160))),
                Span::styled("←→", Style::default().fg(Color::Rgb(120, 220, 255)).add_modifier(Modifier::BOLD)),
                Span::styled(" shift", Style::default().fg(Color::Rgb(140, 140, 160))),
            ]));
            panel_lines.push(Line::from(vec![
                Span::styled(" Z", Style::default().fg(Color::Rgb(255, 200, 80)).add_modifier(Modifier::BOLD)),
                Span::styled(" zero ", Style::default().fg(Color::Rgb(140, 140, 160))),
                Span::styled("B", Style::default().fg(Color::Rgb(140, 140, 160)).add_modifier(Modifier::BOLD)),
                Span::styled(" exit", Style::default().fg(Color::Rgb(140, 140, 160))),
            ]));

            let detail = Paragraph::new(panel_lines)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 180, 140)))
                    .title(format!(" {}-Bump ", bump.size))
                    .title_style(Style::default().fg(Color::Rgb(80, 255, 200)).add_modifier(Modifier::BOLD)));
            frame.render_widget(detail, left_chunks[1]);
        } else {
            // Normal magnet detail panel - vertical layout with magnets stacked
            let mut panel_lines: Vec<Line> = Vec::new();

            // Section header with ramp and step size
            panel_lines.push(Line::from(vec![
                Span::styled(
                    format!(" Sec {}/{}", sec + 1, NUM_SECTIONS),
                    Style::default().fg(Color::Rgb(200, 200, 220)).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" R{}", self.selected_ramp),
                    Style::default().fg(Color::Rgb(180, 140, 255)).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" ⚡{:.3}", self.adjust_speed),
                    Style::default().fg(Color::Rgb(140, 140, 160)),
                ),
            ]));

            // Blank separator
            panel_lines.push(Line::from(Span::styled("", Style::default())));

            // Each magnet on its own line with mini power bar
            for e in 0..MAGNETS_PER_SECTION {
                let mag = &self.magnets[sec_base + e];
                let is_sel = e == elem;
                let bar_width = 8;
                let power_norm = (mag.power.abs() / 0.5).min(1.0);
                let filled = (power_norm * bar_width as f32) as usize;

                let mut spans: Vec<Span> = Vec::new();
                // Selection indicator
                if is_sel {
                    spans.push(Span::styled(" ▸", Style::default().fg(Color::Rgb(255, 255, 100))));
                } else {
                    spans.push(Span::styled("  ", Style::default()));
                }
                // Magnet label
                spans.push(Span::styled(
                    format!("{}", mag.mag_type.label()),
                    Style::default()
                        .fg(if is_sel { Color::Rgb(255, 255, 255) } else { mag.mag_type.color() })
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                ));
                // Power value
                spans.push(Span::styled(
                    format!(" {:+.4} ", mag.power),
                    Style::default()
                        .fg(if is_sel { Color::Rgb(255, 220, 80) } else { Color::Rgb(120, 120, 150) })
                        .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                ));
                // Mini power bar
                let bar_color = mag.mag_type.color();
                for i in 0..bar_width {
                    if i < filled {
                        spans.push(Span::styled("█", Style::default().fg(bar_color)));
                    } else {
                        spans.push(Span::styled("░", Style::default().fg(Color::Rgb(35, 35, 50))));
                    }
                }

                panel_lines.push(Line::from(spans));
            }

            // Blank separator
            panel_lines.push(Line::from(Span::styled("", Style::default())));

            // Controls hint
            panel_lines.push(Line::from(vec![
                Span::styled(" ↑↓", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" mag ", Style::default().fg(Color::Rgb(100, 100, 130))),
                Span::styled("←→", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" pow", Style::default().fg(Color::Rgb(100, 100, 130))),
            ]));
            panel_lines.push(Line::from(vec![
                Span::styled(" []", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" sec ", Style::default().fg(Color::Rgb(100, 100, 130))),
                Span::styled("+/-", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" step", Style::default().fg(Color::Rgb(100, 100, 130))),
            ]));

            let detail = Paragraph::new(panel_lines)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 100, 140)))
                    .title(" Magnet Control ")
                    .title_style(Style::default().fg(Color::Rgb(120, 200, 255))));
            frame.render_widget(detail, left_chunks[1]);
        }

        // Bullseye closed orbit plot (above magnet control)
        {
            let bull_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(80, 80, 120)))
                .title(format!(" Orbit ({:+.1},{:+.1}) ", self.target_x, self.target_y))
                .title_style(Style::default().fg(Color::Rgb(255, 200, 80)));
            let bull_inner = bull_block.inner(left_chunks[0]);
            frame.render_widget(bull_block, left_chunks[0]);

            let bw = bull_inner.width as usize;
            let bh = bull_inner.height as usize;
            if bw > 2 && bh > 2 {
                let plot_range = 20.0_f32; // ±20 range
                let bcx = bw as f32 / 2.0;
                let bcy = bh as f32 / 2.0;
                let sx = bcx / plot_range;
                let sy = bcy / plot_range;

                let mut bgrid: Vec<Vec<(char, Style)>> =
                    vec![vec![(' ', Style::default().bg(Color::Rgb(10, 10, 18))); bw]; bh];

                // Draw faint crosshair axes through center
                let cx_i = bcx as usize;
                let cy_i = bcy as usize;
                for x in 0..bw {
                    if cy_i < bh {
                        bgrid[cy_i][x] = ('─', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18)));
                    }
                }
                for y in 0..bh {
                    if cx_i < bw {
                        bgrid[y][cx_i] = ('│', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18)));
                    }
                }
                if cx_i < bw && cy_i < bh {
                    bgrid[cy_i][cx_i] = ('┼', Style::default().fg(Color::Rgb(30, 30, 50)).bg(Color::Rgb(10, 10, 18)));
                }

                // Draw bullseye rings around target
                let tx = (bcx + self.target_x * sx) as usize;
                let ty = (bcy - self.target_y * sy) as usize;
                // Outer ring (radius ~3 chars)
                for angle_step in 0..24 {
                    let a = (angle_step as f32 / 24.0) * std::f32::consts::PI * 2.0;
                    let rx_r = 4.0_f32;
                    let ry_r = 2.0_f32;
                    let px = (tx as f32 + rx_r * a.cos()) as usize;
                    let py = (ty as f32 + ry_r * a.sin()) as usize;
                    if px < bw && py < bh && bgrid[py][px].0 == ' ' || (px < bw && py < bh && bgrid[py][px].0 == '─') || (px < bw && py < bh && bgrid[py][px].0 == '│') {
                        bgrid[py][px] = ('·', Style::default().fg(Color::Rgb(60, 50, 30)).bg(Color::Rgb(10, 10, 18)));
                    }
                }

                // Draw target marker
                if tx < bw && ty < bh {
                    bgrid[ty][tx] = ('◎', Style::default().fg(Color::Rgb(255, 200, 80)).bg(Color::Rgb(10, 10, 18)).add_modifier(Modifier::BOLD));
                }

                // Draw turn positions (older = dimmer)
                let n = self.turn_positions.len();
                for (i, &(px, py)) in self.turn_positions.iter().enumerate() {
                    let dot_x = (bcx + px * sx) as usize;
                    let dot_y = (bcy - py * sy) as usize;
                    if dot_x < bw && dot_y < bh {
                        let age = (n - 1 - i) as f32;
                        let brightness = (1.0 - age / 12.0).max(0.3);
                        let dist = ((px - self.target_x).powi(2) + (py - self.target_y).powi(2)).sqrt();
                        let (ch, color) = if dist < 2.0 {
                            ('●', Color::Rgb((80.0 * brightness) as u8, (255.0 * brightness) as u8, (80.0 * brightness) as u8))
                        } else if dist < 8.0 {
                            ('●', Color::Rgb((255.0 * brightness) as u8, (255.0 * brightness) as u8, (50.0 * brightness) as u8))
                        } else {
                            ('●', Color::Rgb((255.0 * brightness) as u8, (60.0 * brightness) as u8, (60.0 * brightness) as u8))
                        };
                        bgrid[dot_y][dot_x] = (ch, Style::default().fg(color).bg(Color::Rgb(10, 10, 18)));
                    }
                }

                // Draw current beam position as bright marker (if running)
                if self.beam_running && !self.beam_lost {
                    let cur_x = (bcx + self.beam_position * sx) as usize;
                    let cur_y = (bcy - self.beam_y_position * sy) as usize;
                    if cur_x < bw && cur_y < bh {
                        bgrid[cur_y][cur_x] = ('◆', Style::default().fg(Color::Rgb(100, 255, 255)).bg(Color::Rgb(10, 10, 18)).add_modifier(Modifier::BOLD));
                    }
                }

                let bull_lines: Vec<Line> = bgrid.into_iter()
                    .map(|row| {
                        Line::from(row.into_iter()
                            .map(|(ch, s)| Span::styled(String::from(ch), s))
                            .collect::<Vec<_>>())
                    })
                    .collect();
                frame.render_widget(Paragraph::new(bull_lines), bull_inner);
            }
        }

        // Help bar
        if self.beam_lost {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" ✗ BEAM LOST! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("Adjust magnets and press ENTER to retry, Esc for menu", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(msg, chunks[4]);
        } else if self.beam_completed {
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(
                format!(" ✓ BEAM STABLE! {} turns! Score: {:.4} ", GOAL_TURNS, self.magnets.iter().map(|m| m.power.abs()).sum::<f32>()),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
                Span::styled("Press ENTER to play again", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(msg, chunks[4]);
        } else if self.bump.is_some() {
            // Bump mode help bar
            let help = Paragraph::new(Line::from(vec![
                Span::styled(" BUMP ", Style::default().fg(Color::Rgb(80, 255, 200)).add_modifier(Modifier::BOLD)),
                Span::styled("│ ↑↓ X+Y │ W/S X │ E/Q Y │ ←→ Shift │ 0-9 Ramp │ Z Zero │ B Cycle/Exit │ +/- Step │ P │ Esc",
                    Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(help, chunks[4]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(if self.beam_running { " SPACE: running " } else { " SPACE: start " },
                    Style::default().fg(if self.beam_running { Color::Green } else { Color::Yellow })),
                Span::styled("│ ↑↓ Mag │ ←→ Pow │ [] Sec │ 0-9 Ramp │ B Bump │ C Copy │ +/- Step │ Z Zero │ D Diff │ P │ Esc",
                    Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(help, chunks[4]);
        }
    }

    fn reset(&mut self) {
        let best = self.best_turns;
        let diff = self.difficulty;
        let restrictions = self.restrictions.clone();
        let magnets = self.magnets.clone();
        let selected = self.selected;
        let adjust_speed = self.adjust_speed;
        let bump = self.bump.clone();
        let ramp_powers = self.ramp_powers.clone();
        let selected_ramp = self.selected_ramp;
        let target_x = self.target_x;
        let target_y = self.target_y;
        *self = BeamGame::new();
        self.best_turns = best;
        self.target_x = target_x;
        self.target_y = target_y;
        self.difficulty = diff;
        self.restrictions = restrictions;
        self.magnets = magnets;
        self.selected = selected;
        self.adjust_speed = adjust_speed;
        self.bump = bump;
        self.ramp_powers = ramp_powers;
        self.selected_ramp = selected_ramp;
        // Sync display to show selected ramp values
        self.sync_display_from_ramp();
    }
}
