# 🕹️ Rust-Cade

A TUI-based arcade game suite built with [Ratatui](https://github.com/ratatui/ratatui). Play classic arcade games right in your terminal!

![Version](https://img.shields.io/badge/version-0.9.2-blue)
![Rust](https://img.shields.io/badge/rust-2021_edition-orange)
![License](https://img.shields.io/badge/license-MIT-green)

## 🎮 Games

| Game | Description |
|------|-------------|
| **Beam** | Particle beam simulation — tune magnets across 24 ring sections to keep a beam stable for 5 turns. Features bump mode, power supply ramps, and difficulty settings. |
| **Breakout** | Classic brick-breaking action with paddle, ball, colored bricks, lives, and increasing speed. |
| **Dino Run** | Chrome-style endless runner — jump and duck to dodge cacti and birds as speed ramps up. |
| **Frogger** | Navigate traffic and ride logs across 13 lanes to reach the goal pads. |
| **JezzBall** | Launch growing walls to partition space and trap bouncing balls. Progress through levels with more balls. |
| **Pinball** | Terminal pinball with flippers, bumpers, spinners, and a multiball system. |

## 📦 Installation

### From source

```bash
git clone https://github.com/bigsamich/rust-cade.git
cd rust-cade
cargo build --release
```

The binary will be at `target/release/rustcade`.

### Run directly

```bash
cargo run
```

## 🚀 Usage

Launch the arcade:

```bash
rustcade
```

Use the tab-based menu to browse and select a game.

### Global Controls

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between game tabs |
| `Enter` | Start selected game |
| `Esc` | Return to menu / Quit |
| `q` | Quit |

Each game has its own controls displayed in-game.

## 🏗️ Project Structure

```
src/
├── main.rs          # Terminal setup & main loop
├── app.rs           # Application state & input routing
├── event.rs         # Async key/tick event handler (~30 FPS)
├── ui/
│   ├── mod.rs       # Root UI renderer
│   ├── home.rs      # Home screen
│   └── tabs.rs      # Tab navigation bar
└── games/
    ├── mod.rs       # Game trait & registry
    ├── beam.rs      # Beam simulation
    ├── breakout.rs  # Breakout
    ├── dino_run.rs  # Dino Run
    ├── frogger.rs   # Frogger
    ├── jezzball.rs  # JezzBall
    └── pinball.rs   # Pinball
```

## 🛠️ Dependencies

- [**ratatui**](https://crates.io/crates/ratatui) `0.29` — Terminal UI framework
- [**crossterm**](https://crates.io/crates/crossterm) `0.28` — Cross-platform terminal manipulation
- [**rand**](https://crates.io/crates/rand) `0.8` — Random number generation

## 🔀 Cross Compiling

Rust-cade can be cross-compiled for different platforms using Rust's built-in target support.

### Prerequisites

Install the desired target toolchain:

```bash
rustup target add <target-triple>
```

### Common Targets

| Platform | Target Triple | Notes |
|----------|---------------|-------|
| Linux (x86_64) | `x86_64-unknown-linux-gnu` | Default on most Linux systems |
| Linux (ARM64) | `aarch64-unknown-linux-gnu` | Raspberry Pi 4, ARM servers |
| Linux (ARMv7) | `armv7-unknown-linux-gnueabihf` | Raspberry Pi 2/3, older ARM boards |
| macOS (Apple Silicon) | `aarch64-apple-darwin` | M1/M2/M3 Macs |
| macOS (Intel) | `x86_64-apple-darwin` | Intel Macs |
| Windows (x86_64) | `x86_64-pc-windows-gnu` | Windows via MinGW |
| Windows (MSVC) | `x86_64-pc-windows-msvc` | Windows via MSVC (requires Windows SDK) |

### Building for a Target

```bash
cargo build --release --target <target-triple>
```

For example, to build for ARM64 Linux:

```bash
cargo build --release --target aarch64-unknown-linux-gnu
```

The binary will be at `target/<target-triple>/release/rustcade`.

### Using `cross`

For targets that require a different linker or system libraries, [`cross`](https://github.com/cross-rs/cross) simplifies the process by using pre-configured Docker containers:

```bash
# Install cross
cargo install cross

# Build for any supported target
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target armv7-unknown-linux-gnueabihf
cross build --release --target x86_64-pc-windows-gnu
```

### Linux Cross-Compile Without Docker

If you prefer not to use Docker, install the appropriate cross-compiler toolchain:

```bash
# For ARM64
sudo apt install gcc-aarch64-linux-gnu
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
cargo build --release --target aarch64-unknown-linux-gnu

# For ARMv7
sudo apt install gcc-arm-linux-gnueabihf
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
cargo build --release --target armv7-unknown-linux-gnueabihf
```

### Tips

- Rust-cade is a pure terminal application with no native GUI dependencies, making it straightforward to cross-compile.
- All dependencies (`ratatui`, `crossterm`, `rand`) are pure Rust, so no C library cross-compilation is needed.
- You can list all installed targets with `rustup target list --installed`.
- You can list all available targets with `rustup target list`.

## 🤝 Contributing

Contributions are welcome! To add a new game:

1. Create a new file in `src/games/` implementing the game logic
2. Register it in `src/games/mod.rs`
3. Add a tab entry so it appears in the menu
4. Submit a pull request

## 📄 License

This project is open source. See the repository for license details.
