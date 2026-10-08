// Copyright (C) 2026  Tom Waddington
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

//! Runs the quipu binary on small scripts that wait for and capture text.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quipu-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(dir: &Path, script: &str) -> Output {
    let path = dir.join("script.qp");
    std::fs::write(&path, script).unwrap();
    Command::new(env!("CARGO_BIN_EXE_quipu"))
        .args(["--quiet", "--shell", "/bin/sh"])
        .arg(&path)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn expect_waits_for_text_and_capture_text_saves_the_screen() {
    let dir = scratch_dir("pass");
    let output = run(
        &dir,
        "@ speed:0.01\n\
         $ sleep 1; echo do''ne<ret>\n\
         @ expect:done\n\
         @ capture-text:screen.txt\n",
    );
    assert!(output.status.success(), "{output:?}");
    let screen = std::fs::read_to_string(dir.join("screen.txt")).unwrap();
    // The echoed command has quotes in it, so this line is the output.
    assert!(screen.lines().any(|line| line == "done"), "{screen:?}");
    assert!(!screen.contains('\x1b'), "plain text has no escape codes");
}

#[test]
fn expect_fails_with_the_screen_when_text_never_appears() {
    let dir = scratch_dir("fail");
    let output = run(
        &dir,
        "@ speed:0.01\n\
         $ echo hello<ret>\n\
         @ timeout:0.5\n\
         @ expect:goodbye\n\
         $ echo unreachable<ret>\n",
    );
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("goodbye"), "{stderr}");
    assert!(stderr.contains("hello"), "the screen is shown: {stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("unreachable"), "playback stops: {stdout}");
}

#[test]
fn exits_when_a_program_is_still_running_at_the_end() {
    // The program in the foreground swallows the end of input meant for the
    // shell, so quipu has to stop the shell itself.
    let dir = scratch_dir("lingering");
    let path = dir.join("script.qp");
    std::fs::write(&path, "@ speed:0.01\n$ sleep 30<ret>\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_quipu"))
        .args(["--quiet", "--shell", "/bin/sh"])
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() > deadline {
            child.kill().unwrap();
            panic!("quipu was still running 10s after its script ended");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
