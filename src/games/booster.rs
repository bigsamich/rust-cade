#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::games::Game;

// ── Fermilab Booster Physical Constants ──────────────────────────────────────
const NUM_SECTIONS: usize = 24;       // 24 identical periods (superperiods)
const CIRCUMFERENCE: f64 = 474.2;     // meters
const CELL_LENGTH: f64 = CIRCUMFERENCE / NUM_SECTIONS as f64; // ~19.76 m

// Combined-function magnet parameters
const MAGNET_LENGTH: f64 = 2.889;     // meters per magnet
const SHORT_DRIFT: f64 = 1.2;         // short straight section (m)
const LONG_DRIFT: f64 = 6.0;          // long straight section (m) — RF cavities here

// Gradient strengths (at injection, Bρ_inj)
const K1_F_INJECTION: f64 = 0.0542;   // focusing gradient (m^-2)
const K1_D_INJECTION: f64 = 0.0577;   // defocusing gradient magnitude (m^-2)

// Dipole bending: 96 magnets share 2π of bend
const DIPOLE_ANGLE: f64 = std::f64::consts::TAU / 96.0; // ~0.0654 rad per magnet
const DIPOLE_FIELD_INJECTION: f64 = 0.0542; // T (at 400 MeV) — placeholder normalized

// Energy parameters
const E_INJECTION_GEV: f64 = 0.4;     // kinetic energy at injection (GeV)
const E_EXTRACTION_GEV: f64 = 8.0;    // kinetic energy at extraction (GeV)
const PROTON_MASS_GEV: f64 = 0.93827; // proton rest mass (GeV/c²)
const GAMMA_TRANSITION: f64 = 5.446;  // transition gamma

// RF parameters
const HARMONIC_NUMBER: u32 = 84;
const NUM_RF_CAVITIES: u32 = 22;
const MAX_RF_VOLTAGE_MV: f64 = 1.16;  // MV total ring voltage
const CYCLE_FREQ_HZ: f64 = 15.0;      // cycling rate (Hz)

// Tunes (bare lattice at injection)
const TUNE_X_BARE: f64 = 6.7;
const TUNE_Y_BARE: f64 = 6.8;

// Aperture in normalized coordinates (mm)
const F_APERTURE_H_MM: f64 = 54.6;    // 4.3" / 2 → mm
const F_APERTURE_V_MM: f64 = 20.8;    // 1.64" / 2 → mm
const D_APERTURE_H_MM: f64 = 38.1;    // 3.0" / 2 → mm
const D_APERTURE_V_MM: f64 = 28.6;    // 2.25" / 2 → mm

// Beam parameters
const EMITTANCE_NORM_95: f64 = 12.0;  // π mm·mrad (normalized, 95%)
const LONG_EMITTANCE_EVS: f64 = 0.10; // eV·s (95%) at injection

// Game display scaling
const APERTURE_DISPLAY: f32 = 50.0;   // display units for full aperture
const LOSS_ZONE: f32 = 25.0;          // beam edges past this accumulate losses
const MAX_LOSSES: f32 = 100.0;        // game over threshold

// Simulation
const ELEMENTS_PER_CELL: usize = 6;   // F, short_drift, F, D, long_drift, D
const TOTAL_ELEMENTS: usize = NUM_SECTIONS * ELEMENTS_PER_CELL;
const MAX_HISTORY: usize = 60;

// Ramp timing: total ramp is ~33ms (half-period of 15Hz sinusoid)
// We discretize into turns around the ring
// Revolution period = C / (β·c) ≈ 2.2 μs at injection → ~15,000 turns in a cycle
const TURNS_IN_CYCLE: u32 = 15000;
const TURNS_TO_TRANSITION: u32 = 7100; // approximate turn at γ = γ_t

// Corrector magnets per cell: located in long straight section
const CORRECTORS_PER_CELL: usize = 4; // H-trim, V-trim, trim-quad, skew-quad
const SEXTUPOLES_PER_CELL: usize = 2; // 2 families for chromaticity

// ── Element Types ────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum ElementType {
    FMagnet,     // combined-function: dipole + focusing quad
    DMagnet,     // combined-function: dipole + defocusing quad
    ShortDrift,  // short straight section (~1.2m)
    LongDrift,   // long straight section (~6.0m) — RF + correctors here
}

impl ElementType {
    fn label(&self) -> &str {
        match self {
            ElementType::FMagnet => "F",
            ElementType::DMagnet => "D",
            ElementType::ShortDrift => "Os",
            ElementType::LongDrift => "OL",
        }
    }

    fn color(&self) -> Color {
        match self {
            ElementType::FMagnet => Color::Rgb(80, 180, 255),
            ElementType::DMagnet => Color::Rgb(80, 255, 140),
            ElementType::ShortDrift => Color::Rgb(80, 80, 100),
            ElementType::LongDrift => Color::Rgb(120, 100, 80),
        }
    }

    fn length(&self) -> f64 {
        match self {
            ElementType::FMagnet | ElementType::DMagnet => MAGNET_LENGTH,
            ElementType::ShortDrift => SHORT_DRIFT,
            ElementType::LongDrift => LONG_DRIFT,
        }
    }
}

// ── Lattice Element ──────────────────────────────────────────────────────────
#[derive(Clone)]
struct LatticeElement {
    elem_type: ElementType,
    cell: usize,      // which cell (0-23)
    index: usize,     // position within cell (0-5)
}

// ── Corrector Package (one per cell, in the long straight) ───────────────────
#[derive(Clone)]
struct CorrectorPackage {
    h_trim: f64,       // horizontal trim dipole (rad)
    v_trim: f64,       // vertical trim dipole (rad)
    trim_quad: f64,    // trim quadrupole (ΔK, m^-2)
    skew_quad: f64,    // skew quadrupole (coupling)
    sext_a: f64,       // sextupole family A (chromaticity)
    sext_b: f64,       // sextupole family B (chromaticity)
}

impl CorrectorPackage {
    fn new() -> Self {
        Self {
            h_trim: 0.0,
            v_trim: 0.0,
            trim_quad: 0.0,
            skew_quad: 0.0,
            sext_a: 0.0,
            sext_b: 0.0,
        }
    }
}

// ── Bump Configuration (closed orbit bumps) ──────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum BumpAxis {
    X,
    Y,
}

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
    size: usize,
    start_section: usize,
    axis: BumpAxis,
}

impl BumpConfig {
    fn new(size: usize, start_section: usize) -> Self {
        Self { size, start_section, axis: BumpAxis::X }
    }

    fn coefficients(&self) -> Vec<f64> {
        match self.size {
            3 => vec![1.0, -2.0, 1.0],
            4 => vec![1.0, -1.0, -1.0, 1.0],
            5 => vec![1.0, -2.0, 2.0, -2.0, 1.0],
            _ => vec![],
        }
    }

    fn section_coefficients(&self) -> Vec<(usize, f64)> {
        self.coefficients()
            .iter()
            .enumerate()
            .map(|(i, &c)| ((self.start_section + i) % NUM_SECTIONS, c))
            .collect()
    }

    fn contains_section(&self, sec: usize) -> bool {
        for i in 0..self.size {
            if (self.start_section + i) % NUM_SECTIONS == sec {
                return true;
            }
        }
        false
    }

    fn coeff_for_section(&self, sec: usize) -> Option<f64> {
        let coeffs = self.coefficients();
        for i in 0..self.size {
            if (self.start_section + i) % NUM_SECTIONS == sec {
                return Some(coeffs[i]);
            }
        }
        None
    }
}

// ── Game Phase ───────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum GamePhase {
    Setup,            // pre-injection: adjust correctors
    Injection,        // beam just entered at 400 MeV
    EarlyRamp,        // ramping, tune correction needed
    PreTransition,    // approaching γ_t, chromaticity critical
    Transition,       // at γ ≈ γ_t, RF phase flip
    PostTransition,   // damp oscillations
    Extraction,       // reached 8 GeV — success!
    Lost,             // beam lost
}

impl GamePhase {
    fn label(&self) -> &str {
        match self {
            GamePhase::Setup => "SETUP",
            GamePhase::Injection => "INJECT",
            GamePhase::EarlyRamp => "RAMP",
            GamePhase::PreTransition => "PRE-Xt",
            GamePhase::Transition => "TRANSITION",
            GamePhase::PostTransition => "POST-Xt",
            GamePhase::Extraction => "EXTRACTED!",
            GamePhase::Lost => "LOST",
        }
    }

    fn color(&self) -> Color {
        match self {
            GamePhase::Setup => Color::Rgb(140, 140, 160),
            GamePhase::Injection => Color::Rgb(80, 200, 255),
            GamePhase::EarlyRamp => Color::Rgb(80, 255, 140),
            GamePhase::PreTransition => Color::Rgb(255, 200, 50),
            GamePhase::Transition => Color::Rgb(255, 60, 60),
            GamePhase::PostTransition => Color::Rgb(255, 140, 80),
            GamePhase::Extraction => Color::Rgb(80, 255, 80),
            GamePhase::Lost => Color::Rgb(255, 60, 60),
        }
    }
}

// ── Input Mode (for coordinate injection prompt) ─────────────────────────────
#[derive(Clone, PartialEq)]
enum InputMode {
    None,
    InjectX,  // typing X coordinate
    InjectY,  // typing Y coordinate
}

// ── Player-selected corrector type for editing ───────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum CorrectorSelect {
    HTrim,
    VTrim,
    TrimQuad,
    SkewQuad,
    SextA,
    SextB,
}

impl CorrectorSelect {
    fn label(&self) -> &str {
        match self {
            CorrectorSelect::HTrim => "H-Trim",
            CorrectorSelect::VTrim => "V-Trim",
            CorrectorSelect::TrimQuad => "Tr-Quad",
            CorrectorSelect::SkewQuad => "Sk-Quad",
            CorrectorSelect::SextA => "Sext-A",
            CorrectorSelect::SextB => "Sext-B",
        }
    }

    fn color(&self) -> Color {
        match self {
            CorrectorSelect::HTrim => Color::Rgb(255, 180, 120),
            CorrectorSelect::VTrim => Color::Rgb(200, 120, 255),
            CorrectorSelect::TrimQuad => Color::Rgb(120, 200, 255),
            CorrectorSelect::SkewQuad => Color::Rgb(255, 255, 120),
            CorrectorSelect::SextA => Color::Rgb(255, 120, 180),
            CorrectorSelect::SextB => Color::Rgb(180, 120, 255),
        }
    }

    fn next(&self) -> CorrectorSelect {
        match self {
            CorrectorSelect::HTrim => CorrectorSelect::VTrim,
            CorrectorSelect::VTrim => CorrectorSelect::TrimQuad,
            CorrectorSelect::TrimQuad => CorrectorSelect::SkewQuad,
            CorrectorSelect::SkewQuad => CorrectorSelect::SextA,
            CorrectorSelect::SextA => CorrectorSelect::SextB,
            CorrectorSelect::SextB => CorrectorSelect::HTrim,
        }
    }

    fn prev(&self) -> CorrectorSelect {
        match self {
            CorrectorSelect::HTrim => CorrectorSelect::SextB,
            CorrectorSelect::VTrim => CorrectorSelect::HTrim,
            CorrectorSelect::TrimQuad => CorrectorSelect::VTrim,
            CorrectorSelect::SkewQuad => CorrectorSelect::TrimQuad,
            CorrectorSelect::SextA => CorrectorSelect::SkewQuad,
            CorrectorSelect::SextB => CorrectorSelect::SextA,
        }
    }
}

// ── Display Mode ─────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum DisplayMode {
    Orbit,
    PhaseSpaceX,
    PhaseSpaceY,
    Longitudinal,
    TuneDiagram,
}

impl DisplayMode {
    fn label(&self) -> &str {
        match self {
            DisplayMode::Orbit => "Orbit",
            DisplayMode::PhaseSpaceX => "X-X'",
            DisplayMode::PhaseSpaceY => "Y-Y'",
            DisplayMode::Longitudinal => "Longit.",
            DisplayMode::TuneDiagram => "Tune",
        }
    }

    fn next(&self) -> DisplayMode {
        match self {
            DisplayMode::Orbit => DisplayMode::PhaseSpaceX,
            DisplayMode::PhaseSpaceX => DisplayMode::PhaseSpaceY,
            DisplayMode::PhaseSpaceY => DisplayMode::Longitudinal,
            DisplayMode::Longitudinal => DisplayMode::TuneDiagram,
            DisplayMode::TuneDiagram => DisplayMode::Orbit,
        }
    }

    fn prev(&self) -> DisplayMode {
        match self {
            DisplayMode::Orbit => DisplayMode::TuneDiagram,
            DisplayMode::PhaseSpaceX => DisplayMode::Orbit,
            DisplayMode::PhaseSpaceY => DisplayMode::PhaseSpaceX,
            DisplayMode::Longitudinal => DisplayMode::PhaseSpaceY,
            DisplayMode::TuneDiagram => DisplayMode::Longitudinal,
        }
    }
}

// ── Simulation Speed ─────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum SimSpeed {
    Slow,   // >   ~0.25 rev/sec
    Normal, // >>  ~1 rev/sec
    Fast,   // >>> ~10 rev/sec
}

impl SimSpeed {
    fn steps_per_tick(&self) -> u32 {
        match self {
            SimSpeed::Slow => 4,
            SimSpeed::Normal => 14,
            SimSpeed::Fast => 144,
        }
    }

    fn label(&self) -> &str {
        match self {
            SimSpeed::Slow => ">",
            SimSpeed::Normal => ">>",
            SimSpeed::Fast => ">>>",
        }
    }

    fn next(&self) -> SimSpeed {
        match self {
            SimSpeed::Slow => SimSpeed::Normal,
            SimSpeed::Normal => SimSpeed::Fast,
            SimSpeed::Fast => SimSpeed::Slow,
        }
    }
}

// ── Transfer Matrix (2x2 for one plane, or 3x3 with dispersion) ─────────────
#[derive(Clone, Copy)]
struct Matrix2 {
    m11: f64, m12: f64,
    m21: f64, m22: f64,
}

