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

//! Core types for quipu script execution

use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SetSpeed(f64),
    SetJitter(f64),
    Wait(Duration),
    // Must come before any Type commands
    SetShell(String),
    // Must come before PTY creation
    SetSize(u16, u16),
    Type(String),
}

#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    // Base time between keystrokes in seconds
    pub speed: f64,
    // Jitter as a fraction (0.0 to 1.0) of speed
    pub jitter: f64,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            speed: 0.1,  // 100ms per keystroke
            jitter: 0.0, // No jitter
        }
    }
}

#[derive(Debug)]
pub struct Script {
    pub commands: Vec<Command>,
}
