// Copyright (C) 2025  Tom Waddington
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Playback engine for quipu scripts
//!
//! Executes parsed commands with proper timing and jitter

use anyhow::Result;
use rand::RngExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::time::sleep;

use crate::pty::PtyManager;
use crate::types::{Command, PlaybackConfig, Script};

pub struct PlaybackEngine {
    pty: PtyManager,
    config: PlaybackConfig,
    running: Arc<AtomicBool>,
}

impl PlaybackEngine {
    pub fn new(pty: PtyManager, running: Arc<AtomicBool>) -> Result<Self> {
        // In raw mode Ctrl-C never raises SIGINT (the PTY stdin forwarder
        // handles it instead); this covers non-TTY runs and external signals
        let r = running.clone();
        ctrlc::set_handler(move || {
            eprintln!("\nReceived Ctrl-C, stopping playback...");
            r.store(false, Ordering::SeqCst);
        })?;

        Ok(Self {
            pty,
            config: PlaybackConfig::default(),
            running,
        })
    }

    fn should_continue(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn calculate_delay(&self) -> Duration {
        let mut rng = rand::rng();
        let base_ms = (self.config.speed * 1000.0) as u64;
        let jitter_ms = (base_ms as f64 * self.config.jitter) as u64;

        if jitter_ms > 0 {
            let variation = rng.random_range(0..=jitter_ms * 2);
            let delay = base_ms.saturating_add(variation).saturating_sub(jitter_ms);
            Duration::from_millis(delay)
        } else {
            Duration::from_millis(base_ms)
        }
    }

    // The returned length is used to slice the UTF-8 text by byte offset, so it
    // must never claim a partial multibyte character
    fn escape_sequence_length(bytes: &[u8]) -> usize {
        if bytes.len() < 2 || bytes[0] != 0x1b {
            return 1;
        }

        match bytes[1] {
            // CSI sequences: ESC [ ... (end with letter or ~)
            b'[' => {
                let mut i = 2;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b';') {
                    i += 1;
                }
                if i < bytes.len() && bytes[i].is_ascii() {
                    i + 1
                } else {
                    i
                }
            }
            // SS3 sequences: ESC O + letter
            b'O' => {
                if bytes.len() > 2 && bytes[2].is_ascii() {
                    3
                } else {
                    2
                }
            }
            // Alt-prefixed key: ESC + one ASCII char
            b if b.is_ascii() => 2,
            // ESC followed by a multibyte char: send ESC alone
            _ => 1,
        }
    }

    async fn execute_command(&mut self, command: &Command) -> Result<()> {
        match command {
            Command::SetSpeed(speed) => {
                self.config.speed = *speed;
            }
            Command::SetJitter(jitter) => {
                self.config.jitter = *jitter;
            }
            Command::Wait(duration) => {
                sleep(*duration).await;
            }
            Command::SetShell(_) | Command::SetSize(_, _) => {
                // Shell and size are applied before playback starts, ignore during execution
            }
            Command::Capture(path) => {
                self.pty.capture(path)?;
            }
            Command::Type(text) => {
                // Escape sequences must be sent atomically without delays between bytes
                let mut i = 0;
                let bytes = text.as_bytes();

                while i < bytes.len() {
                    if !self.should_continue() {
                        return Ok(());
                    }

                    if bytes[i] == 0x1b {
                        let seq_len = Self::escape_sequence_length(&bytes[i..]);
                        let sequence = &text[i..i + seq_len];

                        self.pty.send_keystroke(sequence)?;
                        i += seq_len;

                        let delay = self.calculate_delay();
                        sleep(delay).await;
                    } else {
                        let c = text[i..].chars().next().unwrap();
                        self.pty.send_char(c)?;
                        i += c.len_utf8();

                        let delay = self.calculate_delay();
                        sleep(delay).await;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn execute(&mut self, script: Script) -> Result<()> {
        for command in script.commands {
            if !self.should_continue() {
                break;
            }

            self.execute_command(&command).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_sequence_lengths() {
        assert_eq!(PlaybackEngine::escape_sequence_length(b"\x1b[A"), 3);
        assert_eq!(PlaybackEngine::escape_sequence_length(b"\x1b[15~"), 5);
        assert_eq!(PlaybackEngine::escape_sequence_length(b"\x1bOP"), 3);
        assert_eq!(PlaybackEngine::escape_sequence_length(b"\x1bx"), 2);
        assert_eq!(PlaybackEngine::escape_sequence_length(b"\x1b"), 1);
        assert_eq!(PlaybackEngine::escape_sequence_length(b"a"), 1);
    }

    #[test]
    fn test_escape_sequence_length_stays_on_char_boundary() {
        // ESC directly followed by a multibyte char
        let text = "\x1b\u{e9}";
        let len = PlaybackEngine::escape_sequence_length(text.as_bytes());
        assert_eq!(len, 1);
        let _ = &text[..len]; // must not panic

        // CSI params followed by a multibyte char
        let text = "\x1b[1\u{e9}";
        let len = PlaybackEngine::escape_sequence_length(text.as_bytes());
        assert_eq!(len, 3);
        let _ = &text[..len];

        // ESC O followed by a multibyte char
        let text = "\x1bO\u{e9}";
        let len = PlaybackEngine::escape_sequence_length(text.as_bytes());
        assert_eq!(len, 2);
        let _ = &text[..len];
    }
}
