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

//! Script parser for quipu files
//!
//! Parses scripts with the format:
//! - @ directives (speed, jitter, wait)
//! - # comments
//! - $ typing lines

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{char, not_line_ending, space0},
    combinator::{map, value},
};
use std::time::Duration;

use crate::types::{Command, Script};

fn parse_float(input: &str) -> IResult<&str, f64> {
    let (rest, value) = nom::number::complete::double(input)?;
    if !value.is_finite() || value < 0.0 {
        // Failure (not Error) so alt() aborts instead of trying other directives
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )));
    }
    Ok((rest, value))
}

fn parse_speed(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag("@")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("speed:")(input)?;
    let (input, value) = parse_float(input)?;
    Ok((input, Command::SetSpeed(value)))
}

fn parse_jitter(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag("@")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("jitter:")(input)?;
    let (input, value) = parse_float(input)?;
    Ok((input, Command::SetJitter(value)))
}

fn parse_wait(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag("@")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("wait:")(input)?;
    let (input, value) = parse_float(input)?;
    Ok((input, Command::Wait(Duration::from_secs_f64(value))))
}

fn parse_shell(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag("@")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("shell:")(input)?;
    let (input, shell) = not_line_ending(input)?;
    Ok((input, Command::SetShell(shell.trim().to_string())))
}

fn parse_size(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag("@")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("size:")(input)?;
    let (input, cols) = nom::character::complete::u16(input)?;
    let (input, _) = char(':')(input)?;
    let (input, rows) = nom::character::complete::u16(input)?;
    Ok((input, Command::SetSize(cols, rows)))
}

fn parse_directive(input: &str) -> IResult<&str, Command> {
    alt((
        parse_speed,
        parse_jitter,
        parse_wait,
        parse_shell,
        parse_size,
    ))
    .parse(input)
}

fn parse_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = char('#')(input)?;
    let (input, _) = not_line_ending(input)?;
    Ok((input, ()))
}

fn parse_key_spec(input: &str) -> IResult<&str, &str> {
    let (input, _) = char('<')(input)?;
    let (input, key_spec) = take_until(">")(input)?;
    let (input, _) = char('>')(input)?;
    Ok((input, key_spec))
}

fn base_key_seq(key: &str) -> Option<&'static str> {
    Some(match key {
        "esc" => "\x1b",
        "space" => " ",
        "ret" | "return" | "enter" => "\r",
        "tab" => "\t",
        "backspace" | "bs" => "\x7f",
        "F1" => "\x1bOP",
        "F2" => "\x1bOQ",
        "F3" => "\x1bOR",
        "F4" => "\x1bOS",
        "F5" => "\x1b[15~",
        "F6" => "\x1b[17~",
        "F7" => "\x1b[18~",
        "F8" => "\x1b[19~",
        "F9" => "\x1b[20~",
        "F10" => "\x1b[21~",
        "F11" => "\x1b[23~",
        "F12" => "\x1b[24~",
        "up" => "\x1b[A",
        "down" => "\x1b[B",
        "right" => "\x1b[C",
        "left" => "\x1b[D",
        "home" => "\x1b[H",
        "end" => "\x1b[F",
        "pageup" | "pgup" => "\x1b[5~",
        "pagedown" | "pgdn" => "\x1b[6~",
        "insert" | "ins" => "\x1b[2~",
        "delete" | "del" => "\x1b[3~",
        _ => return None,
    })
}

fn resolve_key(spec: &str) -> Result<String, String> {
    if let Some(seq) = base_key_seq(spec) {
        return Ok(seq.to_string());
    }
    if spec.contains('-') {
        return resolve_modifier_combo(spec);
    }
    Err(format!(
        "unknown key <{spec}> (escape literal angle brackets as \\< and \\>)"
    ))
}

// Control code for Ctrl-<key>; Shift makes no difference at the byte level
fn ctrl_code(key: &str, spec: &str) -> Result<String, String> {
    let mut chars = key.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        let ch = ch.to_ascii_lowercase();
        return match ch {
            // Ctrl-letter maps to ASCII 1-26
            'a'..='z' => Ok(char::from(ch as u8 - b'a' + 1).to_string()),
            ' ' => Ok("\x00".to_string()),
            '[' => Ok("\x1b".to_string()), // Ctrl-[ maps to ESC
            ']' => Ok("\x1d".to_string()),
            '\\' => Ok("\x1c".to_string()),
            _ => Err(format!("<{spec}> has no control code")),
        };
    }
    if key == "space" {
        return Ok("\x00".to_string());
    }
    Err(format!("<{spec}> has no control code"))
}