impl Matrix2 {
    fn identity() -> Self {
        Self { m11: 1.0, m12: 0.0, m21: 0.0, m22: 1.0 }
    }

    fn drift(length: f64) -> Self {
        Self { m11: 1.0, m12: length, m21: 0.0, m22: 1.0 }
    }

    fn focusing(k: f64, length: f64) -> Self {
        if k.abs() < 1e-12 {
            return Self::drift(length);
        }
        let sqrt_k = k.abs().sqrt();
        if k > 0.0 {
            // Focusing: cos/sin
            let phi = sqrt_k * length;
            Self {
                m11: phi.cos(),
                m12: phi.sin() / sqrt_k,
                m21: -sqrt_k * phi.sin(),
                m22: phi.cos(),
            }
        } else {
            // Defocusing: cosh/sinh
            let phi = sqrt_k * length;
            Self {
                m11: phi.cosh(),
                m12: phi.sinh() / sqrt_k,
                m21: sqrt_k * phi.sinh(),
                m22: phi.cosh(),
            }
        }
    }

    fn apply(&self, x: f64, xp: f64) -> (f64, f64) {
        (self.m11 * x + self.m12 * xp,
         self.m21 * x + self.m22 * xp)
    }

    fn multiply(&self, other: &Matrix2) -> Matrix2 {
        Matrix2 {
            m11: self.m11 * other.m11 + self.m12 * other.m21,
            m12: self.m11 * other.m12 + self.m12 * other.m22,
            m21: self.m21 * other.m11 + self.m22 * other.m21,
            m22: self.m21 * other.m12 + self.m22 * other.m22,
        }
    }
}

// ── Relativistic helpers ─────────────────────────────────────────────────────
fn kinetic_to_gamma(ke_gev: f64) -> f64 {
    (ke_gev + PROTON_MASS_GEV) / PROTON_MASS_GEV
}

fn gamma_to_beta(gamma: f64) -> f64 {
    (1.0 - 1.0 / (gamma * gamma)).sqrt()
}

fn gamma_to_momentum(gamma: f64) -> f64 {
    // p = γ·β·m·c in GeV/c
    let beta = gamma_to_beta(gamma);
    gamma * beta * PROTON_MASS_GEV
}

fn gamma_to_brho(gamma: f64) -> f64 {
    // Bρ = p / (q·c) in T·m; for protons p in GeV/c → Bρ = p / 0.29979 T·m
    gamma_to_momentum(gamma) / 0.29979
}

fn slip_factor(gamma: f64) -> f64 {
    // η = 1/γ_t² - 1/γ²
    1.0 / (GAMMA_TRANSITION * GAMMA_TRANSITION) - 1.0 / (gamma * gamma)
}

// ── Plot Tick Helpers ────────────────────────────────────────────────────────

/// Pick a "nice" tick interval (1, 2, or 5 × 10^n) yielding ~2-3 ticks per half-axis.
fn nice_tick_interval(half_range: f32) -> f32 {
    if half_range <= 0.0 { return 1.0; }
    let rough = half_range / 3.0;
    let mag = 10.0_f32.powf(rough.log10().floor());
    let norm = rough / mag;
    let nice = if norm < 1.5 { 1.0 } else if norm < 3.5 { 2.0 } else if norm < 7.5 { 5.0 } else { 10.0 };
    nice * mag
}

/// Format a tick value: integers when whole, one decimal otherwise.
fn format_tick_value(v: f32) -> String {
    if (v - v.round()).abs() < 0.01 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}

/// Draw tick marks and numeric labels on crosshair axes of a character-grid plot.
/// Call after drawing crosshairs, before drawing data (so data overwrites labels).
fn draw_plot_ticks(
    grid: &mut [Vec<(char, Style)>],
    bw: usize, bh: usize,
    bcx: f32, bcy: f32,
    sx: f32, sy: f32,
    x_range: f32, y_range: f32,
) {
    let tick_style = Style::default().fg(Color::Rgb(70, 70, 100)).bg(Color::Rgb(10, 10, 18));
    let label_style = Style::default().fg(Color::Rgb(55, 65, 90)).bg(Color::Rgb(10, 10, 18));
    let cx_i = bcx as usize;
    let cy_i = bcy as usize;

    let x_tick = nice_tick_interval(x_range);
    let y_tick = nice_tick_interval(y_range);

    // X-axis ticks (on horizontal crosshair row)
    let mut val = x_tick;
    while val <= x_range * 0.95 {
        for &sign in &[-1.0_f32, 1.0_f32] {
            let v = val * sign;
            let px = (bcx + v * sx) as usize;
            if px > 0 && px < bw && cy_i < bh {
                grid[cy_i][px] = ('+', tick_style);
                // Label one row below, centered on tick
                if cy_i + 1 < bh {
                    let label = format_tick_value(v);
                    let start = px.saturating_sub(label.len() / 2);
                    for (i, c) in label.chars().enumerate() {
                        let col = start + i;
                        if col < bw && col != cx_i {
                            grid[cy_i + 1][col] = (c, label_style);
                        }
                    }
                }
            }
        }
        val += x_tick;
    }

    // Y-axis ticks (on vertical crosshair column)
    val = y_tick;
    while val <= y_range * 0.95 {
        for &sign in &[-1.0_f32, 1.0_f32] {
            let v = val * sign;
            let py = (bcy - v * sy) as usize; // y inverted
            if py > 0 && py < bh && cx_i < bw {
                grid[py][cx_i] = ('+', tick_style);
                // Label to the right of axis
                let label = format_tick_value(v);
                for (i, c) in label.chars().enumerate() {
                    let col = cx_i + 1 + i;
                    if col < bw {
                        grid[py][col] = (c, label_style);
                    }
                }
            }
        }
        val += y_tick;
    }
}

// ── Main Game Struct ─────────────────────────────────────────────────────────
pub struct BoosterGame {
    // Lattice
    lattice: Vec<LatticeElement>,
    correctors: Vec<CorrectorPackage>,  // one per cell (24 total)

    // Beam transverse state (x, x', y, y' in mm and mrad)
    beam_x: f64,
    beam_xp: f64,
    beam_y: f64,
    beam_yp: f64,

    // Beam envelope (RMS sizes in mm)
    beam_sigma_x: f64,
    beam_sigma_y: f64,

    // Momentum offset δ = Δp/p
    beam_dp: f64,

    // Longitudinal coordinates (RF bucket)
    beam_phi: f64,      // RF phase relative to synchronous (rad)
    beam_de: f64,       // energy deviation from synchronous (GeV)

    // Energy ramp state
    current_ke_gev: f64,       // current kinetic energy
    current_gamma: f64,
    current_beta: f64,
    current_brho: f64,
    ramp_turn: u32,            // current turn number in the cycle

    // RF state
    rf_voltage_mv: f64,        // total RF voltage (MV) — player adjustable
    rf_phase_deg: f64,         // synchronous phase (degrees) — player adjustable

    // Computed optics (updated each turn based on energy)
    tune_x: f64,
    tune_y: f64,
    beta_x_max: f64,
    beta_y_max: f64,
    dispersion_max: f64,
    chromaticity_x: f64,       // natural + sextupole contribution
    chromaticity_y: f64,

    // Space charge tune shift
    sc_tune_shift: f64,
    beam_intensity: f64,       // relative intensity (1.0 = full, decreases with losses)

    // Tracking state
    beam_cell: usize,          // current cell (0-23)
    beam_element: usize,       // current element within cell (0-5)
    beam_progress: f64,        // fractional progress through element
    beam_running: bool,
    beam_lost: bool,
    beam_losses: f32,          // accumulated fractional losses

    // Game state
    phase: GamePhase,
    tick: u64,
    paused: bool,
    turns_completed: u32,
    best_turns: u32,
    transition_crossed: bool,

    // Player controls
    selected_cell: usize,       // which cell's correctors we're editing
    selected_corrector: CorrectorSelect,
    adjust_speed: f64,

    // Bump mode
    bump: Option<BumpConfig>,

    // Display data
    trail: Vec<(usize, f32, f32)>,   // (cell, x_pos, x_size) at cell boundaries
    pos_history: Vec<f32>,
    size_history: Vec<f32>,
    y_pos_history: Vec<f32>,
    y_size_history: Vec<f32>,
    turn_positions: Vec<(f32, f32)>,  // (x, y) at turn boundaries for orbit plot

    // Phase space history for longitudinal display
    phi_history: Vec<f32>,
    de_history: Vec<f32>,

    // Phase space history (turn-by-turn)
    x_xp_history: Vec<(f32, f32)>,
    y_yp_history: Vec<(f32, f32)>,

    // Display mode
    display_mode: DisplayMode,

    // Simulation speed
    sim_speed: SimSpeed,

    // Main bend bus (MDAT) and quad bus (MQAT) trims
    bend_bus_trim: f64,
    quad_bus_trim: f64,

    // Message flash
    message: Option<(String, u32, Color)>,

    // Injection coordinate input
    input_mode: InputMode,
    input_buffer: String,
    inject_x: f64,
    inject_y: f64,

    // Scoring
    initial_emittance_x: f64,
    initial_emittance_y: f64,
}

impl BoosterGame {
    pub fn new() -> Self {
        // Build lattice: per cell is F, Os, F, D, OL, D
        let mut lattice = Vec::new();
        for cell in 0..NUM_SECTIONS {
            lattice.push(LatticeElement { elem_type: ElementType::FMagnet, cell, index: 0 });
            lattice.push(LatticeElement { elem_type: ElementType::ShortDrift, cell, index: 1 });
            lattice.push(LatticeElement { elem_type: ElementType::FMagnet, cell, index: 2 });
            lattice.push(LatticeElement { elem_type: ElementType::DMagnet, cell, index: 3 });
            lattice.push(LatticeElement { elem_type: ElementType::LongDrift, cell, index: 4 });
            lattice.push(LatticeElement { elem_type: ElementType::DMagnet, cell, index: 5 });
        }

        let correctors: Vec<CorrectorPackage> = (0..NUM_SECTIONS)
            .map(|_| CorrectorPackage::new())
            .collect();

        let gamma_inj = kinetic_to_gamma(E_INJECTION_GEV);
        let beta_inj = gamma_to_beta(gamma_inj);
        let brho_inj = gamma_to_brho(gamma_inj);

        // Initial beam size from emittance: σ = √(ε·β_twiss / (βγ))
        // Using geometric emittance = ε_n / (βγ)
        let bg = beta_inj * gamma_inj;
        let geom_emit = EMITTANCE_NORM_95 / bg; // mm·mrad (geometric, 95%)
        // RMS = 95% / 6 for Gaussian
        let geom_emit_rms = geom_emit / 6.0;
        let sigma_x = (geom_emit_rms * 33.7_f64).sqrt(); // β_x,max ~ 33.7 m → convert to mm
        let sigma_y = (geom_emit_rms * 20.4_f64).sqrt(); // β_y,max ~ 20.4 m

        Self {
            lattice,
            correctors,

            beam_x: 0.0,
            beam_xp: 0.0,
            beam_y: 0.0,
            beam_yp: 0.0,
            beam_sigma_x: sigma_x,
            beam_sigma_y: sigma_y,
            beam_dp: 0.0,

            beam_phi: 0.0,
            beam_de: 0.0,

            current_ke_gev: E_INJECTION_GEV,
            current_gamma: gamma_inj,
            current_beta: beta_inj,
            current_brho: brho_inj,
            ramp_turn: 0,

            rf_voltage_mv: 0.5,    // start at moderate voltage
            rf_phase_deg: 0.0,     // synchronous phase

            tune_x: TUNE_X_BARE,
            tune_y: TUNE_Y_BARE,
            beta_x_max: 33.7,
            beta_y_max: 20.4,
            dispersion_max: 3.2,
            chromaticity_x: -7.0,  // natural chromaticity
            chromaticity_y: -8.0,
            sc_tune_shift: 0.0,
            beam_intensity: 1.0,

            beam_cell: 0,
            beam_element: 0,
            beam_progress: 0.0,
            beam_running: false,
            beam_lost: false,
            beam_losses: 0.0,

            phase: GamePhase::Setup,
            tick: 0,
            paused: false,
            turns_completed: 0,
            best_turns: 0,
            transition_crossed: false,

            selected_cell: 0,
            selected_corrector: CorrectorSelect::HTrim,
            adjust_speed: 0.001,

            bump: None,

            trail: Vec::new(),
            pos_history: Vec::new(),
            size_history: Vec::new(),
            y_pos_history: Vec::new(),
            y_size_history: Vec::new(),
            turn_positions: Vec::new(),
            phi_history: Vec::new(),
            de_history: Vec::new(),
            x_xp_history: Vec::new(),
            y_yp_history: Vec::new(),

            display_mode: DisplayMode::Orbit,
            sim_speed: SimSpeed::Slow,
            bend_bus_trim: 0.0,
            quad_bus_trim: 0.0,

            message: None,

            input_mode: InputMode::None,
            input_buffer: String::new(),
            inject_x: 0.0,
            inject_y: 0.0,

            initial_emittance_x: geom_emit_rms,
            initial_emittance_y: geom_emit_rms,
        }
    }

    // ── Energy Ramp ──────────────────────────────────────────────────────
    /// Sinusoidal ramp: B(t) = B_min + 0.5*(B_max - B_min)*(1 - cos(ωt))
    /// We parameterize by turn number within the cycle.
    fn update_energy_for_turn(&mut self) {
        let t_frac = self.ramp_turn as f64 / TURNS_IN_CYCLE as f64;
        // Sinusoidal energy ramp from injection to extraction
        let ke = E_INJECTION_GEV
            + 0.5 * (E_EXTRACTION_GEV - E_INJECTION_GEV) * (1.0 - (std::f64::consts::PI * t_frac).cos());
        self.current_ke_gev = ke;
        self.current_gamma = kinetic_to_gamma(ke);
        self.current_beta = gamma_to_beta(self.current_gamma);
        self.current_brho = gamma_to_brho(self.current_gamma);

        // Update game phase
        let gamma_ratio = self.current_gamma / GAMMA_TRANSITION;
        if gamma_ratio < 0.85 {
            self.phase = GamePhase::EarlyRamp;
        } else if gamma_ratio < 0.97 {
            self.phase = GamePhase::PreTransition;
        } else if gamma_ratio < 1.03 {
            self.phase = GamePhase::Transition;
        } else if self.ramp_turn < TURNS_IN_CYCLE - 500 {
            self.phase = GamePhase::PostTransition;
        } else {
            self.phase = GamePhase::Extraction;
        }
    }

