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

//! PTY management for quipu
//!
//! Handles spawning processes in a PTY and sending keystrokes to them

use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{IsTerminal, Read, Write};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
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

// Shared, closable handle to the PTY master writer. Both the scripted playback
// and the stdin-forwarding thread write through this. The Option lets Drop close
// the writer to signal EOF even though the forwarding thread holds an Arc clone.
type SharedWriter = Arc<Mutex<Option<Box<dyn Write + Send>>>>;

// Shared VT parser fed by the reader thread as PTY output flows through. It
// mirrors the visible screen so a capture directive can serialise the current
// state to escape codes.
type SharedParser = Arc<Mutex<vt100::Parser>>;

pub struct PtyManager {
    writer: SharedWriter,
    parser: SharedParser,
    reader_thread: Option<thread::JoinHandle<()>>,
    _raw_mode_guard: RawModeGuard,
}

impl PtyManager {
    pub fn new(shell: &str, cols: u16, rows: u16, running: Arc<AtomicBool>) -> Result<Self> {
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
        let writer: SharedWriter = Arc::new(Mutex::new(Some(writer)));

        // Forward the real terminal's stdin into the PTY master. Without this,
        // terminal query/response protocols break: a program in the PTY sends a
        // query (e.g. ESC[6n for cursor position), the real terminal replies on
        // our stdin, and the reply never reaches the program. It then mispositions
        // the cursor and eventually times out. Started here so early prompt
        // queries (e.g. after a `cd`) are answered. Raw mode is already enabled,
        // so stdin bytes arrive verbatim. The thread is detached; it may block in
        // read at shutdown, which is fine since the process exits after playback.
        let stdin_writer = writer.clone();
        thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut buffer = [0u8; 1024];

            loop {
                match stdin.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        // Raw mode disables ISIG, so Ctrl-C arrives here as a
                        // byte instead of raising SIGINT. Stop playback and
                        // still forward it so the inner program is interrupted.
                        if buffer[..n].contains(&0x03) {
                            running.store(false, Ordering::SeqCst);
                        }
                        let Ok(mut guard) = stdin_writer.lock() else {
                            break;
                        };
                        let Some(w) = guard.as_mut() else { break };
                        if w.write_all(&buffer[..n]).is_err() || w.flush().is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Mirror the visible screen at the PTY's dimensions; no scrollback, as a
        // capture only serialises the visible grid.
        let parser: SharedParser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));
        let reader_parser = parser.clone();

        let reader_thread = thread::spawn(move || {
            let mut reader = reader;
            let mut stdout = std::io::stdout();
            let mut buffer = [0u8; 8192];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        // stdout is the primary path: never let a locked or
                        // poisoned parser block live output.
                        if stdout.write_all(&buffer[..n]).is_err() {
                            break;
                        }
                        if stdout.flush().is_err() {
                            break;
                        }
                        if let Ok(mut parser) = reader_parser.lock() {
                            parser.process(&buffer[..n]);
                        }
                    }
                }
            }
        });

        Ok(Self {
            writer,
            parser,
            reader_thread: Some(reader_thread),
            _raw_mode_guard: raw_mode_guard,
        })
    }

    pub fn send_keystroke(&mut self, data: &str) -> Result<()> {
        let mut guard = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("PTY writer lock poisoned"))?;
        let writer = guard.as_mut().context("PTY writer has been closed")?;
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

    // Serialise the current screen to a file as terminal escape codes. The
    // resulting file, when written to a raw terminal (e.g. `cat`), reproduces
    // the visible state at this point in playback.
    pub fn capture(&self, path: &Path) -> Result<()> {
        let contents = {
            let parser = self
                .parser
                .lock()
                .map_err(|_| anyhow::anyhow!("PTY parser lock poisoned"))?;
            parser.screen().contents_formatted()
        };
        std::fs::write(path, contents)
            .with_context(|| format!("Failed to write capture to {}", path.display()))?;
        Ok(())
    }
}

impl Drop for PtyManager {
    fn drop(&mut self) {
        // Close the writer to signal EOF. This drops the writer Box regardless of
        // the detached stdin-forwarding thread's surviving Arc clone.
        if let Ok(mut guard) = self.writer.lock() {
            let _ = guard.take();
        }

        // Wait for reader thread to ensure all output is flushed before raw mode is disabled
        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }

        // Allow time for parent terminal to respond to any terminal queries
        thread::sleep(Duration::from_millis(100));

        // Note: stdin is owned by the forwarding thread, which relays terminal
        // query responses into the PTY live, so there is no backlog to drain here.

        // _raw_mode_guard drops here, restoring terminal state
    }
}