fn resolve_modifier_combo(spec: &str) -> Result<String, String> {
    let parts: Vec<&str> = spec.split('-').collect();
    let (modifiers, key) = parts.split_at(parts.len() - 1);
    let key = key[0];

    let mut has_ctrl = false;
    let mut has_alt = false;
    let mut has_shift = false;

    for m in modifiers {
        match *m {
            "C" | "c" | "Ctrl" | "ctrl" => has_ctrl = true,
            "A" | "a" | "Alt" | "alt" | "M" | "m" | "Meta" | "meta" => has_alt = true,
            "S" | "s" | "Shift" | "shift" => has_shift = true,
            _ => return Err(format!("unknown modifier '{m}' in <{spec}>")),
        }
    }

    let is_single_char = key.chars().count() == 1;

    if has_ctrl {
        let code = if is_single_char || key == "space" {
            ctrl_code(key, spec)?
        } else if has_alt {
            // Ctrl-Alt-<special>: fall back to Alt behaviour
            base_key_seq(key)
                .ok_or_else(|| format!("unknown key '{key}' in <{spec}>"))?
                .to_string()
        } else {
            return Err(format!("<{spec}> has no control code"));
        };
        // Alt prepends ESC
        return Ok(if has_alt { format!("\x1b{code}") } else { code });
    }

    if has_alt {
        let base = if let Some(seq) = base_key_seq(key) {
            seq.to_string()
        } else if is_single_char {
            if has_shift {
                key.to_uppercase()
            } else {
                key.to_string()
            }
        } else {
            return Err(format!("unknown key '{key}' in <{spec}>"));
        };
        return Ok(format!("\x1b{base}"));
    }

    // Shift only
    if key == "tab" {
        return Ok("\x1b[Z".to_string()); // Backtab
    }
    if is_single_char {
        return Ok(key.to_uppercase());
    }
    Err(format!("<{spec}> has no standard escape sequence"))
}

fn parse_type_content(input: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        if remaining.starts_with("\\<") || remaining.starts_with("\\>") {
            result.push_str(&remaining[1..2]);
            remaining = &remaining[2..];
        } else if remaining.starts_with('<') {
            if let Ok((rest, spec)) = parse_key_spec(remaining) {
                result.push_str(&resolve_key(spec)?);
                remaining = rest;
            } else {
                // No closing '>' on the line: literal '<' (e.g. shell redirection)
                result.push('<');
                remaining = &remaining[1..];
            }
        } else {
            let c = remaining.chars().next().unwrap();
            result.push(c);
            remaining = &remaining[c.len_utf8()..];
        }
    }

    Ok(result)
}

// Returns the raw text; special keys are expanded in parse_script so
// unknown key specs can be reported with a line number
fn parse_type(input: &str) -> IResult<&str, Command> {
    let (input, _) = char('$')(input)?;
    let (input, _) = space0(input)?;
    let (input, text) = not_line_ending(input)?;
    Ok((input, Command::Type(text.to_string())))
}

fn parse_line(input: &str) -> IResult<&str, Option<Command>> {
    alt((
        map(parse_directive, Some),
        value(None, parse_comment),
        map(parse_type, Some),
    ))
    .parse(input)
}