    // ── Optics Computation ───────────────────────────────────────────────
    /// Compute energy-dependent focusing strengths and approximate Twiss parameters.
    fn update_optics(&mut self) {
        let brho_ratio = gamma_to_brho(kinetic_to_gamma(E_INJECTION_GEV)) / self.current_brho;

        // K scales as K_injection * (Bρ_inj / Bρ_current) because gradients
        // track the main field in combined-function magnets
        // quad_bus_trim scales all gradients (MQAT)
        let k_f = K1_F_INJECTION * brho_ratio * (1.0 + self.quad_bus_trim);
        let k_d = K1_D_INJECTION * brho_ratio * (1.0 + self.quad_bus_trim);

        // Approximate tune from thin-lens FODO formula:
        // cos(μ) ≈ 1 - L²·K/2
        // For combined function: use full transfer matrix trace
        let mf_x = Matrix2::focusing(k_f, MAGNET_LENGTH);
        let md_x = Matrix2::focusing(-k_d, MAGNET_LENGTH);
        let m_os = Matrix2::drift(SHORT_DRIFT);
        let m_ol = Matrix2::drift(LONG_DRIFT);

        // One cell: F · Os · F · D · OL · D
        let cell_x = mf_x.multiply(&m_os).multiply(&mf_x).multiply(&md_x).multiply(&m_ol).multiply(&md_x);
        let cos_mu_x = (cell_x.m11 + cell_x.m22) / 2.0;

        // Y plane: F is defocusing, D is focusing
        let mf_y = Matrix2::focusing(-k_f, MAGNET_LENGTH);
        let md_y = Matrix2::focusing(k_d, MAGNET_LENGTH);
        let cell_y = mf_y.multiply(&m_os).multiply(&mf_y).multiply(&md_y).multiply(&m_ol).multiply(&md_y);
        let cos_mu_y = (cell_y.m11 + cell_y.m22) / 2.0;

        // Phase advance per cell → tune = N_cells * μ / (2π)
        if cos_mu_x.abs() <= 1.0 {
            let mu_x = cos_mu_x.acos();
            self.tune_x = NUM_SECTIONS as f64 * mu_x / std::f64::consts::TAU;
        }
        if cos_mu_y.abs() <= 1.0 {
            let mu_y = cos_mu_y.acos();
            self.tune_y = NUM_SECTIONS as f64 * mu_y / std::f64::consts::TAU;
        }

        // Apply trim quad corrections to tune
        let trim_quad_sum: f64 = self.correctors.iter().map(|c| c.trim_quad).sum();
        self.tune_x += trim_quad_sum * 0.05; // approximate sensitivity
        self.tune_y -= trim_quad_sum * 0.05;

        // Beta functions: β_max ≈ cell_length * (1 + sin(μ/2)) / sin(μ)
        // Simplified approximation
        if cos_mu_x.abs() < 1.0 {
            let sin_mu_x = (1.0 - cos_mu_x * cos_mu_x).sqrt();
            if sin_mu_x > 0.01 {
                self.beta_x_max = cell_x.m12.abs() / sin_mu_x;
            }
        }
        if cos_mu_y.abs() < 1.0 {
            let sin_mu_y = (1.0 - cos_mu_y * cos_mu_y).sqrt();
            if sin_mu_y > 0.01 {
                self.beta_y_max = cell_y.m12.abs() / sin_mu_y;
            }
        }

        // Dispersion scales with energy roughly as D ∝ 1/(1 - γ²/γ_t²) near transition
        let eta = slip_factor(self.current_gamma);
        self.dispersion_max = if eta.abs() > 0.01 {
            3.2 / (eta.abs() * GAMMA_TRANSITION * GAMMA_TRANSITION)
                .min(50.0)
        } else {
            50.0 // blows up at transition
        };

        // Chromaticity: natural + sextupole correction
        let sext_a_sum: f64 = self.correctors.iter().map(|c| c.sext_a).sum();
        let sext_b_sum: f64 = self.correctors.iter().map(|c| c.sext_b).sum();
        // Natural chromaticity is ~ -1 per unit of tune
        self.chromaticity_x = -self.tune_x + sext_a_sum * 2.0 + sext_b_sum * 1.0;
        self.chromaticity_y = -self.tune_y - sext_a_sum * 1.0 + sext_b_sum * 2.0;

        // Space charge tune shift: ΔQ ∝ N / (ε_n · β · γ²)
        let bg2 = self.current_beta * self.current_gamma * self.current_gamma;
        let emit_factor = if self.initial_emittance_x > 0.0 { self.initial_emittance_x } else { 1.0 };
        self.sc_tune_shift = -0.3 * self.beam_intensity / (emit_factor * bg2);
    }

    // ── Transfer Matrix for one element at current energy ────────────────
    fn element_matrices(&self, elem: &LatticeElement) -> (Matrix2, Matrix2) {
        let brho_ratio = gamma_to_brho(kinetic_to_gamma(E_INJECTION_GEV)) / self.current_brho;

        match elem.elem_type {
            ElementType::FMagnet => {
                let k = K1_F_INJECTION * brho_ratio * (1.0 + self.quad_bus_trim);
                (Matrix2::focusing(k, MAGNET_LENGTH),
                 Matrix2::focusing(-k, MAGNET_LENGTH))
            }
            ElementType::DMagnet => {
                let k = K1_D_INJECTION * brho_ratio * (1.0 + self.quad_bus_trim);
                (Matrix2::focusing(-k, MAGNET_LENGTH),
                 Matrix2::focusing(k, MAGNET_LENGTH))
            }
            ElementType::ShortDrift => {
                let m = Matrix2::drift(SHORT_DRIFT);
                (m, m)
            }
            ElementType::LongDrift => {
                let m = Matrix2::drift(LONG_DRIFT);
                (m, m)
            }
        }
    }

    // ── Apply one lattice element ────────────────────────────────────────
    fn apply_element(&mut self) {
        let global_idx = self.beam_cell * ELEMENTS_PER_CELL + self.beam_element;
        if global_idx >= self.lattice.len() { return; }

        let elem = self.lattice[global_idx].clone();
        let (mx, my) = self.element_matrices(&elem);

        // Dispersion contribution: x_disp = D * δ
        let disp_kick = if matches!(elem.elem_type, ElementType::FMagnet | ElementType::DMagnet) {
            self.dispersion_max * self.beam_dp * 0.1 // scaled dispersion effect
        } else {
            0.0
        };

        // Apply transfer matrix
        let (new_x, new_xp) = mx.apply(self.beam_x + disp_kick, self.beam_xp);
        self.beam_x = new_x;
        self.beam_xp = new_xp;

        let (new_y, new_yp) = my.apply(self.beam_y, self.beam_yp);
        self.beam_y = new_y;
        self.beam_yp = new_yp;

        // Apply main bend bus trim (MDAT) angular kick in dipole elements
        if matches!(elem.elem_type, ElementType::FMagnet | ElementType::DMagnet) {
            let brho_inj = gamma_to_brho(kinetic_to_gamma(E_INJECTION_GEV));
            let brho_scale = brho_inj / self.current_brho;
            self.beam_xp += self.bend_bus_trim * DIPOLE_ANGLE * brho_scale;
        }

        // Apply correctors at long drift (element index 4 in cell)
        if elem.index == 4 {
            let corr = &self.correctors[self.beam_cell];
            // Trim dipoles: angular kicks
            self.beam_xp += corr.h_trim;
            self.beam_yp += corr.v_trim;
            // Trim quad: thin-lens kick proportional to position
            self.beam_xp -= corr.trim_quad * self.beam_x * 0.001;
            self.beam_yp += corr.trim_quad * self.beam_y * 0.001;
            // Skew quad: couples planes
            let x_kick = corr.skew_quad * self.beam_y * 0.001;
            let y_kick = corr.skew_quad * self.beam_x * 0.001;
            self.beam_xp += x_kick;
            self.beam_yp += y_kick;
            // Sextupoles: nonlinear kick ∝ x² (chromaticity correction)
            let sext_kick_x = (corr.sext_a + corr.sext_b) * self.beam_x * self.beam_x * 1e-6;
            self.beam_xp -= sext_kick_x;
        }

        // Beam size evolution: approximate via envelope tracking
        // σ' proportional to β-function variation
        let beta_ratio_x = if self.beta_x_max > 0.1 { 1.0 + 0.01 * (self.beam_x.abs() / self.beta_x_max) } else { 1.0 };
        let beta_ratio_y = if self.beta_y_max > 0.1 { 1.0 + 0.01 * (self.beam_y.abs() / self.beta_y_max) } else { 1.0 };
        self.beam_sigma_x = (self.beam_sigma_x * beta_ratio_x).max(0.5);
        self.beam_sigma_y = (self.beam_sigma_y * beta_ratio_y).max(0.5);
    }

    // ── Longitudinal dynamics (one turn) ─────────────────────────────────
    fn advance_longitudinal(&mut self) {
        let eta = slip_factor(self.current_gamma);
        let total_e_gev = self.current_ke_gev + PROTON_MASS_GEV;

        // Synchrotron equation of motion:
        // Δφ = 2π·h·η·δ   (phase slip per turn)
        // Δδ = eV/(2π·β²·E) · (sin(φ_s + Δφ) - sin(φ_s))
        let phi_s = self.rf_phase_deg.to_radians();
        let v_per_turn = self.rf_voltage_mv * 1e-3; // convert MV to GV

        // Phase update
        self.beam_phi += std::f64::consts::TAU * HARMONIC_NUMBER as f64 * eta * self.beam_dp;

        // Energy kick from RF
        let sin_phi = (phi_s + self.beam_phi).sin();
        let sin_phi_s = phi_s.sin();
        let de_kick = v_per_turn / (std::f64::consts::TAU * self.current_beta * self.current_beta * total_e_gev)
            * (sin_phi - sin_phi_s);
        self.beam_de += de_kick;

        // Update momentum offset from energy deviation
        self.beam_dp = self.beam_de / total_e_gev;

        // Record longitudinal coordinates
        self.phi_history.push(self.beam_phi as f32);
        self.de_history.push(self.beam_de as f32);
        if self.phi_history.len() > MAX_HISTORY {
            self.phi_history.remove(0);
            self.de_history.remove(0);
        }

        // Check for longitudinal beam loss (escaped RF bucket)
        // Bucket half-height ≈ √(2·eV·β²·E / (π·h·|η|))
        let bucket_area = if eta.abs() > 1e-6 {
            (2.0 * v_per_turn * self.current_beta.powi(2) * total_e_gev
             / (std::f64::consts::PI * HARMONIC_NUMBER as f64 * eta.abs())).sqrt()
        } else {
            0.001 // very small bucket at transition
        };

        if self.beam_de.abs() > bucket_area * 3.0 || self.beam_phi.abs() > std::f64::consts::PI {
            self.beam_losses += 2.0; // longitudinal loss
        }
    }

    // ── Transition crossing special handling ─────────────────────────────
    fn handle_transition(&mut self) {
        if self.transition_crossed { return; }

        let gamma_ratio = self.current_gamma / GAMMA_TRANSITION;
        if gamma_ratio > 0.99 && gamma_ratio < 1.01 {
            // At transition: RF phase must flip for stability
            // If player hasn't set chromaticity correctly, large losses occur
            let chrom_quality = (self.chromaticity_x.abs() - 7.0).abs()
                + (self.chromaticity_y.abs() - 7.0).abs();

            // Bunch length oscillation excitation
            let oscillation_amp = 0.5 + chrom_quality * 0.3;
            self.beam_phi += oscillation_amp * 0.1;
            self.beam_de += oscillation_amp * 0.001;

            // Beam size blow-up near transition
            let blowup = 1.0 + chrom_quality * 0.05;
            self.beam_sigma_x *= blowup;
            self.beam_sigma_y *= blowup;

            // Losses proportional to poor chromaticity control
            self.beam_losses += (chrom_quality * 2.0) as f32;
            self.beam_intensity *= (1.0 - chrom_quality * 0.01).max(0.5);

            if gamma_ratio > 1.005 {
                self.transition_crossed = true;
                self.message = Some((
                    format!("Transition crossed! Chrom quality: {:.1}", chrom_quality),
                    90,
                    if chrom_quality < 3.0 { Color::Rgb(80, 255, 80) } else { Color::Rgb(255, 80, 80) },
                ));
            }
        }
    }

