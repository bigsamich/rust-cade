use std::fs;
use std::path::PathBuf;

const MAGIC: &[u8; 4] = b"RCHS";
const NUM_GAMES: usize = 6;
const SCORES_PER_GAME: usize = 3;
const TOTAL_SCORES: usize = NUM_GAMES * SCORES_PER_GAME;
// File size: 4 magic + 18 * 4 bytes = 76 bytes
const FILE_SIZE: usize = 4 + TOTAL_SCORES * 4;

pub const GAME_NAMES: [&str; NUM_GAMES] = [
    "Frogger", "Breakout", "Dino Run", "Pinball", "JezzBall", "Beam",
];

#[derive(Clone)]
pub struct HighScores {
    scores: [[u32; SCORES_PER_GAME]; NUM_GAMES],
    path: PathBuf,
    /// Track which games have had their score submitted this session
    /// to avoid duplicate submissions
    submitted: [bool; NUM_GAMES],
}

impl HighScores {
    pub fn load() -> Self {
        let path = Self::scores_path();
        let mut hs = HighScores {
            scores: [[0; SCORES_PER_GAME]; NUM_GAMES],
            path,
            submitted: [false; NUM_GAMES],
        };
        hs.read_file();
        hs
    }

    fn scores_path() -> PathBuf {
        // Store next to the executable
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join("rustcade.scores");
            }
        }
        PathBuf::from("rustcade.scores")
    }

    fn read_file(&mut self) {
        let Ok(data) = fs::read(&self.path) else { return };
        if data.len() < FILE_SIZE { return; }
        if &data[0..4] != MAGIC { return; }

        let mut offset = 4;
        for game in 0..NUM_GAMES {
            for slot in 0..SCORES_PER_GAME {
                if offset + 4 <= data.len() {
                    let bytes: [u8; 4] = [
                        data[offset], data[offset + 1],
                        data[offset + 2], data[offset + 3],
                    ];
                    self.scores[game][slot] = u32::from_le_bytes(bytes);
                    offset += 4;
                }
            }
        }
    }

    fn write_file(&self) {
        let mut buf = Vec::with_capacity(FILE_SIZE);
        buf.extend_from_slice(MAGIC);
        for game in 0..NUM_GAMES {
            for slot in 0..SCORES_PER_GAME {
                buf.extend_from_slice(&self.scores[game][slot].to_le_bytes());
            }
        }
        let _ = fs::write(&self.path, &buf);
    }

    /// Submit a score for a game. Returns true if it's a new high score (top 3).
    pub fn submit(&mut self, game_idx: usize, score: u32) -> bool {
        if game_idx >= NUM_GAMES || score == 0 { return false; }

        // Find insertion point (sorted descending)
        let mut insert_at = None;
        for i in 0..SCORES_PER_GAME {
            if score > self.scores[game_idx][i] {
                insert_at = Some(i);
                break;
            }
        }

        if let Some(pos) = insert_at {
            // Shift lower scores down
            for i in (pos + 1..SCORES_PER_GAME).rev() {
                self.scores[game_idx][i] = self.scores[game_idx][i - 1];
            }
            self.scores[game_idx][pos] = score;
            self.write_file();
            true
        } else {
            false
        }
    }

    /// Get top 3 scores for a game
    pub fn top_scores(&self, game_idx: usize) -> [u32; SCORES_PER_GAME] {
        if game_idx >= NUM_GAMES {
            return [0; SCORES_PER_GAME];
        }
        self.scores[game_idx]
    }

    /// Check if a game score has been submitted this run (to avoid duplicates)
    pub fn was_submitted(&self, game_idx: usize) -> bool {
        if game_idx >= NUM_GAMES { return false; }
        self.submitted[game_idx]
    }

    /// Mark a game as submitted
    pub fn mark_submitted(&mut self, game_idx: usize) {
        if game_idx < NUM_GAMES {
            self.submitted[game_idx] = true;
        }
    }

    /// Clear submitted flag (called when game resets)
    pub fn clear_submitted(&mut self, game_idx: usize) {
        if game_idx < NUM_GAMES {
            self.submitted[game_idx] = false;
        }
    }
}
