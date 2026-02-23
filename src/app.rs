use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::games::asteroids::Asteroids;
use crate::games::beam::BeamGame;
use crate::games::booster::BoosterGame;
use crate::games::breakout::Breakout;
use crate::games::dino_run::DinoRun;
use crate::games::frogger::Frogger;
use crate::games::jezzball::JezzBall;
use crate::games::space_invaders::SpaceInvaders;
use crate::games::Game;
use crate::scores::HighScores;

const MAX_NAME_LEN: usize = 9;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Home,
    Frogger,
    Breakout,
    DinoRun,
    SpaceInvaders,
    JezzBall,
    Asteroids,
    Booster,
    Beam,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Home, Tab::Frogger, Tab::Breakout, Tab::DinoRun, Tab::SpaceInvaders, Tab::JezzBall, Tab::Asteroids, Tab::Booster, Tab::Beam]
    }

    pub fn title(&self) -> &str {
        match self {
            Tab::Home => " Home ",
            Tab::Frogger => " Frogger ",
            Tab::Breakout => " Breakout ",
            Tab::DinoRun => " Dino Run ",
            Tab::SpaceInvaders => " Invaders ",
            Tab::JezzBall => " JezzBall ",
            Tab::Asteroids => " Asteroids ",
            Tab::Booster => " Booster ",
            Tab::Beam => " Beam ",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Home => 0,
            Tab::Frogger => 1,
            Tab::Breakout => 2,
            Tab::DinoRun => 3,
            Tab::SpaceInvaders => 4,
            Tab::JezzBall => 5,
            Tab::Asteroids => 6,
            Tab::Booster => 7,
            Tab::Beam => 8,
        }
    }

}