    // ── Advance beam through one step ────────────────────────────────────
    fn advance_beam(&mut self) {
        self.beam_progress += 0.35;

        if self.beam_progress >= 1.0 {
            self.beam_progress = 0.0;
            self.apply_element();

            // Scale to display coordinates for loss checking
            let display_x = (self.beam_x * 0.5) as f32; // mm → display units
            let display_y = (self.beam_y * 0.5) as f32;
            let display_sx = (self.beam_sigma_x * 0.5) as f32;
            let display_sy = (self.beam_sigma_y * 0.5) as f32;

            // Hard wall check
            if display_x.abs() > APERTURE_DISPLAY || display_y.abs() > APERTURE_DISPLAY {
                self.beam_lost = true;
                self.phase = GamePhase::Lost;
                self.message = Some(("Hit aperture wall!".to_string(), 60, Color::Rgb(255, 60, 60)));
                return;
            }

            // Loss zone accumulation
            let x_edge_pos = display_x + display_sx * 0.5;
            let x_edge_neg = display_x - display_sx * 0.5;
            let y_edge_pos = display_y + display_sy * 0.5;
            let y_edge_neg = display_y - display_sy * 0.5;
            let mut loss_this_step = 0.0_f32;
            if x_edge_pos > LOSS_ZONE { loss_this_step += (x_edge_pos - LOSS_ZONE) * 0.3; }
            if x_edge_neg < -LOSS_ZONE { loss_this_step += (-x_edge_neg - LOSS_ZONE) * 0.3; }
            if y_edge_pos > LOSS_ZONE { loss_this_step += (y_edge_pos - LOSS_ZONE) * 0.3; }
            if y_edge_neg < -LOSS_ZONE { loss_this_step += (-y_edge_neg - LOSS_ZONE) * 0.3; }
            if loss_this_step > 0.0 {
                self.beam_losses += loss_this_step;
                self.beam_intensity *= (1.0 - loss_this_step as f64 * 0.001).max(0.0);
            }

            if self.beam_losses >= MAX_LOSSES {
                self.beam_lost = true;
                self.phase = GamePhase::Lost;
                self.message = Some((
                    format!("Beam losses exceeded {:.0}!", MAX_LOSSES),
                    60, Color::Rgb(255, 100, 100),
                ));
                return;
            }

            // Advance to next element
            self.beam_element += 1;
            if self.beam_element >= ELEMENTS_PER_CELL {
                self.beam_element = 0;

                // Record trail
                self.trail.push((self.beam_cell, display_x, display_sx));
                if self.trail.len() > NUM_SECTIONS * 3 {
                    self.trail.remove(0);
                }

                self.beam_cell += 1;
                if self.beam_cell >= NUM_SECTIONS {
                    self.beam_cell = 0;
                    self.turns_completed += 1;
                    self.ramp_turn += 1;

                    // Record turn position
                    self.turn_positions.push((display_x, display_y));
                    if self.turn_positions.len() > 20 {
                        self.turn_positions.remove(0);
                    }

                    // Record phase space history
                    self.x_xp_history.push((self.beam_x as f32, self.beam_xp as f32));
                    self.y_yp_history.push((self.beam_y as f32, self.beam_yp as f32));
                    if self.x_xp_history.len() > MAX_HISTORY {
                        self.x_xp_history.remove(0);
                    }
                    if self.y_yp_history.len() > MAX_HISTORY {
                        self.y_yp_history.remove(0);
                    }

                    if self.turns_completed > self.best_turns {
                        self.best_turns = self.turns_completed;
                    }

                    // Update energy each turn
                    self.update_energy_for_turn();
                    self.update_optics();
                    self.advance_longitudinal();
                    self.handle_transition();

                    // Check extraction
                    if self.ramp_turn >= TURNS_IN_CYCLE {
                        self.phase = GamePhase::Extraction;
                        self.beam_running = false;
                    }
                }
            }
        }
    }

    // ── Helper methods ───────────────────────────────────────────────────

    fn stability_score(&self) -> f32 {
        if self.pos_history.is_empty() { return 0.0; }
        let avg_pos: f32 = self.pos_history.iter().map(|p| p.abs()).sum::<f32>() / self.pos_history.len() as f32;
        let avg_size: f32 = self.size_history.iter().sum::<f32>() / self.size_history.len().max(1) as f32;
        let avg_y_pos: f32 = self.y_pos_history.iter().map(|p| p.abs()).sum::<f32>() / self.y_pos_history.len().max(1) as f32;
        let avg_y_size: f32 = self.y_size_history.iter().sum::<f32>() / self.y_size_history.len().max(1) as f32;
        let x_pos_score = (1.0 - avg_pos / APERTURE_DISPLAY).max(0.0);
        let x_size_score = (1.0 - avg_size / APERTURE_DISPLAY).max(0.0);
        let y_pos_score = (1.0 - avg_y_pos / APERTURE_DISPLAY).max(0.0);
        let y_size_score = (1.0 - avg_y_size / APERTURE_DISPLAY).max(0.0);
        let x_score = x_pos_score * 0.6 + x_size_score * 0.4;
        let y_score = y_pos_score * 0.6 + y_size_score * 0.4;
        ((x_score + y_score) * 0.5) * 100.0
    }

    fn adjust_corrector(&mut self, cell: usize, corr_type: CorrectorSelect, delta: f64) {
        let corr = &mut self.correctors[cell];
        match corr_type {
            CorrectorSelect::HTrim => corr.h_trim += delta,
            CorrectorSelect::VTrim => corr.v_trim += delta,
            CorrectorSelect::TrimQuad => corr.trim_quad += delta,
            CorrectorSelect::SkewQuad => corr.skew_quad += delta,
            CorrectorSelect::SextA => corr.sext_a += delta,
            CorrectorSelect::SextB => corr.sext_b += delta,
        }
    }

    fn copy_correctors_to_all(&mut self) {
        let src = self.correctors[self.selected_cell].clone();
        for i in 0..NUM_SECTIONS {
            if i != self.selected_cell {
                self.correctors[i] = src.clone();
            }
        }
        self.message = Some((
            format!("Copied cell {} correctors to all!", self.selected_cell + 1),
            45, Color::Rgb(80, 255, 180),
        ));
    }

    fn energy_fraction(&self) -> f64 {
        (self.current_ke_gev - E_INJECTION_GEV) / (E_EXTRACTION_GEV - E_INJECTION_GEV)
    }

    fn transition_fraction(&self) -> f64 {
        let gamma_inj = kinetic_to_gamma(E_INJECTION_GEV);
        (self.current_gamma - gamma_inj) / (GAMMA_TRANSITION - gamma_inj)
    }
}

// ── Game Trait Implementation ────────────────────────────────────────────────
impl Game for BoosterGame {
    fn update(&mut self) {
        // Tick message timer
        if let Some((_, ref mut ticks, _)) = self.message {
            if *ticks > 0 { *ticks -= 1; } else { self.message = None; }
        }
        if self.paused || self.beam_lost || self.phase == GamePhase::Extraction { return; }
        self.tick += 1;
        if self.beam_running {
            for _ in 0..self.sim_speed.steps_per_tick() {
                if self.beam_lost || self.phase == GamePhase::Extraction { break; }
                self.advance_beam();
            }
            // Record display history
            if self.tick % 3 == 0 {
                let dx = (self.beam_x * 0.5) as f32;
                let dy = (self.beam_y * 0.5) as f32;
                let sx = (self.beam_sigma_x * 0.5) as f32;
                let sy = (self.beam_sigma_y * 0.5) as f32;
                self.pos_history.push(dx);
                self.size_history.push(sx);
                self.y_pos_history.push(dy);
                self.y_size_history.push(sy);
                if self.pos_history.len() > MAX_HISTORY { self.pos_history.remove(0); }
                if self.size_history.len() > MAX_HISTORY { self.size_history.remove(0); }
                if self.y_pos_history.len() > MAX_HISTORY { self.y_pos_history.remove(0); }
                if self.y_size_history.len() > MAX_HISTORY { self.y_size_history.remove(0); }
            }
        }
    }

