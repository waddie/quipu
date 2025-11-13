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

//! PTY management for typecast
//!
//! Handles spawning processes in a PTY and sending keystrokes to them

use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{IsTerminal, Read, Write};
use std::thread;
use std::time::Duration;

// RAII guard for terminal raw mode - only enables if stdout is a TTY
struct RawModeGuard {
    enabled: bool,
}

impl RawModeGuard {
    fn new() -> Result<Self> {
        let enabled = if std::io::stdout().is_terminal() {
            enable_raw_mode().context("Failed to enable raw mode")?;
            true
        } else {
            false
        };
        Ok(RawModeGuard { enabled })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.enabled {
            let _ = disable_raw_mode();
        }
    }
}

pub struct PtyManager {
    writer: Option<Box<dyn Write + Send>>,
    _reader_thread: Option<thread::JoinHandle<()>>,
    _raw_mode_guard: RawModeGuard,
}

impl PtyManager {
    pub fn new(shell: &str, cols: u16, rows: u16) -> Result<Self> {
        // Enable raw mode before PTY creation for proper escape sequence handling
        let raw_mode_guard = RawModeGuard::new()?;

        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to create PTY")?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");

        let _child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell in PTY")?;

        let reader = pair
            .master
            .try_clone_reader()
            .context("Failed to get PTY reader")?;

        let writer = pair
            .master
            .take_writer()
            .context("Failed to get PTY writer")?;

        let reader_thread = thread::spawn(move || {
            let mut reader = reader;
            let mut stdout = std::io::stdout();
            let mut buffer = [0u8; 8192];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if stdout.write_all(&buffer[..n]).is_err() {
                            break;
                        }
                        if stdout.flush().is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            writer: Some(writer),
            _reader_thread: Some(reader_thread),
            _raw_mode_guard: raw_mode_guard,
        })
    }

    pub fn send_keystroke(&mut self, data: &str) -> Result<()> {
        let writer = self.writer.as_mut().context("PTY writer has been closed")?;
        writer
            .write_all(data.as_bytes())
            .context("Failed to write to PTY")?;
        writer.flush().context("Failed to flush PTY")?;
        Ok(())
    }

    pub fn send_char(&mut self, c: char) -> Result<()> {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.send_keystroke(s)
    }
}

impl Drop for PtyManager {
    fn drop(&mut self) {
        // Close writer to signal EOF
        drop(self.writer.take());

        // Wait for reader thread to ensure all output is flushed before raw mode is disabled
        if let Some(handle) = self._reader_thread.take() {
            let _ = handle.join();
        }

        // Allow time for parent terminal to respond to any terminal queries
        thread::sleep(Duration::from_millis(100));

        // Drain stdin to prevent terminal query responses from appearing as garbage after exit
        if std::io::stdin().is_terminal() {
            use crossterm::event::{poll, read};
            while poll(Duration::from_millis(0)).unwrap_or(false) {
                let _ = read();
            }
        }

        // _raw_mode_guard drops here, restoring terminal state
    }
}
