# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo run                # Run in debug mode
cargo build --release    # Build optimized binary (target/release/rustcade)
cargo check              # Quick compilation check
cargo clippy             # Lint
cargo fmt                # Format code
```

Cross-compile with `cross build --release --target <triple>` (all deps are pure Rust).

No test suite exists; validation is done through manual gameplay.

## Architecture

Rust-Cade is a TUI arcade suite (~9k lines) with 8 games, built on ratatui + crossterm + rand.

### Main Loop & Event System

`main.rs` sets up the terminal (raw mode, alternate screen, mouse capture) and runs a draw/event/tick loop. `event.rs` spawns a background thread polling crossterm events at ~60 FPS (16ms tick), sending them via mpsc channel.

### App State Machine (app.rs)

`App` owns all game instances and routes input. A `Tab` enum selects between Home and each game. Global keys (tab switching, pause, reset, name entry for high scores) are handled in `App::on_key()`; game-specific input is forwarded to the active game's `handle_input()`. `App::on_tick()` calls `update()` on the active game.

### Game Trait (games/mod.rs)

Every game implements:
```rust
pub trait Game {
    fn update(&mut self);
    fn handle_input(&mut self, key: KeyEvent);
    fn render(&mut self, frame: &mut Frame, area: Rect);
    fn reset(&mut self);
    fn get_score(&self) -> u32;
    fn is_game_over(&self) -> bool;
}
```

### Adding a New Game

1. Create `src/games/new_game.rs` implementing the `Game` trait
2. Add `pub mod new_game;` in `src/games/mod.rs`
3. Add a `Tab` variant in `app.rs` and wire up input routing + tick forwarding
4. Add a tile in `ui/home.rs` and rendering dispatch in `ui/mod.rs`
5. Register in the score system (`scores.rs`) — format supports a fixed number of games

### UI Layer (ui/)

`ui/mod.rs` splits the screen into a tab bar (3 rows) + content area. `home.rs` renders a 2x4 game tile grid. `tabs.rs` renders the navigation bar. Modal overlays (name entry, help screen) render on top of everything via `Clear` + centered `Paragraph`.

### Help System

Press `?` from any screen to show a scrollable help overlay. Help content is defined per-tab in `help_lines_for_tab()` in `ui/mod.rs`. Scroll state (`help_scroll`) lives in `App`. All in-game help bars show `? Help` hint. When adding a new game, add a help entry in the match arm in `ui/mod.rs`.

### High Score System (scores.rs)

Binary file format: magic header "RCS2" + fixed slots (3 entries per game, each entry = 9-byte name + u32 score). File is stored next to the executable. Session deduplication prevents repeat submissions.

### Rendering Patterns

- All positions use `f32` for smooth physics; field dimensions are recalculated at render time to handle terminal resizes
- Braille Unicode blocks provide sub-character resolution (used in Space Invaders shields, Asteroids ship/asteroid shapes)
- Hardcoded RGB colors throughout (e.g., `Color::Rgb(255, 220, 80)` for gold highlights)
- Games: frogger, breakout, dino_run, space_invaders, jezzball, asteroids, beam, booster