    fn handle_input(&mut self, key: KeyEvent) {
        // ── Input mode: intercept all keys for coordinate entry ──
        if self.input_mode != InputMode::None {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::None;
                    self.input_buffer.clear();
                    self.message = Some(("Inject cancelled".to_string(), 30, Color::Rgb(140, 140, 160)));
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Enter => {
                    match self.input_buffer.parse::<f64>() {
                        Ok(val) => {
                            if self.input_mode == InputMode::InjectX {
                                self.inject_x = val;
                                self.input_mode = InputMode::InjectY;
                                self.input_buffer.clear();
                            } else {
                                // InjectY — perform injection
                                self.inject_y = val;
                                self.input_mode = InputMode::None;
                                self.input_buffer.clear();
                                // Inject beam at (inject_x, inject_y)
                                self.beam_running = true;
                                self.phase = GamePhase::Injection;
                                self.beam_x = self.inject_x;
                                self.beam_xp = 0.0;
                                self.beam_y = self.inject_y;
                                self.beam_yp = 0.0;
                                self.beam_dp = 0.0;
                                self.beam_phi = 0.0;
                                self.beam_de = 0.0;
                                self.beam_cell = 0;
                                self.beam_element = 0;
                                self.beam_progress = 0.0;
                                self.beam_losses = 0.0;
                                self.ramp_turn = 0;
                                self.transition_crossed = false;
                                self.current_ke_gev = E_INJECTION_GEV;
                                self.current_gamma = kinetic_to_gamma(E_INJECTION_GEV);
                                self.current_beta = gamma_to_beta(self.current_gamma);
                                self.current_brho = gamma_to_brho(self.current_gamma);
                                self.beam_intensity = 1.0;
                                self.update_optics();
                                self.trail.clear();
                                self.pos_history.clear();
                                self.size_history.clear();
                                self.y_pos_history.clear();
                                self.y_size_history.clear();
                                self.phi_history.clear();
                                self.de_history.clear();
                                self.x_xp_history.clear();
                                self.y_yp_history.clear();
                                self.message = Some((
                                    format!("Injected at x={:.1} y={:.1} mm", self.inject_x, self.inject_y),
                                    60, Color::Rgb(80, 200, 255),
                                ));
                            }
                        }
                        Err(_) => {
                            let label = if self.input_mode == InputMode::InjectX { "X" } else { "Y" };
                            self.message = Some((
                                format!("Invalid {}: '{}'", label, self.input_buffer),
                                45, Color::Rgb(255, 80, 80),
                            ));
                            self.input_buffer.clear();
                        }
                    }
                }
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' => {
                    self.input_buffer.push(c);
                }
                _ => {} // ignore other keys in input mode
            }
            return;
        }

        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => self.reset(),
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if !self.beam_lost && self.phase != GamePhase::Extraction {
                    self.paused = !self.paused;
                }
            }
            _ => {
                if self.beam_lost || self.phase == GamePhase::Extraction {
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
                            self.phase = GamePhase::Injection;
                            self.beam_x = 0.0;
                            self.beam_xp = 0.0;
                            self.beam_y = 0.0;
                            self.beam_yp = 0.0;
                            self.beam_dp = 0.0;
                            self.beam_phi = 0.0;
                            self.beam_de = 0.0;
                            self.beam_cell = 0;
                            self.beam_element = 0;
                            self.beam_progress = 0.0;
                            self.beam_losses = 0.0;
                            self.ramp_turn = 0;
                            self.transition_crossed = false;
                            self.current_ke_gev = E_INJECTION_GEV;
                            self.current_gamma = kinetic_to_gamma(E_INJECTION_GEV);
                            self.current_beta = gamma_to_beta(self.current_gamma);
                            self.current_brho = gamma_to_brho(self.current_gamma);
                            self.beam_intensity = 1.0;
                            self.update_optics();
                            self.trail.clear();
                            self.pos_history.clear();
                            self.size_history.clear();
                            self.y_pos_history.clear();
                            self.y_size_history.clear();
                            self.phi_history.clear();
                            self.de_history.clear();
                            self.x_xp_history.clear();
                            self.y_yp_history.clear();
                            self.message = Some(("Beam injected at 400 MeV!".to_string(), 60, Color::Rgb(80, 200, 255)));
                        }
                    }
                    // Injection with coordinate input
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        if !self.beam_running {
                            self.input_mode = InputMode::InjectX;
                            self.input_buffer.clear();
                        }
                    }
                    // Navigate cells
                    KeyCode::Char(']') => {
                        if self.bump.is_none() {
                            self.selected_cell = (self.selected_cell + 1) % NUM_SECTIONS;
                        }
                    }
                    KeyCode::Char('[') => {
                        if self.bump.is_none() {
                            self.selected_cell = if self.selected_cell == 0 { NUM_SECTIONS - 1 } else { self.selected_cell - 1 };
                        }
                    }
                    // Navigate corrector types
                    KeyCode::Up => {
                        if self.bump.is_some() {
                            // Bump mode: adjust trim correctors up
                            if let Some(ref bump) = self.bump {
                                let sec_coeffs = bump.section_coefficients();
                                let speed = self.adjust_speed;
                                for (sec, coeff) in &sec_coeffs {
                                    let corr = &mut self.correctors[*sec];
                                    corr.h_trim += speed * coeff;
                                    corr.v_trim += speed * coeff;
                                }
                            }
                        } else {
                            self.selected_corrector = self.selected_corrector.prev();
                        }
                    }
                    KeyCode::Down => {
                        if self.bump.is_some() {
                            if let Some(ref bump) = self.bump {
                                let sec_coeffs = bump.section_coefficients();
                                let speed = self.adjust_speed;
                                for (sec, coeff) in &sec_coeffs {
                                    let corr = &mut self.correctors[*sec];
                                    corr.h_trim -= speed * coeff;
                                    corr.v_trim -= speed * coeff;
                                }
                            }
                        } else {
                            self.selected_corrector = self.selected_corrector.next();
                        }
                    }
                    // Adjust corrector power
                    KeyCode::Left => {
                        if self.bump.is_some() {
                            if let Some(ref mut bump) = self.bump {
                                bump.start_section = if bump.start_section == 0 {
                                    NUM_SECTIONS - 1
                                } else {
                                    bump.start_section - 1
                                };
                            }
                        } else {
                            let cell = self.selected_cell;
                            let ct = self.selected_corrector;
                            let spd = self.adjust_speed;
                            self.adjust_corrector(cell, ct, -spd);
                        }
                    }
                    KeyCode::Right => {
                        if self.bump.is_some() {
                            if let Some(ref mut bump) = self.bump {
                                bump.start_section = (bump.start_section + 1) % NUM_SECTIONS;
                            }
                        } else {
                            let cell = self.selected_cell;
                            let ct = self.selected_corrector;
                            let spd = self.adjust_speed;
                            self.adjust_corrector(cell, ct, spd);
                        }
                    }
                    // Bump mode W/S: H-trim only
                    KeyCode::Char('w') | KeyCode::Char('W') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                self.correctors[*sec].h_trim += speed * coeff;
                            }
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                self.correctors[*sec].h_trim -= speed * coeff;
                            }
                        }
                    }
                    // Bump mode E/Q: V-trim only
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                self.correctors[*sec].v_trim += speed * coeff;
                            }
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            let speed = self.adjust_speed;
                            for (sec, coeff) in &sec_coeffs {
                                self.correctors[*sec].v_trim -= speed * coeff;
                            }
                        }
                    }
                    // RF controls: F/G for voltage, V for phase
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        self.rf_voltage_mv = (self.rf_voltage_mv + 0.02).min(MAX_RF_VOLTAGE_MV);
                        self.message = Some((
                            format!("RF V: {:.2} MV", self.rf_voltage_mv), 30, Color::Rgb(255, 200, 80),
                        ));
                    }
                    KeyCode::Char('g') | KeyCode::Char('G') => {
                        self.rf_voltage_mv = (self.rf_voltage_mv - 0.02).max(0.0);
                        self.message = Some((
                            format!("RF V: {:.2} MV", self.rf_voltage_mv), 30, Color::Rgb(255, 200, 80),
                        ));
                    }
                    KeyCode::Char('v') | KeyCode::Char('V') => {
                        // Cycle display mode (View)
                        self.display_mode = self.display_mode.next();
                    }
                    // Step size
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        self.adjust_speed = (self.adjust_speed * 2.0).min(1.0);
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        self.adjust_speed = (self.adjust_speed * 0.5).max(0.0001);
                    }
                    // Copy correctors
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.copy_correctors_to_all();
                    }
                    // Zero current corrector
                    KeyCode::Char('z') | KeyCode::Char('Z') => {
                        if let Some(ref bump) = self.bump {
                            let sec_coeffs = bump.section_coefficients();
                            for (sec, _) in &sec_coeffs {
                                self.correctors[*sec].h_trim = 0.0;
                                self.correctors[*sec].v_trim = 0.0;
                            }
                            self.message = Some(("Zeroed bump trims".to_string(), 30, Color::Rgb(255, 200, 80)));
                        } else {
                            let cell = self.selected_cell;
                            let ct = self.selected_corrector;
                            self.adjust_corrector(cell, ct, 0.0);
                            // Actually zero it
                            let corr = &mut self.correctors[cell];
                            match ct {
                                CorrectorSelect::HTrim => corr.h_trim = 0.0,
                                CorrectorSelect::VTrim => corr.v_trim = 0.0,
                                CorrectorSelect::TrimQuad => corr.trim_quad = 0.0,
                                CorrectorSelect::SkewQuad => corr.skew_quad = 0.0,
                                CorrectorSelect::SextA => corr.sext_a = 0.0,
                                CorrectorSelect::SextB => corr.sext_b = 0.0,
                            }
                        }
                    }
                    // Toggle RF phase for transition crossing
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        if self.rf_phase_deg < 90.0 {
                            self.rf_phase_deg = 180.0 - self.rf_phase_deg;
                        } else {
                            self.rf_phase_deg = 180.0 - self.rf_phase_deg;
                        }
                        self.message = Some((
                            format!("RF phase: {:.0} deg", self.rf_phase_deg), 30, Color::Rgb(200, 180, 255),
                        ));
                    }
                    // Quad bus trim (MQAT)
                    KeyCode::Char('j') | KeyCode::Char('J') => {
                        self.quad_bus_trim = (self.quad_bus_trim + self.adjust_speed).min(0.2);
                        self.message = Some((
                            format!("MQAT: {:+.4}", self.quad_bus_trim), 30, Color::Rgb(120, 200, 255),
                        ));
                    }
                    KeyCode::Char('k') | KeyCode::Char('K') => {
                        self.quad_bus_trim = (self.quad_bus_trim - self.adjust_speed).max(-0.2);
                        self.message = Some((
                            format!("MQAT: {:+.4}", self.quad_bus_trim), 30, Color::Rgb(120, 200, 255),
                        ));
                    }
                    // Main bend bus trim (MDAT)
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        self.bend_bus_trim = (self.bend_bus_trim + self.adjust_speed).min(0.1);
                        self.message = Some((
                            format!("MDAT: {:+.4}", self.bend_bus_trim), 30, Color::Rgb(255, 180, 120),
                        ));
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        self.bend_bus_trim = (self.bend_bus_trim - self.adjust_speed).max(-0.1);
                        self.message = Some((
                            format!("MDAT: {:+.4}", self.bend_bus_trim), 30, Color::Rgb(255, 180, 120),
                        ));
                    }
                    // Bump mode toggle
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        if let Some(ref bump) = self.bump {
                            let start = bump.start_section;
                            match bump.size {
                                3 => {
                                    self.bump = Some(BumpConfig::new(4, start));
                                    self.message = Some((format!("4-Bump mode"), 45, Color::Rgb(80, 255, 200)));
                                }
                                4 => {
                                    self.bump = Some(BumpConfig::new(5, start));
                                    self.message = Some((format!("5-Bump mode"), 45, Color::Rgb(80, 255, 200)));
                                }
                                _ => {
                                    self.bump = None;
                                    self.message = Some(("Bump mode OFF".to_string(), 30, Color::Rgb(140, 140, 160)));
                                }
                            }
                        } else {
                            let start = self.selected_cell;
                            self.bump = Some(BumpConfig::new(3, start));
                            self.message = Some((format!("3-Bump mode"), 45, Color::Rgb(80, 255, 200)));
                        }
                    }
                    // Simulation speed
                    KeyCode::Char('.') | KeyCode::Char('>') => {
                        self.sim_speed = self.sim_speed.next();
                        let desc = match self.sim_speed {
                            SimSpeed::Slow => "0.25 rev/s",
                            SimSpeed::Normal => "1 rev/s",
                            SimSpeed::Fast => "10 rev/s",
                        };
                        self.message = Some((
                            format!("Speed: {} ({})", self.sim_speed.label(), desc),
                            30, Color::Rgb(255, 255, 100),
                        ));
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
            .title(" Fermilab Booster ")
            .title_style(Style::default().fg(Color::Rgb(120, 200, 255)).add_modifier(Modifier::BOLD));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // Status + energy
                Constraint::Length(2),  // Beam X display bar
                Constraint::Length(2),  // Beam Y display bar
                Constraint::Min(8),    // Middle: controls + ring
                Constraint::Length(2),  // Help
            ])
            .split(inner);

        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(32), // Control panel
                Constraint::Min(20),   // Ring visualization
            ])
            .split(chunks[3]);

        // ── Status Bar (2 lines) ─────────────────────────────────────────
        let stability = self.stability_score();
        let stab_color = if stability > 80.0 { Color::Rgb(80, 255, 80) }
            else if stability > 50.0 { Color::Yellow }
            else if stability > 20.0 { Color::Rgb(255, 160, 50) }
            else { Color::Rgb(255, 60, 60) };

        let energy_bar_w = 20;
        let energy_frac = self.energy_fraction();
        let energy_filled = (energy_frac * energy_bar_w as f64) as usize;
        let mut energy_bar = String::new();
        for i in 0..energy_bar_w {
            if i == (self.transition_fraction().clamp(0.0, 1.0) * energy_bar_w as f64) as usize {
                energy_bar.push('|'); // transition marker
            } else if i < energy_filled {
                energy_bar.push('=');
            } else {
                energy_bar.push('-');
            }
        }

        let eta = slip_factor(self.current_gamma);
        let status_line1 = Line::from(vec![
            Span::styled(
                format!("[{}] ", self.phase.label()),
                Style::default().fg(self.phase.color()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("E:{:.2}GeV ", self.current_ke_gev),
                Style::default().fg(Color::Rgb(255, 200, 80)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}] ", energy_bar),
                Style::default().fg(if self.phase == GamePhase::Transition { Color::Red } else { Color::Rgb(80, 180, 80) }),
            ),
            Span::styled(
                format!("Turn:{}/{} ", self.ramp_turn, TURNS_IN_CYCLE),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("I:{:.0}% ", self.beam_intensity * 100.0),
                Style::default().fg(if self.beam_intensity > 0.8 { Color::Green } else { Color::Red }),
            ),
        ]);

        let status_line2_spans = vec![
            Span::styled(
                format!("Qx:{:.3} Qy:{:.3} ", self.tune_x + self.sc_tune_shift, self.tune_y + self.sc_tune_shift),
                Style::default().fg(Color::Rgb(120, 200, 255)),
            ),
            Span::styled(
                format!("eta:{:+.4} ", eta),
                Style::default().fg(if eta.abs() < 0.01 { Color::Red } else { Color::Rgb(140, 140, 160) }),
            ),
            Span::styled(
                format!("Cx:{:.1} Cy:{:.1} ", self.chromaticity_x, self.chromaticity_y),
                Style::default().fg(Color::Rgb(255, 120, 180)),
            ),
            Span::styled(
                format!("RF:{:.2}MV/{:.0}deg ", self.rf_voltage_mv, self.rf_phase_deg),
                Style::default().fg(Color::Rgb(255, 200, 80)),
            ),
            Span::styled(
                format!("Stab:{:.0}% ", stability),
                Style::default().fg(stab_color),
            ),
            Span::styled(
                format!("MDAT:{:+.3} ", self.bend_bus_trim),
                Style::default().fg(Color::Rgb(255, 180, 120)),
            ),
            Span::styled(
                format!("MQAT:{:+.3} ", self.quad_bus_trim),
                Style::default().fg(Color::Rgb(120, 200, 255)),
            ),
            Span::styled(
                format!("[{}] ", self.display_mode.label()),
                Style::default().fg(Color::Rgb(200, 200, 100)),
            ),
        ];
        // Append flash message if active
        let mut line2_spans = status_line2_spans;
        if let Some((ref msg, ticks, color)) = self.message {
            if ticks > 0 {
                line2_spans.push(Span::styled(format!(" {} ", msg), Style::default().fg(color).add_modifier(Modifier::BOLD)));
            }
        }
        let status_line2 = Line::from(line2_spans);

        frame.render_widget(Paragraph::new(vec![status_line1, status_line2]), chunks[0]);

        // ── Beam X Position Bar ──────────────────────────────────────────
        let display_x = (self.beam_x * 0.5) as f32;
        let display_sx = (self.beam_sigma_x * 0.5) as f32;
        self.render_beam_bar(frame, chunks[1], display_x, display_sx, "X",
            Color::Rgb(80, 200, 255), Color::Rgb(10, 60, 80));

        // ── Beam Y Position Bar ──────────────────────────────────────────
        let display_y = (self.beam_y * 0.5) as f32;
        let display_sy = (self.beam_sigma_y * 0.5) as f32;
        self.render_beam_bar(frame, chunks[2], display_y, display_sy, "Y",
            Color::Rgb(200, 120, 255), Color::Rgb(60, 10, 80));

        // ── Left Panel: orbit plot + corrector control ───────────────────
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(8),     // Orbit plot / phase space
                Constraint::Length(14), // Corrector control
            ])
            .split(middle[0]);

        match self.display_mode {
            DisplayMode::Orbit => self.render_orbit_plot(frame, left_chunks[0]),
            DisplayMode::PhaseSpaceX => self.render_phase_space_x(frame, left_chunks[0]),
            DisplayMode::PhaseSpaceY => self.render_phase_space_y(frame, left_chunks[0]),
            DisplayMode::Longitudinal => self.render_longitudinal_plot(frame, left_chunks[0]),
            DisplayMode::TuneDiagram => self.render_tune_diagram(frame, left_chunks[0]),
        }
        self.render_corrector_panel(frame, left_chunks[1]);

        // ── Ring Visualization ───────────────────────────────────────────
        self.render_ring(frame, middle[1]);

        // ── Help Bar ─────────────────────────────────────────────────────
        self.render_help_bar(frame, chunks[4]);
    }

    fn get_score(&self) -> u32 {
        // Score: intensity survival * turns * emittance preservation
        let intensity_score = (self.beam_intensity * 1000.0) as u32;
        let turn_score = self.ramp_turn;
        let transition_bonus = if self.transition_crossed { 500 } else { 0 };
        let extraction_bonus = if self.phase == GamePhase::Extraction { 2000 } else { 0 };
        intensity_score + turn_score + transition_bonus + extraction_bonus
    }

    fn is_game_over(&self) -> bool {
        self.phase == GamePhase::Extraction
    }

    fn reset(&mut self) {
        let best = self.best_turns;
        let correctors = self.correctors.clone();
        let selected_cell = self.selected_cell;
        let selected_corrector = self.selected_corrector;
        let adjust_speed = self.adjust_speed;
        let bump = self.bump.clone();
        let rf_voltage = self.rf_voltage_mv;
        let rf_phase = self.rf_phase_deg;
        let display_mode = self.display_mode;
        let sim_speed = self.sim_speed;
        let bend_bus_trim = self.bend_bus_trim;
        let quad_bus_trim = self.quad_bus_trim;
        *self = BoosterGame::new();
        self.best_turns = best;
        self.correctors = correctors;
        self.selected_cell = selected_cell;
        self.selected_corrector = selected_corrector;
        self.adjust_speed = adjust_speed;
        self.bump = bump;
        self.rf_voltage_mv = rf_voltage;
        self.rf_phase_deg = rf_phase;
        self.display_mode = display_mode;
        self.sim_speed = sim_speed;
        self.bend_bus_trim = bend_bus_trim;
        self.quad_bus_trim = quad_bus_trim;
    }
}