pub struct App {
    pub should_quit: bool,
    pub current_tab: Tab,
    pub selected_game: usize, // 0-7 for home screen game selection
    pub frogger: Frogger,
    pub breakout: Breakout,
    pub dino_run: DinoRun,
    pub space_invaders: SpaceInvaders,
    pub jezzball: JezzBall,
    pub asteroids: Asteroids,
    pub booster: BoosterGame,
    pub beam: BeamGame,
    pub high_scores: HighScores,
    pub show_high_scores: bool,
    // Name entry state
    pub entering_name: bool,
    pub name_buffer: String,
    pub name_game_idx: usize,
    pub name_score: u32,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            current_tab: Tab::Home,
            selected_game: 0,
            frogger: Frogger::new(),
            breakout: Breakout::new(),
            dino_run: DinoRun::new(),
            space_invaders: SpaceInvaders::new(),
            jezzball: JezzBall::new(),
            asteroids: Asteroids::new(),
            booster: BoosterGame::new(),
            beam: BeamGame::new(),
            high_scores: HighScores::load(),
            show_high_scores: false,
            entering_name: false,
            name_buffer: String::new(),
            name_game_idx: 0,
            name_score: 0,
        }
    }

    pub fn on_tick(&mut self) {
        // Don't update games while entering a name
        if self.entering_name {
            return;
        }

        match self.current_tab {
            Tab::Home => {}
            Tab::Frogger => self.frogger.update(),
            Tab::Breakout => self.breakout.update(),
            Tab::DinoRun => self.dino_run.update(),
            Tab::SpaceInvaders => self.space_invaders.update(),
            Tab::JezzBall => self.jezzball.update(),
            Tab::Asteroids => self.asteroids.update(),
            Tab::Booster => self.booster.update(),
            Tab::Beam => self.beam.update(),
        }
        // Check for high scores when games end
        self.check_submit_scores();
    }

    fn check_submit_scores(&mut self) {
        let games: [(usize, bool, u32); 8] = [
            (0, self.frogger.is_game_over(), self.frogger.get_score()),
            (1, self.breakout.is_game_over(), self.breakout.get_score()),
            (2, self.dino_run.is_game_over(), self.dino_run.get_score()),
            (3, self.space_invaders.is_game_over(), self.space_invaders.get_score()),
            (4, self.jezzball.is_game_over(), self.jezzball.get_score()),
            (5, self.asteroids.is_game_over(), self.asteroids.get_score()),
            (6, self.booster.is_game_over(), self.booster.get_score()),
            (7, self.beam.is_game_over(), self.beam.get_score()),
        ];
        for (idx, game_over, score) in games {
            if game_over && score > 0 && !self.high_scores.was_submitted(idx) {
                if self.high_scores.qualifies(idx, score) {
                    // Prompt for name entry
                    self.entering_name = true;
                    self.name_buffer.clear();
                    self.name_game_idx = idx;
                    self.name_score = score;
                    self.high_scores.mark_submitted(idx);
                    return; // Only one at a time
                } else {
                    // Score doesn't qualify, just mark as submitted
                    self.high_scores.mark_submitted(idx);
                }
            }
            if !game_over && self.high_scores.was_submitted(idx) {
                self.high_scores.clear_submitted(idx);
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        // Ctrl+C always quits
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        // If entering a name, intercept all input
        if self.entering_name {
            self.handle_name_input(key);
            return;
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                if matches!(self.current_tab, Tab::Home) {
                    self.should_quit = true;
                    return;
                }
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.prev_tab();
                } else {
                    self.next_tab();
                }
                return;
            }
            KeyCode::BackTab => {
                self.prev_tab();
                return;
            }
            KeyCode::Esc => {
                if !matches!(self.current_tab, Tab::Home) {
                    self.current_tab = Tab::Home;
                    return;
                }
            }
            _ => {}
        }

        // Home screen shortcuts and navigation
        if matches!(self.current_tab, Tab::Home) && key.modifiers.is_empty() {
            match key.code {
                KeyCode::Char('1') => { self.current_tab = Tab::Frogger; return; }
                KeyCode::Char('2') => { self.current_tab = Tab::Breakout; return; }
                KeyCode::Char('3') => { self.current_tab = Tab::DinoRun; return; }
                KeyCode::Char('4') => { self.current_tab = Tab::SpaceInvaders; return; }
                KeyCode::Char('5') => { self.current_tab = Tab::JezzBall; return; }
                KeyCode::Char('6') => { self.current_tab = Tab::Asteroids; return; }
                KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.show_high_scores = !self.show_high_scores;
                    return;
                }
                KeyCode::Char('7') => { self.current_tab = Tab::Booster; return; }
                KeyCode::Char('8') => { self.current_tab = Tab::Beam; return; }
                // Arrow key navigation for game tile selection (2 rows: 4 + 4)
                KeyCode::Right => {
                    self.selected_game = (self.selected_game + 1) % 8;
                    return;
                }
                KeyCode::Left => {
                    self.selected_game = (self.selected_game + 7) % 8;
                    return;
                }
                KeyCode::Down => {
                    if self.selected_game < 4 {
                        self.selected_game = self.selected_game + 4;
                    } else {
                        self.selected_game -= 4;
                    }
                    return;
                }
                KeyCode::Up => {
                    if self.selected_game >= 4 {
                        self.selected_game -= 4;
                    } else {
                        self.selected_game = self.selected_game + 4;
                    }
                    return;
                }
                // Enter launches the selected game
                KeyCode::Enter => {
                    self.current_tab = match self.selected_game {
                        0 => Tab::Frogger,
                        1 => Tab::Breakout,
                        2 => Tab::DinoRun,
                        3 => Tab::SpaceInvaders,
                        4 => Tab::JezzBall,
                        5 => Tab::Asteroids,
                        6 => Tab::Booster,
                        7 => Tab::Beam,
                        _ => Tab::Home,
                    };
                    return;
                }
                _ => {}
            }
        }

        // Forward to active game
        match self.current_tab {
            Tab::Home => {}
            Tab::Frogger => self.frogger.handle_input(key),
            Tab::Breakout => self.breakout.handle_input(key),
            Tab::DinoRun => self.dino_run.handle_input(key),
            Tab::SpaceInvaders => self.space_invaders.handle_input(key),
            Tab::JezzBall => self.jezzball.handle_input(key),
            Tab::Asteroids => self.asteroids.handle_input(key),
            Tab::Booster => self.booster.handle_input(key),
            Tab::Beam => self.beam.handle_input(key),
        }
    }

    fn handle_name_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                // Submit the score with the entered name
                let name = if self.name_buffer.is_empty() {
                    "???".to_string()
                } else {
                    self.name_buffer.clone()
                };
                self.high_scores.submit(self.name_game_idx, &name, self.name_score);
                self.entering_name = false;
                self.name_buffer.clear();
            }
            KeyCode::Backspace => {
                self.name_buffer.pop();
            }
            KeyCode::Esc => {
                // Cancel — submit with default name
                self.high_scores.submit(self.name_game_idx, "???", self.name_score);
                self.entering_name = false;
                self.name_buffer.clear();
            }
            KeyCode::Char(c) => {
                // Only allow printable ASCII characters, up to MAX_NAME_LEN
                if self.name_buffer.chars().count() < MAX_NAME_LEN && c.is_ascii_graphic() {
                    self.name_buffer.push(c.to_ascii_uppercase());
                }
            }
            _ => {}
        }
    }

    fn next_tab(&mut self) {
        let tabs = Tab::all();
        let idx = self.current_tab.index();
        self.current_tab = tabs[(idx + 1) % tabs.len()];
    }

    fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let idx = self.current_tab.index();
        self.current_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
    }
}
