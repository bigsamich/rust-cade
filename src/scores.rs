use std::fs;
use std::path::PathBuf;

const MAGIC: &[u8; 4] = b"RCS2";
const NUM_GAMES: usize = 8;
const SCORES_PER_GAME: usize = 3;
const TOTAL_SCORES: usize = NUM_GAMES * SCORES_PER_GAME;
const NAME_LEN: usize = 9;
// Each entry: 9 bytes name + 4 bytes score = 13 bytes
const ENTRY_SIZE: usize = NAME_LEN + 4;
// File size: 4 magic + 18 * 13 = 238 bytes
const FILE_SIZE: usize = 4 + TOTAL_SCORES * ENTRY_SIZE;

pub const GAME_NAMES: [&str; NUM_GAMES] = [
    "Frogger", "Breakout", "Dino Run", "Pinball", "JezzBall", "Asteroids", "Booster", "Beam",
];

#[derive(Clone)]
pub struct ScoreEntry {
    pub name: String,
    pub score: u32,
}

impl ScoreEntry {
    fn empty() -> Self {
        ScoreEntry {
            name: String::new(),
            score: 0,
        }
    }
}

#[derive(Clone)]
pub struct HighScores {
    scores: Vec<Vec<ScoreEntry>>,
    path: PathBuf,
    /// Track which games have had their score submitted this session
    /// to avoid duplicate submissions
    submitted: [bool; NUM_GAMES],
}

impl HighScores {
    pub fn load() -> Self {
        let path = Self::scores_path();
        let mut hs = HighScores {
            scores: (0..NUM_GAMES)
                .map(|_| (0..SCORES_PER_GAME).map(|_| ScoreEntry::empty()).collect())
                .collect(),
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
                if offset + ENTRY_SIZE <= data.len() {
                    // Read 9-byte name
                    let name_bytes = &data[offset..offset + NAME_LEN];
                    let name = String::from_utf8_lossy(name_bytes)
                        .trim_end_matches('\0')
                        .trim_end()
                        .to_string();
                    offset += NAME_LEN;

                    // Read 4-byte score
                    let bytes: [u8; 4] = [
                        data[offset], data[offset + 1],
                        data[offset + 2], data[offset + 3],
                    ];
                    let score = u32::from_le_bytes(bytes);
                    offset += 4;

                    self.scores[game][slot] = ScoreEntry { name, score };
                }
            }
        }
    }

    fn write_file(&self) {
        let mut buf = Vec::with_capacity(FILE_SIZE);
        buf.extend_from_slice(MAGIC);
        for game in 0..NUM_GAMES {
            for slot in 0..SCORES_PER_GAME {
                let entry = &self.scores[game][slot];
                // Write 9-byte name (padded with zeros)
                let name_bytes = entry.name.as_bytes();
                let len = name_bytes.len().min(NAME_LEN);
                buf.extend_from_slice(&name_bytes[..len]);
                for _ in len..NAME_LEN {
                    buf.push(0);
                }
                // Write 4-byte score
                buf.extend_from_slice(&entry.score.to_le_bytes());
            }
        }
        let _ = fs::write(&self.path, &buf);
    }

    /// Check if a score would qualify for the top 3 (without inserting it)
    pub fn qualifies(&self, game_idx: usize, score: u32) -> bool {
        if game_idx >= NUM_GAMES || score == 0 { return false; }
        for i in 0..SCORES_PER_GAME {
            if score > self.scores[game_idx][i].score {
                return true;
            }
        }
        false
    }

    /// Submit a score for a game with a name. Returns true if it's a new high score (top 3).
    pub fn submit(&mut self, game_idx: usize, name: &str, score: u32) -> bool {
        if game_idx >= NUM_GAMES || score == 0 { return false; }

        // Truncate name to 9 chars
        let name: String = name.chars().take(NAME_LEN).collect();

        // Find insertion point (sorted descending)
        let mut insert_at = None;
        for i in 0..SCORES_PER_GAME {
            if score > self.scores[game_idx][i].score {
                insert_at = Some(i);
                break;
            }
        }

        if let Some(pos) = insert_at {
            // Shift lower scores down
            for i in (pos + 1..SCORES_PER_GAME).rev() {
                self.scores[game_idx][i] = self.scores[game_idx][i - 1].clone();
            }
            self.scores[game_idx][pos] = ScoreEntry { name, score };
            self.write_file();
            true
        } else {
            false
        }
    }

    /// Get top 3 score entries for a game
    pub fn top_scores(&self, game_idx: usize) -> Vec<ScoreEntry> {
        if game_idx >= NUM_GAMES {
            return vec![ScoreEntry::empty(); SCORES_PER_GAME];
        }
        self.scores[game_idx].clone()
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
