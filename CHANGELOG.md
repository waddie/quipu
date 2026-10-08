# Changelog

## v0.4.0 (2026-10-08)

### Added

- `@ expect:TEXT` waits until the screen shows `TEXT`, and fails the run with
  exit status 1 and a copy of the screen if it doesn't appear in time
- `@ timeout:N` sets how long `expect` waits (default 10 seconds)
- `@ capture-text:PATH` captures the screen as plain text

### Fixed

- quipu no longer hangs at the end of a script that leaves a program running
  in the foreground. The program swallowed the end of input meant for the
  shell, so the shell never exited; quipu now hangs up on it after a second.

## v0.3.0 (2026-08-07)

### Added

- `-V` / `--version`

### Changed

- Repeating an option is no longer an error; the last occurrence wins