pub fn parse_script(input: &str) -> Result<Script, String> {
    let mut commands = Vec::new();

    for (line_num, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        match parse_line(trimmed) {
            Ok((remaining, Some(cmd))) => {
                if !remaining.trim().is_empty() {
                    return Err(format!(
                        "Line {}: Unexpected text after command: '{}'",
                        line_num + 1,
                        remaining
                    ));
                }
                let cmd = match cmd {
                    Command::Type(raw) => Command::Type(
                        parse_type_content(&raw)
                            .map_err(|e| format!("Line {}: {e}", line_num + 1))?,
                    ),
                    other => other,
                };
                commands.push(cmd);
            }
            Ok((_, None)) => {}
            Err(nom::Err::Failure(e)) if e.code == nom::error::ErrorKind::Verify => {
                return Err(format!(
                    "Line {}: invalid directive value '{}': must be a non-negative number",
                    line_num + 1,
                    e.input
                ));
            }
            Err(e) => {
                return Err(format!("Line {}: Parse error: {}", line_num + 1, e));
            }
        }
    }

    Ok(Script { commands })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_speed() {
        let input = "@ speed:0.2";
        let result = parse_speed(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        assert_eq!(cmd, Command::SetSpeed(0.2));
    }

    #[test]
    fn test_parse_jitter() {
        let input = "@ jitter:0.02";
        let result = parse_jitter(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        assert_eq!(cmd, Command::SetJitter(0.02));
    }

    #[test]
    fn test_parse_wait() {
        let input = "@ wait:2.0";
        let result = parse_wait(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        assert_eq!(cmd, Command::Wait(Duration::from_secs_f64(2.0)));
    }

    #[test]
    fn test_parse_shell() {
        let input = "@ shell:/bin/zsh";
        let result = parse_shell(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        assert_eq!(cmd, Command::SetShell("/bin/zsh".to_string()));
    }

    #[test]
    fn test_parse_type() {
        let input = "$ echo hello";
        let result = parse_type(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        assert_eq!(cmd, Command::Type("echo hello".to_string()));
    }

    #[test]
    fn test_parse_type_with_special_keys() {
        assert_eq!(
            parse_type_content("echo hello<ret>"),
            Ok("echo hello\r".to_string())
        );
    }

    #[test]
    fn test_parse_type_with_ctrl() {
        assert_eq!(parse_type_content("<C-c>"), Ok("\x03".to_string()));
    }

    #[test]
    fn test_parse_type_with_escaped() {
        assert_eq!(
            parse_type_content(r"\<not a key\>"),
            Ok("<not a key>".to_string())
        );
    }

    #[test]
    fn test_parse_type_unclosed_bracket_is_literal() {
        // Shell redirection with no '>' on the line stays literal
        assert_eq!(
            parse_type_content("cat < input.txt"),
            Ok("cat < input.txt".to_string())
        );
    }

    #[test]
    fn test_parse_type_unknown_key_is_error() {
        assert!(parse_type_content("<Ret>").is_err());
        assert!(parse_type_content("<D-x>").is_err());
        assert!(parse_type_content("cat <file >out").is_err());
    }

    #[test]
    fn test_parse_shift_tab() {
        assert_eq!(parse_type_content("<S-tab>"), Ok("\x1b[Z".to_string()));
    }

    #[test]
    fn test_parse_size() {
        let input = "@ size:120:40";
        let result = parse_size(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        assert_eq!(cmd, Command::SetSize(120, 40));
    }

    #[test]
    fn test_negative_directive_values_are_errors() {
        let err = parse_script("@ wait:-1").unwrap_err();
        assert!(err.contains("Line 1"), "unexpected error: {err}");
        assert!(parse_script("@ speed:-0.1").is_err());
        assert!(parse_script("@ jitter:-0.5").is_err());
    }

    #[test]
    fn test_parse_script() {
        let input = r"@ speed:0.2
@ jitter:0.02
# This is a comment
$ echo hello
@ wait:1.0
$ ls -la
";
        let result = parse_script(input);
        if let Err(e) = &result {
            eprintln!("Parse error: {e}");
        }
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.commands.len(), 5);
    }

    #[test]
    fn test_parse_alt_with_special_keys() {
        // ESC + carriage return
        assert_eq!(parse_type_content("<A-ret>"), Ok("\x1b\r".to_string()));
        // ESC + space
        assert_eq!(parse_type_content("<A-space>"), Ok("\x1b ".to_string()));
    }

    #[test]
    fn test_parse_ctrl_with_special_keys() {
        assert_eq!(parse_type_content("<C-space>"), Ok("\x00".to_string()));
        assert_eq!(parse_type_content("<C-S-x>"), Ok("\x18".to_string()));
        assert_eq!(parse_type_content("<C-A-c>"), Ok("\x1b\x03".to_string()));
    }

    #[test]
    fn test_example_scripts_parse() {
        let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        for entry in std::fs::read_dir(examples).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "qp") {
                let content = std::fs::read_to_string(&path).unwrap();
                assert!(
                    parse_script(&content).is_ok(),
                    "failed to parse {}",
                    path.display()
                );
            }
        }
    }
}
