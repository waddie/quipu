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

pub struct PlaybackEngine {
    pty: PtyManager,
    config: PlaybackConfig,
    running: Arc<AtomicBool>,
}

impl PlaybackEngine {
    pub fn new(pty: PtyManager) -> Result<Self> {
        let running = Arc::new(AtomicBool::new(true));

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

    fn escape_sequence_length(&self, bytes: &[u8]) -> usize {
        if bytes.is_empty() || bytes[0] != 0x1b {
            return 1;
        }

        if bytes.len() == 1 {
            return 1;
        }

        match bytes[1] {
            // CSI sequences: ESC [ ... (end with letter or ~)
            b'[' => {
                let mut i = 2;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b';') {
                    i += 1;
                }
                if i < bytes.len() { i + 1 } else { bytes.len() }
            }
            // SS3 sequences: ESC O + letter
            b'O' => {
                if bytes.len() > 2 {
                    3
                } else {
                    bytes.len()
                }
            }
            _ => 2,
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
            Command::SetShell(_) => {
                // Shell is set before playback starts, ignore during execution
            }
            Command::SetSize(_, _) => {
                // Size is set before PTY creation, ignore during execution
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
                        let seq_len = self.escape_sequence_length(&bytes[i..]);
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
