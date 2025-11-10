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

//! Core types for typecast script execution

use std::time::Duration;

/// A command from the typecast script
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Set the speed (time between keystrokes)
    SetSpeed(f64),
    /// Set the jitter (random variation in timing)
    SetJitter(f64),
    /// Wait for a duration
    Wait(Duration),
    /// Set the shell to use (must come before any Type commands)
    SetShell(String),
    /// Set the terminal size (cols, rows) - must come before PTY creation
    SetSize(u16, u16),
    /// Type a sequence of text/keystrokes
    Type(String),
}

/// Configuration for playback timing
#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    /// Base time between keystrokes in seconds
    pub speed: f64,
    /// Maximum jitter as a fraction (0.0 to 1.0) of speed
    pub jitter: f64,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            speed: 0.1,  // 100ms between keystrokes
            jitter: 0.0, // No jitter by default
        }
    }
}

/// Result of parsing a script
#[derive(Debug)]
pub struct Script {
    pub commands: Vec<Command>,
}
