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

//! Playback engine for typecast scripts
//!
//! Executes parsed commands with proper timing and jitter

use anyhow::Result;
use rand::Rng;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::time::sleep;

use crate::pty::PtyManager;
use crate::types::{Command, PlaybackConfig, Script};

/// Execute a script in a PTY
pub struct PlaybackEngine {
    pty: PtyManager,
    config: PlaybackConfig,
    running: Arc<AtomicBool>,
}

impl PlaybackEngine {
    /// Create a new playback engine
    pub fn new(pty: PtyManager) -> Result<Self> {
        let running = Arc::new(AtomicBool::new(true));

        // Set up Ctrl-C handler
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

    /// Check if playback should continue
    fn should_continue(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Calculate delay with jitter
    fn calculate_delay(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let base_ms = (self.config.speed * 1000.0) as u64;
        let jitter_ms = (base_ms as f64 * self.config.jitter) as u64;

        if jitter_ms > 0 {
            let variation = rng.gen_range(0..=jitter_ms * 2);
            let delay = base_ms.saturating_add(variation).saturating_sub(jitter_ms);
            Duration::from_millis(delay)
        } else {
            Duration::from_millis(base_ms)
        }
    }

    /// Determine the length of an escape sequence starting with ESC (0x1b)
    fn escape_sequence_length(&self, bytes: &[u8]) -> usize {
        if bytes.is_empty() || bytes[0] != 0x1b {
            return 1;
        }

        if bytes.len() == 1 {
            return 1; // Just ESC alone
        }

        match bytes[1] {
            // CSI sequences: ESC [ ... (letter or ~)
            b'[' => {
                let mut i = 2;
                // Skip parameter bytes (digits, semicolon, etc.)
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b';') {
                    i += 1;
                }
                // Final byte is a letter or ~
                if i < bytes.len() { i + 1 } else { bytes.len() }
            }
            // SS3 sequences: ESC O (letter)
            b'O' => {
                if bytes.len() > 2 {
                    3
                } else {
                    bytes.len()
                }
            }
            // Simple two-byte escape
            _ => 2,
        }
    }

    /// Execute a single command
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
            Command::SetShell(_) => {
                // Shell is set before playback starts, ignore during execution
            }
            Command::SetSize(_, _) => {
                // Size is set before PTY creation, ignore during execution
            }
            Command::Type(text) => {
                // Split text into chunks: regular chars and escape sequences
                // Escape sequences must be sent atomically (without delays) to work properly
                let mut i = 0;
                let bytes = text.as_bytes();

                while i < bytes.len() {
                    if !self.should_continue() {
                        return Ok(());
                    }

                    // Check if this is the start of an escape sequence
                    if bytes[i] == 0x1b {
                        // Find the end of the escape sequence
                        let seq_len = self.escape_sequence_length(&bytes[i..]);
                        let sequence = &text[i..i + seq_len];

                        // Send entire escape sequence at once (no delay between bytes)
                        self.pty.send_keystroke(sequence)?;
                        i += seq_len;

                        // Add delay after the escape sequence
                        let delay = self.calculate_delay();
                        sleep(delay).await;
                    } else {
                        // Regular character - send with delay
                        let c = text[i..].chars().next().unwrap();
                        self.pty.send_char(c)?;
                        i += c.len_utf8();

                        // Add delay between characters
                        let delay = self.calculate_delay();
                        sleep(delay).await;
                    }
                }
            }
        }
        Ok(())
    }

    /// Execute an entire script
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