// ── Rendering Helpers ────────────────────────────────────────────────────────
impl BoosterGame {
    fn render_beam_bar(&self, frame: &mut Frame, area: Rect, pos: f32, size: f32, label: &str,
                       beam_color: Color, _bg_hint: Color) {
        let bar_w = area.width as usize;
        let center = bar_w / 2;
        let scale = center as f32 / APERTURE_DISPLAY;
        let mut bar_chars: Vec<(char, Style)> = vec![(' ', Style::default().bg(Color::Rgb(15, 15, 25))); bar_w];

        // Aperture limits
        let left_ap = center.saturating_sub((APERTURE_DISPLAY * scale) as usize);
        let right_ap = (center + (APERTURE_DISPLAY * scale) as usize).min(bar_w.saturating_sub(1));
        if left_ap < bar_w { bar_chars[left_ap] = ('|', Style::default().fg(Color::Red).bg(Color::Rgb(15, 15, 25))); }
        if right_ap < bar_w { bar_chars[right_ap] = ('|', Style::default().fg(Color::Red).bg(Color::Rgb(15, 15, 25))); }

        // Loss zone
        let left_lz = center.saturating_sub((LOSS_ZONE * scale) as usize);
        let right_lz = (center + (LOSS_ZONE * scale) as usize).min(bar_w.saturating_sub(1));
        if left_lz < bar_w && bar_chars[left_lz].0 == ' ' {
            bar_chars[left_lz] = (':', Style::default().fg(Color::Rgb(255, 200, 50)).bg(Color::Rgb(15, 15, 25)));
        }
        if right_lz < bar_w && bar_chars[right_lz].0 == ' ' {
            bar_chars[right_lz] = (':', Style::default().fg(Color::Rgb(255, 200, 50)).bg(Color::Rgb(15, 15, 25)));
        }

        // Draw beam
        if self.beam_running && !self.beam_lost {
            let beam_center = ((center as f32) + pos * scale) as usize;
            let beam_half = (size * scale * 0.5) as usize;
            let bstart = beam_center.saturating_sub(beam_half);
            let bend = (beam_center + beam_half).min(bar_w);
            for x in bstart..bend {
                if x < bar_w {
                    let dist = (x as f32 - beam_center as f32).abs();
                    let intensity = 1.0 - dist / (beam_half as f32 + 1.0);
                    let ch = if intensity > 0.8 { '#' } else if intensity > 0.6 { '=' } else if intensity > 0.35 { '-' } else { '.' };
                    let (r, g, b) = match beam_color {
                        Color::Rgb(br, bg, bb) => {
                            ((br as f32 * intensity) as u8, (bg as f32 * intensity) as u8, (bb as f32 * intensity) as u8)
                        }
                        _ => (100, 200, 255),
                    };
                    bar_chars[x] = (ch, Style::default().fg(Color::Rgb(r, g, b)).bg(Color::Rgb(15, 15, 25)));
                }
            }
            if beam_center < bar_w {
                bar_chars[beam_center] = ('#', Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(15, 15, 25)));
            }
        }

