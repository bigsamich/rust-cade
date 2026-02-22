pub mod beam;
pub mod breakout;
pub mod dino_run;
pub mod frogger;
pub mod jezzball;
pub mod pinball;

use crossterm::event::KeyEvent;
use ratatui::prelude::*;

pub trait Game {
    fn update(&mut self);
    fn handle_input(&mut self, key: KeyEvent);
    fn render(&mut self, frame: &mut Frame, area: Rect);
    fn reset(&mut self);
    fn get_score(&self) -> u32;
    fn is_game_over(&self) -> bool;
}
