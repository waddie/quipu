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

mod parser;
mod playback;
mod pty;
mod types;

use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use std::path::PathBuf;

#[derive(ClapParser, Debug)]
#[command(name = "typecast")]
#[command(about = "Script keyboard entry in the terminal", long_about = None)]
struct Args {
    /// The script file to execute
    #[arg(value_name = "SCRIPT")]
    script: PathBuf,

    /// Shell to use for the PTY session (defaults to current shell)
    #[arg(short, long)]
    shell: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Read the script file
    let script_content = std::fs::read_to_string(&args.script)
        .with_context(|| format!("Failed to read script file: {}", args.script.display()))?;

    // Parse the script
    let script =
        parser::parse_script(&script_content).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    // Determine shell to use (priority: CLI arg > script directive > $SHELL env > bash)
    let default_shell = args
        .shell
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "bash".to_string());

    // Check if script specifies a shell or size (must come before any Type commands)
    let mut shell = default_shell;
    let mut cols = 80u16;
    let mut rows = 24u16;

    for command in &script.commands {
        match command {
            types::Command::SetShell(s) => {
                shell = s.clone();
            }
            types::Command::SetSize(c, r) => {
                cols = *c;
                rows = *r;
            }
            types::Command::Type(_) => {
                // Stop looking once we hit a Type command
                break;
            }
            _ => {}
        }
    }

    println!("Parsed {} commands", script.commands.len());
    println!("Using shell: {}", shell);
    println!("Terminal size: {}x{}", cols, rows);
    println!("Starting playback in 1 second...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Create PTY manager with specified size
    let pty = pty::PtyManager::new(&shell, cols, rows).context("Failed to create PTY")?;

    // Create playback engine and execute
    let mut engine =
        playback::PlaybackEngine::new(pty).context("Failed to create playback engine")?;

    engine
        .execute(script)
        .await
        .context("Failed to execute script")?;

    // Drop the engine and PTY explicitly to clean up and restore terminal state
    // before printing completion message
    drop(engine);

    println!("\nPlayback complete!");

    // Brief pause so user can see the result
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(())
}
