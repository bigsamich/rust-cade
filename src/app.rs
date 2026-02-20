use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::games::beam::BeamGame;
use crate::games::breakout::Breakout;
use crate::games::dino_run::DinoRun;
use crate::games::frogger::Frogger;
use crate::games::jezzball::JezzBall;
use crate::games::pinball::Pinball;
use crate::games::Game;

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Home,
    Frogger,
    Breakout,
    DinoRun,
    Pinball,
    JezzBall,
    Beam,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Home, Tab::Frogger, Tab::Breakout, Tab::DinoRun, Tab::Pinball, Tab::JezzBall, Tab::Beam]
    }

    pub fn title(&self) -> &str {
        match self {
            Tab::Home => " Home ",
            Tab::Frogger => " Frogger ",
            Tab::Breakout => " Breakout ",
            Tab::DinoRun => " Dino Run ",
            Tab::Pinball => " Pinball ",
            Tab::JezzBall => " JezzBall ",
            Tab::Beam => " Beam ",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Home => 0,
            Tab::Frogger => 1,
            Tab::Breakout => 2,
            Tab::DinoRun => 3,
            Tab::Pinball => 4,
            Tab::JezzBall => 5,
            Tab::Beam => 6,
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub current_tab: Tab,
    pub selected_game: usize, // 0-5 for home screen game selection
    pub frogger: Frogger,
    pub breakout: Breakout,
    pub dino_run: DinoRun,
    pub pinball: Pinball,
    pub jezzball: JezzBall,
    pub beam: BeamGame,
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
            pinball: Pinball::new(),
            jezzball: JezzBall::new(),
            beam: BeamGame::new(),
        }
    }

    pub fn on_tick(&mut self) {
        match self.current_tab {
            Tab::Home => {}
            Tab::Frogger => self.frogger.update(),
            Tab::Breakout => self.breakout.update(),
            Tab::DinoRun => self.dino_run.update(),
            Tab::Pinball => self.pinball.update(),
            Tab::JezzBall => self.jezzball.update(),
            Tab::Beam => self.beam.update(),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        // Ctrl+C always quits
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
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
                KeyCode::Char('4') => { self.current_tab = Tab::Pinball; return; }
                KeyCode::Char('5') => { self.current_tab = Tab::JezzBall; return; }
                KeyCode::Char('6') => { self.current_tab = Tab::Beam; return; }
                // Arrow key navigation for game tile selection (2 rows x 3 cols)
                KeyCode::Right => {
                    self.selected_game = (self.selected_game + 1) % 6;
                    return;
                }
                KeyCode::Left => {
                    self.selected_game = (self.selected_game + 5) % 6;
                    return;
                }
                KeyCode::Down => {
                    self.selected_game = (self.selected_game + 3) % 6;
                    return;
                }
                KeyCode::Up => {
                    self.selected_game = (self.selected_game + 3) % 6;
                    return;
                }
                // Enter launches the selected game
                KeyCode::Enter => {
                    self.current_tab = match self.selected_game {
                        0 => Tab::Frogger,
                        1 => Tab::Breakout,
                        2 => Tab::DinoRun,
                        3 => Tab::Pinball,
                        4 => Tab::JezzBall,
                        5 => Tab::Beam,
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
            Tab::Pinball => self.pinball.handle_input(key),
            Tab::JezzBall => self.jezzball.handle_input(key),
            Tab::Beam => self.beam.handle_input(key),
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