        // Center mark
        if bar_chars[center].0 == ' ' {
            bar_chars[center] = ('.', Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(15, 15, 25)));
        }

        let pos_color = if pos.abs() > 30.0 { Color::Red } else { Color::Green };
        let label_line = Line::from(vec![
            Span::styled(format!(" Beam {}: ", label), Style::default().fg(Color::Rgb(100, 100, 140))),
            Span::styled(format!("{:+.1}mm", pos), Style::default().fg(pos_color).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" sig:{:.1}", size), Style::default().fg(Color::Rgb(140, 140, 160))),
        ]);
        let bar_spans: Vec<Span> = bar_chars.iter().map(|(ch, s)| Span::styled(String::from(*ch), *s)).collect();
        let bar_line = Line::from(bar_spans);
        frame.render_widget(Paragraph::new(vec![label_line, bar_line]), area);
    }

    fn render_orbit_plot(&self, frame: &mut Frame, area: Rect) {
        let bull_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(80, 80, 120)))
            .title(format!(" Orbit ({:.1},{:.1}) ", self.beam_x, self.beam_y))
            .title_style(Style::default().fg(Color::Rgb(255, 200, 80)));
        let bull_inner = bull_block.inner(area);
        frame.render_widget(bull_block, area);

        let bw = bull_inner.width as usize;
        let bh = bull_inner.height as usize;
        if bw < 3 || bh < 3 { return; }

        let plot_range = 20.0_f32;
        let bcx = bw as f32 / 2.0;
        let bcy = bh as f32 / 2.0;
        let sx = bcx / plot_range;
        let sy = bcy / plot_range;

        let mut bgrid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(10, 10, 18))); bw]; bh];

        // Crosshair
        let cx_i = bcx as usize;
        let cy_i = bcy as usize;
        for x in 0..bw {
            if cy_i < bh { bgrid[cy_i][x] = ('-', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        for y in 0..bh {
            if cx_i < bw { bgrid[y][cx_i] = ('|', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        if cx_i < bw && cy_i < bh {
            bgrid[cy_i][cx_i] = ('+', Style::default().fg(Color::Rgb(30, 30, 50)).bg(Color::Rgb(10, 10, 18)));
        }
        draw_plot_ticks(&mut bgrid, bw, bh, bcx, bcy, sx, sy, plot_range, plot_range);

        // Turn positions
        let n = self.turn_positions.len();
        for (i, &(px, py)) in self.turn_positions.iter().enumerate() {
            let dot_x = (bcx + px * sx) as usize;
            let dot_y = (bcy - py * sy) as usize;
            if dot_x < bw && dot_y < bh {
                let brightness = (1.0 - (n - 1 - i) as f32 / 12.0).max(0.3);
                let dist = (px * px + py * py).sqrt();
                let color = if dist < 2.0 {
                    Color::Rgb((80.0 * brightness) as u8, (255.0 * brightness) as u8, (80.0 * brightness) as u8)
                } else if dist < 8.0 {
                    Color::Rgb((255.0 * brightness) as u8, (255.0 * brightness) as u8, (50.0 * brightness) as u8)
                } else {
                    Color::Rgb((255.0 * brightness) as u8, (60.0 * brightness) as u8, (60.0 * brightness) as u8)
                };
                bgrid[dot_y][dot_x] = ('o', Style::default().fg(color).bg(Color::Rgb(10, 10, 18)));
            }
        }

        // Current position
        if self.beam_running && !self.beam_lost {
            let cur_x = (bcx + (self.beam_x * 0.5) as f32 * sx) as usize;
            let cur_y = (bcy - (self.beam_y * 0.5) as f32 * sy) as usize;
            if cur_x < bw && cur_y < bh {
                bgrid[cur_y][cur_x] = ('*', Style::default().fg(Color::Rgb(100, 255, 255)).bg(Color::Rgb(10, 10, 18)).add_modifier(Modifier::BOLD));
            }
        }

        let lines: Vec<Line> = bgrid.into_iter()
            .map(|row| Line::from(row.into_iter().map(|(ch, s)| Span::styled(String::from(ch), s)).collect::<Vec<_>>()))
            .collect();
        frame.render_widget(Paragraph::new(lines), bull_inner);
    }

    fn render_phase_space_x(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(120, 80, 40)))
            .title(format!(" X-X' ({:.1},{:.1}) ", self.beam_x, self.beam_xp))
            .title_style(Style::default().fg(Color::Rgb(255, 180, 120)));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let bw = inner.width as usize;
        let bh = inner.height as usize;
        if bw < 3 || bh < 3 { return; }

        let plot_range_x = 30.0_f32; // mm
        let plot_range_xp = 10.0_f32; // mrad
        let bcx = bw as f32 / 2.0;
        let bcy = bh as f32 / 2.0;
        let sx = bcx / plot_range_x;
        let sy = bcy / plot_range_xp;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(10, 10, 18))); bw]; bh];

        // Crosshair axes
        let cx_i = bcx as usize;
        let cy_i = bcy as usize;
        for x in 0..bw {
            if cy_i < bh { grid[cy_i][x] = ('-', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        for y in 0..bh {
            if cx_i < bw { grid[y][cx_i] = ('|', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        if cx_i < bw && cy_i < bh {
            grid[cy_i][cx_i] = ('+', Style::default().fg(Color::Rgb(30, 30, 50)).bg(Color::Rgb(10, 10, 18)));
        }
        draw_plot_ticks(&mut grid, bw, bh, bcx, bcy, sx, sy, plot_range_x, plot_range_xp);

        // Draw approximate Courant-Snyder ellipse boundary
        let bg = self.current_beta * self.current_gamma;
        let emit = if bg > 0.01 { EMITTANCE_NORM_95 / bg / 6.0 } else { 1.0 };
        let beta_tw = self.beta_x_max;
        if beta_tw > 0.1 && emit > 0.0 {
            let x_max = (emit * beta_tw).sqrt() as f32;
            let xp_max = (emit / beta_tw).sqrt() as f32;
            let steps = 60;
            for i in 0..steps {
                let theta = (i as f32 / steps as f32) * std::f32::consts::TAU;
                let ex = x_max * theta.cos();
                let exp = xp_max * theta.sin();
                let px = (bcx + ex * sx) as usize;
                let py = (bcy - exp * sy) as usize;
                if px < bw && py < bh && grid[py][px].0 == ' ' || (px < bw && py < bh && grid[py][px].0 == '-') || (px < bw && py < bh && grid[py][px].0 == '|') {
                    grid[py][px] = ('.', Style::default().fg(Color::Rgb(80, 50, 20)).bg(Color::Rgb(10, 10, 18)));
                }
            }
        }

        // History dots (fading)
        let n = self.x_xp_history.len();
        for (i, &(hx, hxp)) in self.x_xp_history.iter().enumerate() {
            let px = (bcx + hx * sx) as usize;
            let py = (bcy - hxp * sy) as usize;
            if px < bw && py < bh {
                let brightness = (0.3 + 0.7 * (i as f32 / n.max(1) as f32)).min(1.0);
                let r = (255.0 * brightness) as u8;
                let g = (140.0 * brightness) as u8;
                let b = (50.0 * brightness) as u8;
                grid[py][px] = ('o', Style::default().fg(Color::Rgb(r, g, b)).bg(Color::Rgb(10, 10, 18)));
            }
        }

        // Current position
        if self.beam_running && !self.beam_lost {
            let cur_px = (bcx + self.beam_x as f32 * sx) as usize;
            let cur_py = (bcy - self.beam_xp as f32 * sy) as usize;
            if cur_px < bw && cur_py < bh {
                grid[cur_py][cur_px] = ('*', Style::default().fg(Color::Rgb(255, 220, 100)).bg(Color::Rgb(10, 10, 18)).add_modifier(Modifier::BOLD));
            }
        }

        let lines: Vec<Line> = grid.into_iter()
            .map(|row| Line::from(row.into_iter().map(|(ch, s)| Span::styled(String::from(ch), s)).collect::<Vec<_>>()))
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_phase_space_y(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(80, 40, 120)))
            .title(format!(" Y-Y' ({:.1},{:.1}) ", self.beam_y, self.beam_yp))
            .title_style(Style::default().fg(Color::Rgb(200, 120, 255)));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let bw = inner.width as usize;
        let bh = inner.height as usize;
        if bw < 3 || bh < 3 { return; }

        let plot_range_y = 20.0_f32; // mm
        let plot_range_yp = 8.0_f32; // mrad
        let bcx = bw as f32 / 2.0;
        let bcy = bh as f32 / 2.0;
        let sx = bcx / plot_range_y;
        let sy = bcy / plot_range_yp;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(10, 10, 18))); bw]; bh];

        // Crosshair axes
        let cx_i = bcx as usize;
        let cy_i = bcy as usize;
        for x in 0..bw {
            if cy_i < bh { grid[cy_i][x] = ('-', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        for y in 0..bh {
            if cx_i < bw { grid[y][cx_i] = ('|', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        if cx_i < bw && cy_i < bh {
            grid[cy_i][cx_i] = ('+', Style::default().fg(Color::Rgb(30, 30, 50)).bg(Color::Rgb(10, 10, 18)));
        }
        draw_plot_ticks(&mut grid, bw, bh, bcx, bcy, sx, sy, plot_range_y, plot_range_yp);

        // Draw approximate Courant-Snyder ellipse boundary
        let bg = self.current_beta * self.current_gamma;
        let emit = if bg > 0.01 { EMITTANCE_NORM_95 / bg / 6.0 } else { 1.0 };
        let beta_tw = self.beta_y_max;
        if beta_tw > 0.1 && emit > 0.0 {
            let y_max = (emit * beta_tw).sqrt() as f32;
            let yp_max = (emit / beta_tw).sqrt() as f32;
            let steps = 60;
            for i in 0..steps {
                let theta = (i as f32 / steps as f32) * std::f32::consts::TAU;
                let ey = y_max * theta.cos();
                let eyp = yp_max * theta.sin();
                let px = (bcx + ey * sx) as usize;
                let py = (bcy - eyp * sy) as usize;
                if px < bw && py < bh {
                    let ch = grid[py][px].0;
                    if ch == ' ' || ch == '-' || ch == '|' {
                        grid[py][px] = ('.', Style::default().fg(Color::Rgb(50, 20, 80)).bg(Color::Rgb(10, 10, 18)));
                    }
                }
            }
        }

        // History dots (fading)
        let n = self.y_yp_history.len();
        for (i, &(hy, hyp)) in self.y_yp_history.iter().enumerate() {
            let px = (bcx + hy * sx) as usize;
            let py = (bcy - hyp * sy) as usize;
            if px < bw && py < bh {
                let brightness = (0.3 + 0.7 * (i as f32 / n.max(1) as f32)).min(1.0);
                let r = (180.0 * brightness) as u8;
                let g = (80.0 * brightness) as u8;
                let b = (255.0 * brightness) as u8;
                grid[py][px] = ('o', Style::default().fg(Color::Rgb(r, g, b)).bg(Color::Rgb(10, 10, 18)));
            }
        }

        // Current position
        if self.beam_running && !self.beam_lost {
            let cur_px = (bcx + self.beam_y as f32 * sx) as usize;
            let cur_py = (bcy - self.beam_yp as f32 * sy) as usize;
            if cur_px < bw && cur_py < bh {
                grid[cur_py][cur_px] = ('*', Style::default().fg(Color::Rgb(220, 160, 255)).bg(Color::Rgb(10, 10, 18)).add_modifier(Modifier::BOLD));
            }
        }

        let lines: Vec<Line> = grid.into_iter()
            .map(|row| Line::from(row.into_iter().map(|(ch, s)| Span::styled(String::from(ch), s)).collect::<Vec<_>>()))
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_longitudinal_plot(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(80, 120, 40)))
            .title(format!(" Longit. phi:{:.2} dE:{:.4} ", self.beam_phi, self.beam_de))
            .title_style(Style::default().fg(Color::Rgb(180, 255, 80)));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let bw = inner.width as usize;
        let bh = inner.height as usize;
        if bw < 3 || bh < 3 { return; }

        let phi_range = std::f32::consts::PI; // +/- pi
        let bcx = bw as f32 / 2.0;
        let bcy = bh as f32 / 2.0;
        let sx = bcx / phi_range;

        // Compute bucket half-height for dE scaling
        let eta = slip_factor(self.current_gamma);
        let v_per_turn = self.rf_voltage_mv * 1e-3;
        let total_e_gev = self.current_ke_gev + PROTON_MASS_GEV;
        let bucket_height = if eta.abs() > 1e-6 {
            (2.0 * v_per_turn * self.current_beta.powi(2) * total_e_gev
             / (std::f64::consts::PI * HARMONIC_NUMBER as f64 * eta.abs())).sqrt()
        } else {
            0.001
        };
        let de_range = (bucket_height * 2.0).max(0.01) as f32;
        let sy = bcy / de_range;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(10, 10, 18))); bw]; bh];

        // Crosshair axes
        let cx_i = bcx as usize;
        let cy_i = bcy as usize;
        for x in 0..bw {
            if cy_i < bh { grid[cy_i][x] = ('-', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        for y in 0..bh {
            if cx_i < bw { grid[y][cx_i] = ('|', Style::default().fg(Color::Rgb(25, 25, 40)).bg(Color::Rgb(10, 10, 18))); }
        }
        if cx_i < bw && cy_i < bh {
            grid[cy_i][cx_i] = ('+', Style::default().fg(Color::Rgb(30, 30, 50)).bg(Color::Rgb(10, 10, 18)));
        }
        draw_plot_ticks(&mut grid, bw, bh, bcx, bcy, sx, sy, phi_range, de_range);

        // Draw RF bucket separatrix
        // dE = sqrt(eV*beta^2*E / (pi*h*|eta|) * (cos(phi) + 1))
        if eta.abs() > 1e-6 {
            let coeff = v_per_turn * self.current_beta.powi(2) * total_e_gev
                / (std::f64::consts::PI * HARMONIC_NUMBER as f64 * eta.abs());
            let steps = bw * 2;
            for i in 0..steps {
                let phi = -std::f64::consts::PI + (i as f64 / steps as f64) * 2.0 * std::f64::consts::PI;
                let val = coeff * (phi.cos() + 1.0);
                if val > 0.0 {
                    let de_sep = val.sqrt();
                    // Upper separatrix
                    let px = (bcx + phi as f32 * sx) as usize;
                    let py_up = (bcy - de_sep as f32 * sy) as usize;
                    let py_dn = (bcy + de_sep as f32 * sy) as usize;
                    if px < bw && py_up < bh {
                        let ch = grid[py_up][px].0;
                        if ch == ' ' || ch == '-' || ch == '|' {
                            grid[py_up][px] = ('.', Style::default().fg(Color::Rgb(40, 80, 30)).bg(Color::Rgb(10, 10, 18)));
                        }
                    }
                    if px < bw && py_dn < bh {
                        let ch = grid[py_dn][px].0;
                        if ch == ' ' || ch == '-' || ch == '|' {
                            grid[py_dn][px] = ('.', Style::default().fg(Color::Rgb(40, 80, 30)).bg(Color::Rgb(10, 10, 18)));
                        }
                    }
                }
            }
        }

        // History dots
        let n = self.phi_history.len();
        for (i, (&phi, &de)) in self.phi_history.iter().zip(self.de_history.iter()).enumerate() {
            let px = (bcx + phi * sx) as usize;
            let py = (bcy - de * sy) as usize;
            if px < bw && py < bh {
                let brightness = (0.3 + 0.7 * (i as f32 / n.max(1) as f32)).min(1.0);
                let r = (120.0 * brightness) as u8;
                let g = (255.0 * brightness) as u8;
                let b = (50.0 * brightness) as u8;
                grid[py][px] = ('o', Style::default().fg(Color::Rgb(r, g, b)).bg(Color::Rgb(10, 10, 18)));
            }
        }

        // Current position
        if self.beam_running && !self.beam_lost {
            let cur_px = (bcx + self.beam_phi as f32 * sx) as usize;
            let cur_py = (bcy - self.beam_de as f32 * sy) as usize;
            if cur_px < bw && cur_py < bh {
                grid[cur_py][cur_px] = ('*', Style::default().fg(Color::Rgb(180, 255, 80)).bg(Color::Rgb(10, 10, 18)).add_modifier(Modifier::BOLD));
            }
        }

        let lines: Vec<Line> = grid.into_iter()
            .map(|row| Line::from(row.into_iter().map(|(ch, s)| Span::styled(String::from(ch), s)).collect::<Vec<_>>()))
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_tune_diagram(&self, frame: &mut Frame, area: Rect) {
        let qx = (self.tune_x + self.sc_tune_shift).fract() as f32;
        let qy = (self.tune_y + self.sc_tune_shift).fract() as f32;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(40, 80, 120)))
            .title(format!(" Tune Qx:{:.3} Qy:{:.3} ", qx, qy))
            .title_style(Style::default().fg(Color::Rgb(100, 200, 255)));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let bw = inner.width as usize;
        let bh = inner.height as usize;
        if bw < 3 || bh < 3 { return; }

        // Display range: 0.5 to 1.0 for both planes
        let q_min = 0.5_f32;
        let q_max = 1.0_f32;
        let q_range = q_max - q_min;

        let mut grid: Vec<Vec<(char, Style)>> =
            vec![vec![(' ', Style::default().bg(Color::Rgb(10, 10, 18))); bw]; bh];

        // Map tune value to pixel coordinate
        let to_px = |q: f32| -> usize {
            ((q - q_min) / q_range * (bw - 1) as f32).round() as usize
        };
        let to_py = |q: f32| -> usize {
            (((q_max - q) / q_range) * (bh - 1) as f32).round() as usize
        };

        // Half-integer resonance lines: qx=0.5, qy=0.5
        let px_half = to_px(0.5);
        let py_half = to_py(0.5);
        if px_half < bw {
            for y in 0..bh {
                grid[y][px_half] = ('|', Style::default().fg(Color::Rgb(180, 120, 40)).bg(Color::Rgb(10, 10, 18)));
            }
        }
        if py_half < bh {
            for x in 0..bw {
                grid[py_half][x] = ('-', Style::default().fg(Color::Rgb(180, 120, 40)).bg(Color::Rgb(10, 10, 18)));
            }
        }

        // Third-integer resonance lines: qx=2/3, qy=2/3
        let px_third = to_px(2.0 / 3.0);
        let py_third = to_py(2.0 / 3.0);
        if px_third < bw {
            for y in 0..bh {
                if grid[y][px_third].0 == ' ' {
                    grid[y][px_third] = (':', Style::default().fg(Color::Rgb(180, 180, 40)).bg(Color::Rgb(10, 10, 18)));
                }
            }
        }
        if py_third < bh {
            for x in 0..bw {
                if grid[py_third][x].0 == ' ' {
                    grid[py_third][x] = ('.', Style::default().fg(Color::Rgb(180, 180, 40)).bg(Color::Rgb(10, 10, 18)));
                }
            }
        }

        // Coupling resonance: qx = qy (diagonal)
        for i in 0..bw.min(bh * 2) {
            let q = q_min + (i as f32 / (bw.min(bh * 2) - 1).max(1) as f32) * q_range;
            let px = to_px(q);
            let py = to_py(q);
            if px < bw && py < bh {
                if grid[py][px].0 == ' ' {
                    grid[py][px] = ('/', Style::default().fg(Color::Rgb(40, 180, 180)).bg(Color::Rgb(10, 10, 18)));
                }
            }
        }

        // Sum resonance: qx + qy = 1 (anti-diagonal)
        for i in 0..bw.min(bh * 2) {
            let qx_val = q_min + (i as f32 / (bw.min(bh * 2) - 1).max(1) as f32) * q_range;
            let qy_val = 1.0 - qx_val;
            if qy_val >= q_min && qy_val <= q_max {
                let px = to_px(qx_val);
                let py = to_py(qy_val);
                if px < bw && py < bh {
                    if grid[py][px].0 == ' ' {
                        grid[py][px] = ('\\', Style::default().fg(Color::Rgb(40, 180, 180)).bg(Color::Rgb(10, 10, 18)));
                    }
                }
            }
        }

        // Working point
        if qx >= q_min && qx <= q_max && qy >= q_min && qy <= q_max {
            let wpx = to_px(qx);
            let wpy = to_py(qy);
            if wpx < bw && wpy < bh {
                grid[wpy][wpx] = ('*', Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(10, 10, 18)).add_modifier(Modifier::BOLD));
            }
        }

        // Axis tick labels along bottom (Qx) and left edge (Qy)
        let tick_label_style = Style::default().fg(Color::Rgb(55, 65, 90)).bg(Color::Rgb(10, 10, 18));
        let tick_mark_style = Style::default().fg(Color::Rgb(70, 70, 100)).bg(Color::Rgb(10, 10, 18));
        for &q in &[0.5, 0.6, 0.7, 0.8, 0.9, 1.0] {
            // Bottom edge: Qx labels
            let px = to_px(q);
            if px < bw && bh > 1 {
                if grid[bh - 2][px].0 == ' ' || grid[bh - 2][px].0 == '-' {
                    grid[bh - 2][px] = ('+', tick_mark_style);
                }
                let label = format!("{:.1}", q);
                let start = px.saturating_sub(label.len() / 2);
                for (i, c) in label.chars().enumerate() {
                    let col = start + i;
                    if col < bw {
                        grid[bh - 1][col] = (c, tick_label_style);
                    }
                }
            }
            // Left edge: Qy labels
            let py = to_py(q);
            if py < bh && bw > 4 {
                if grid[py][1].0 == ' ' || grid[py][1].0 == '|' {
                    grid[py][1] = ('+', tick_mark_style);
                }
                let label = format!("{:.1}", q);
                for (i, c) in label.chars().enumerate() {
                    if i + 2 < bw.min(6) {
                        grid[py][i + 2] = (c, tick_label_style);
                    }
                }
            }
        }

        let lines: Vec<Line> = grid.into_iter()
            .map(|row| Line::from(row.into_iter().map(|(ch, s)| Span::styled(String::from(ch), s)).collect::<Vec<_>>()))
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_corrector_panel(&self, frame: &mut Frame, area: Rect) {
        if let Some(ref bump) = self.bump {
            // Bump mode panel
            let sec_coeffs = bump.section_coefficients();
            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(vec![
                Span::styled(format!(" {}-BUMP", bump.size), Style::default().fg(Color::Rgb(80, 255, 200)).add_modifier(Modifier::BOLD)),
            ]));
            for (s, c) in &sec_coeffs {
                let sign = if *c > 0.0 { "+" } else { "-" };
                let color = if *c > 0.0 { Color::Rgb(80, 255, 180) } else { Color::Rgb(255, 140, 80) };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}C{}(x{:.0})", sign, s + 1, c.abs()), Style::default().fg(color)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(format!("    H:{:+.4} V:{:+.4}", self.correctors[*s].h_trim, self.correctors[*s].v_trim),
                        Style::default().fg(Color::Rgb(160, 160, 180))),
                ]));
            }
            lines.push(Line::from(Span::styled("", Style::default())));
            lines.push(Line::from(vec![
                Span::styled(" U/D", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" H+V ", Style::default().fg(Color::Rgb(140, 140, 160))),
                Span::styled("W/S", Style::default().fg(Color::Rgb(255, 180, 120))),
                Span::styled(" H", Style::default().fg(Color::Rgb(140, 140, 160))),
            ]));
            lines.push(Line::from(vec![
                Span::styled(" E/Q", Style::default().fg(Color::Rgb(200, 120, 255))),
                Span::styled(" V ", Style::default().fg(Color::Rgb(140, 140, 160))),
                Span::styled("L/R", Style::default().fg(Color::Rgb(120, 220, 255))),
                Span::styled(" shift", Style::default().fg(Color::Rgb(140, 140, 160))),
            ]));

            let panel = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 180, 140)))
                    .title(format!(" {}-Bump ", bump.size))
                    .title_style(Style::default().fg(Color::Rgb(80, 255, 200)).add_modifier(Modifier::BOLD)));
            frame.render_widget(panel, area);
        } else {
            // Normal corrector panel
            let cell = self.selected_cell;
            let corr = &self.correctors[cell];
            let mut lines: Vec<Line> = Vec::new();

            lines.push(Line::from(vec![
                Span::styled(format!(" Cell {}/{}", cell + 1, NUM_SECTIONS),
                    Style::default().fg(Color::Rgb(200, 200, 220)).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" step:{:.4}", self.adjust_speed),
                    Style::default().fg(Color::Rgb(140, 140, 160))),
            ]));
            lines.push(Line::from(Span::styled("", Style::default())));

            let corrector_values: Vec<(CorrectorSelect, f64)> = vec![
                (CorrectorSelect::HTrim, corr.h_trim),
                (CorrectorSelect::VTrim, corr.v_trim),
                (CorrectorSelect::TrimQuad, corr.trim_quad),
                (CorrectorSelect::SkewQuad, corr.skew_quad),
                (CorrectorSelect::SextA, corr.sext_a),
                (CorrectorSelect::SextB, corr.sext_b),
            ];

            for (ct, val) in &corrector_values {
                let is_sel = *ct == self.selected_corrector;
                let indicator = if is_sel { " >" } else { "  " };
                lines.push(Line::from(vec![
                    Span::styled(indicator, Style::default().fg(Color::Rgb(255, 255, 100))),
                    Span::styled(format!("{}", ct.label()),
                        Style::default().fg(if is_sel { Color::White } else { ct.color() })
                            .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() })),
                    Span::styled(format!(" {:+.5}", val),
                        Style::default().fg(if is_sel { Color::Rgb(255, 220, 80) } else { Color::Rgb(120, 120, 150) })
                            .add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() })),
                ]));
            }

            lines.push(Line::from(Span::styled("", Style::default())));
            lines.push(Line::from(vec![
                Span::styled(" U/D", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" sel ", Style::default().fg(Color::Rgb(100, 100, 130))),
                Span::styled("L/R", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" adj", Style::default().fg(Color::Rgb(100, 100, 130))),
            ]));
            lines.push(Line::from(vec![
                Span::styled(" []", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" cell ", Style::default().fg(Color::Rgb(100, 100, 130))),
                Span::styled("+/-", Style::default().fg(Color::Rgb(255, 255, 100))),
                Span::styled(" step", Style::default().fg(Color::Rgb(100, 100, 130))),
            ]));

            let panel = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(60, 100, 140)))
                    .title(" Correctors ")
                    .title_style(Style::default().fg(Color::Rgb(120, 200, 255))));
            frame.render_widget(panel, area);
        }
    }

    fn render_ring(&self, frame: &mut Frame, area: Rect) {
        let ring_w = area.width as usize;
        let ring_h = area.height as usize;
        if ring_w < 10 || ring_h < 6 { return; }

        let cx = ring_w as f32 / 2.0;
        let cy = ring_h as f32 / 2.0;
        let rx = (ring_w as f32 * 0.35).min(cx - 4.0);
        let ry = (ring_h as f32 * 0.38).min(cy - 2.0);

        let mut grid: Vec<Vec<(char, Style)>> = vec![vec![(' ', Style::default()); ring_w]; ring_h];

        // Arc connectors
        let connect_steps = 2;
        for sec in 0..NUM_SECTIONS {
            let a1 = (sec as f32 / NUM_SECTIONS as f32) * std::f32::consts::PI * 2.0 - std::f32::consts::FRAC_PI_2;
            let a2 = ((sec + 1) as f32 / NUM_SECTIONS as f32) * std::f32::consts::PI * 2.0 - std::f32::consts::FRAC_PI_2;
            for step in 1..=connect_steps {
                let t = step as f32 / (connect_steps + 1) as f32;
                let a = a1 + (a2 - a1) * t;
                let dx = (cx + rx * a.cos()) as usize;
                let dy = (cy + ry * a.sin()) as usize;
                if dx < ring_w && dy < ring_h && grid[dy][dx].0 == ' ' {
                    let tangent = a.cos().abs();
                    let ch = if tangent > 0.7 { '-' } else if tangent < 0.3 { '|' } else { '.' };
                    grid[dy][dx] = (ch, Style::default().fg(Color::Rgb(30, 40, 55)));
                }
            }
        }

        // Section markers
        for sec in 0..NUM_SECTIONS {
            let angle = (sec as f32 / NUM_SECTIONS as f32) * std::f32::consts::PI * 2.0 - std::f32::consts::FRAC_PI_2;
            let x = (cx + rx * angle.cos()) as usize;
            let y = (cy + ry * angle.sin()) as usize;
            if x >= ring_w || y >= ring_h { continue; }

            let is_beam_here = self.beam_running && !self.beam_lost && self.beam_cell == sec;
            let is_selected = self.selected_cell == sec;
            let is_bump = self.bump.as_ref().map_or(false, |b| b.contains_section(sec));

            let trail_entry = self.trail.iter().rev().find(|(s, _, _)| *s == sec);

            let (ch, style) = if is_beam_here {
                ('O', Style::default().fg(Color::Rgb(100, 255, 255)).add_modifier(Modifier::BOLD))
            } else if let Some((_, pos, _)) = trail_entry {
                let intensity = if pos.abs() < 10.0 { 200 } else if pos.abs() < 30.0 { 140 } else { 80 };
                ('o', Style::default().fg(Color::Rgb(30, intensity as u8, (intensity + 30).min(255) as u8)))
            } else if is_bump {
                let coeff = self.bump.as_ref().and_then(|b| b.coeff_for_section(sec)).unwrap_or(0.0);
                let ch = if coeff > 0.0 { '+' } else { '-' };
                let color = if coeff > 0.0 { Color::Rgb(80, 255, 180) } else { Color::Rgb(255, 140, 80) };
                (ch, Style::default().fg(color).add_modifier(Modifier::BOLD))
            } else if is_selected {
                ('*', Style::default().fg(Color::Rgb(255, 220, 80)).add_modifier(Modifier::BOLD))
            } else {
                ('o', Style::default().fg(Color::Rgb(60, 80, 100)))
            };

            grid[y][x] = (ch, style);

            // Labels
            let lx = (cx + (rx + 2.0) * angle.cos()) as usize;
            let ly = (cy + (ry + 1.0) * angle.sin()) as usize;
            if lx < ring_w && ly < ring_h {
                let label = format!("{}", sec + 1);
                for (i, c) in label.chars().enumerate() {
                    let nx = lx + i;
                    if nx < ring_w {
                        let col = if is_selected { Color::Rgb(255, 220, 80) } else { Color::Rgb(60, 60, 80) };
                        grid[ly][nx] = (c, Style::default().fg(col));
                    }
                }
            }
        }

        // Center text
        let center_text = match self.phase {
            GamePhase::Extraction => "EXTRACTED!",
            GamePhase::Lost => "LOST",
            GamePhase::Setup => "READY",
            GamePhase::Transition => "TRANSITION!",
            _ if self.paused => "PAUSED",
            _ if self.beam_running => "RUNNING",
            _ => "READY",
        };
        let ctx = (cx as usize).saturating_sub(center_text.len() / 2);
        let cty = cy as usize;
        let ct_color = self.phase.color();
        for (i, c) in center_text.chars().enumerate() {
            let x = ctx + i;
            if x < ring_w && cty < ring_h {
                grid[cty][x] = (c, Style::default().fg(ct_color).add_modifier(Modifier::BOLD));
            }
        }

        // Energy display below center
        if cty + 1 < ring_h {
            let energy_text = format!("{:.2} GeV", self.current_ke_gev);
            let ex = (cx as usize).saturating_sub(energy_text.len() / 2);
            for (i, c) in energy_text.chars().enumerate() {
                let x = ex + i;
                if x < ring_w {
                    grid[cty + 1][x] = (c, Style::default().fg(Color::Rgb(255, 200, 80)));
                }
            }
        }

        // Sparkline below energy
        if !self.pos_history.is_empty() && cty + 2 < ring_h {
            let sparkline_chars = ['_', '.', '-', '=', '#', '#', '#', '#'];
            let spark_w = (rx as usize).min(self.pos_history.len()).min(20);
            let start_idx = self.pos_history.len().saturating_sub(spark_w);
            let spark_x = (cx as usize).saturating_sub(spark_w / 2);
            for (i, &val) in self.pos_history[start_idx..].iter().enumerate() {
                let x = spark_x + i;
                if x < ring_w && cty + 2 < ring_h {
                    let norm = (val.abs() / APERTURE_DISPLAY).min(1.0);
                    let idx = (norm * 7.0) as usize;
                    let color = if norm < 0.2 { Color::Rgb(50, 200, 100) }
                        else if norm < 0.5 { Color::Rgb(200, 200, 50) }
                        else { Color::Rgb(200, 60, 60) };
                    grid[cty + 2][x] = (sparkline_chars[idx], Style::default().fg(color));
                }
            }
        }

        let lines: Vec<Line> = grid.into_iter()
            .map(|row| Line::from(row.into_iter().map(|(ch, s)| Span::styled(String::from(ch), s)).collect::<Vec<_>>()))
            .collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_help_bar(&self, frame: &mut Frame, area: Rect) {
        // Input mode: show coordinate prompt instead of normal help
        if self.input_mode != InputMode::None {
            let (label, prompt_color) = match self.input_mode {
                InputMode::InjectX => ("Inject X (mm)", Color::Rgb(255, 180, 120)),
                InputMode::InjectY => ("Inject Y (mm)", Color::Rgb(200, 120, 255)),
                InputMode::None => unreachable!(),
            };
            let lines = vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {}: ", label),
                        Style::default().fg(prompt_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}_", self.input_buffer),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" Enter", Style::default().fg(Color::Rgb(255, 255, 100))),
                    Span::styled(" Confirm ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Esc", Style::default().fg(Color::Rgb(255, 255, 100))),
                    Span::styled(" Cancel ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Bksp", Style::default().fg(Color::Rgb(255, 255, 100))),
                    Span::styled(" Delete", Style::default().fg(Color::DarkGray)),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), area);
            return;
        }

        let lines = match self.phase {
            GamePhase::Lost => {
                vec![
                    Line::from(vec![
                        Span::styled(" BEAM LOST! ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        Span::styled("Adjust correctors and press ENTER to retry, Esc for menu", Style::default().fg(Color::Gray)),
                    ]),
                    Line::from(vec![
                        Span::styled(" V", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" View ", Style::default().fg(Color::DarkGray)),
                        Span::styled("R", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Reset", Style::default().fg(Color::DarkGray)),
                    ]),
                ]
            }
            GamePhase::Extraction => {
                vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" EXTRACTED! I:{:.0}% Score:{} ", self.beam_intensity * 100.0, self.get_score()),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("Press ENTER to play again", Style::default().fg(Color::Gray)),
                    ]),
                    Line::from(vec![
                        Span::styled(" V", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" View ", Style::default().fg(Color::DarkGray)),
                        Span::styled("R", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Reset", Style::default().fg(Color::DarkGray)),
                    ]),
                ]
            }
            _ if self.bump.is_some() => {
                vec![
                    Line::from(vec![
                        Span::styled(" BUMP ", Style::default().fg(Color::Rgb(80, 255, 200)).add_modifier(Modifier::BOLD)),
                        Span::styled("U/D", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" H+V ", Style::default().fg(Color::DarkGray)),
                        Span::styled("W/S", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" H ", Style::default().fg(Color::DarkGray)),
                        Span::styled("E/Q", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" V ", Style::default().fg(Color::DarkGray)),
                        Span::styled("L/R", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Shift ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Z", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Zero ", Style::default().fg(Color::DarkGray)),
                        Span::styled("B", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Cycle", Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(vec![
                        Span::styled(" J/K", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Quad ", Style::default().fg(Color::DarkGray)),
                        Span::styled("M/N", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Bend ", Style::default().fg(Color::DarkGray)),
                        Span::styled("F/G", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" RF-V ", Style::default().fg(Color::DarkGray)),
                        Span::styled("T", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" RF-ph ", Style::default().fg(Color::DarkGray)),
                        Span::styled("V", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" View ", Style::default().fg(Color::DarkGray)),
                        Span::styled("+/-", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Step ", Style::default().fg(Color::DarkGray)),
                        Span::styled("P", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Pause ", Style::default().fg(Color::DarkGray)),
                        Span::styled(".", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(format!(" {}", self.sim_speed.label()), Style::default().fg(Color::DarkGray)),
                    ]),
                ]
            }
            _ => {
                vec![
                    Line::from({
                        let mut spans = vec![
                            Span::styled(if self.beam_running { " SPACE" } else { " SPACE" },
                                Style::default().fg(Color::Rgb(255, 255, 100))),
                            Span::styled(if self.beam_running { " Run " } else { " Inject " },
                                Style::default().fg(if self.beam_running { Color::Green } else { Color::Yellow })),
                        ];
                        if !self.beam_running {
                            spans.push(Span::styled("I", Style::default().fg(Color::Rgb(255, 255, 100))));
                            spans.push(Span::styled(" Inject@XY ", Style::default().fg(Color::DarkGray)));
                        }
                        spans.extend_from_slice(&[
                            Span::styled("U/D", Style::default().fg(Color::Rgb(255, 255, 100))),
                            Span::styled(" Corr ", Style::default().fg(Color::DarkGray)),
                            Span::styled("L/R", Style::default().fg(Color::Rgb(255, 255, 100))),
                            Span::styled(" Adj ", Style::default().fg(Color::DarkGray)),
                            Span::styled("[]", Style::default().fg(Color::Rgb(255, 255, 100))),
                            Span::styled(" Cell ", Style::default().fg(Color::DarkGray)),
                            Span::styled("B", Style::default().fg(Color::Rgb(255, 255, 100))),
                            Span::styled(" Bump ", Style::default().fg(Color::DarkGray)),
                            Span::styled("C", Style::default().fg(Color::Rgb(255, 255, 100))),
                            Span::styled(" Copy ", Style::default().fg(Color::DarkGray)),
                            Span::styled("Z", Style::default().fg(Color::Rgb(255, 255, 100))),
                            Span::styled(" Zero", Style::default().fg(Color::DarkGray)),
                        ]);
                        spans
                    }),
                    Line::from(vec![
                        Span::styled(" J/K", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Quad ", Style::default().fg(Color::DarkGray)),
                        Span::styled("M/N", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Bend ", Style::default().fg(Color::DarkGray)),
                        Span::styled("F/G", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" RF-V ", Style::default().fg(Color::DarkGray)),
                        Span::styled("T", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" RF-ph ", Style::default().fg(Color::DarkGray)),
                        Span::styled("V", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" View ", Style::default().fg(Color::DarkGray)),
                        Span::styled("+/-", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Step ", Style::default().fg(Color::DarkGray)),
                        Span::styled("P", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Pause ", Style::default().fg(Color::DarkGray)),
                        Span::styled(".", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(format!(" {} ", self.sim_speed.label()), Style::default().fg(Color::DarkGray)),
                        Span::styled("R", Style::default().fg(Color::Rgb(255, 255, 100))),
                        Span::styled(" Reset", Style::default().fg(Color::DarkGray)),
                    ]),
                ]
            }
        };
        frame.render_widget(Paragraph::new(lines), area);
    }
}
