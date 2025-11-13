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

//! Script parser for typecast files
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
    nom::number::complete::double(input)
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

fn parse_special_key(input: &str) -> IResult<&str, String> {
    let (input, _) = char('<')(input)?;
    let (input, key_spec) = take_until(">")(input)?;
    let (input, _) = char('>')(input)?;

    let escape_seq = match key_spec {
        "esc" => "\x1b".to_string(),
        "space" => " ".to_string(),
        "ret" | "return" | "enter" => "\r".to_string(),
        "tab" => "\t".to_string(),
        "backspace" | "bs" => "\x7f".to_string(),
        "F1" => "\x1bOP".to_string(),
        "F2" => "\x1bOQ".to_string(),
        "F3" => "\x1bOR".to_string(),
        "F4" => "\x1bOS".to_string(),
        "F5" => "\x1b[15~".to_string(),
        "F6" => "\x1b[17~".to_string(),
        "F7" => "\x1b[18~".to_string(),
        "F8" => "\x1b[19~".to_string(),
        "F9" => "\x1b[20~".to_string(),
        "F10" => "\x1b[21~".to_string(),
        "F11" => "\x1b[23~".to_string(),
        "F12" => "\x1b[24~".to_string(),
        "up" => "\x1b[A".to_string(),
        "down" => "\x1b[B".to_string(),
        "right" => "\x1b[C".to_string(),
        "left" => "\x1b[D".to_string(),
        "home" => "\x1b[H".to_string(),
        "end" => "\x1b[F".to_string(),
        "pageup" | "pgup" => "\x1b[5~".to_string(),
        "pagedown" | "pgdn" => "\x1b[6~".to_string(),
        "insert" | "ins" => "\x1b[2~".to_string(),
        "delete" | "del" => "\x1b[3~".to_string(),
        spec if spec.contains('-') => parse_modifier_combo(spec),
        _ => format!("<{}>", key_spec),
    };

    Ok((input, escape_seq))
}

fn parse_modifier_combo(spec: &str) -> String {
    let parts: Vec<&str> = spec.split('-').collect();

    if parts.len() < 2 {
        return format!("<{}>", spec);
    }

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
            _ => {}
        }
    }

    // Resolve the base key to its escape sequence first
    let base_key = match key {
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
        // Single character - leave as-is for modifier processing below
        _ if key.len() == 1 => key,
        _ => return format!("<{}>", spec),
    };

    // Apply modifiers to the base key
    if has_ctrl && !has_alt && !has_shift {
        if key.len() == 1 {
            let ch = key.chars().next().unwrap().to_ascii_lowercase();
            if ch.is_ascii_lowercase() {
                // Ctrl-letter maps to ASCII 1-26
                let code = (ch as u8) - b'a' + 1;
                return std::char::from_u32(code as u32).unwrap().to_string();
            } else if ch == ' ' {
                return "\x00".to_string();
            } else if ch == '[' {
                return "\x1b".to_string(); // Ctrl-[ maps to ESC
            } else if ch == ']' {
                return "\x1d".to_string();
            } else if ch == '\\' {
                return "\x1c".to_string();
            }
        } else {
            match key {
                "space" => return "\x00".to_string(),
                // Special keys don't have standard Ctrl combinations
                _ => return format!("<{}>", spec),
            }
        }
    }

    // Alt combinations: prepend ESC to the base key
    if has_alt && !has_ctrl {
        return format!("\x1b{}", base_key);
    }

    // Shift: uppercase single character keys
    if has_shift && !has_ctrl && !has_alt && key.len() == 1 {
        return key.to_uppercase();
    }

    if has_ctrl && has_shift && !has_alt && key.len() == 1 {
        let ch = key.chars().next().unwrap().to_ascii_uppercase();
        if ch.is_ascii_uppercase() {
            let code = (ch as u8) - b'A' + 1;
            return std::char::from_u32(code as u32).unwrap().to_string();
        }
    }

    // Ctrl-Alt: ESC followed by Ctrl-key
    if has_ctrl && has_alt {
        if key.len() == 1 {
            let ch = key.chars().next().unwrap().to_ascii_lowercase();
            if ch.is_ascii_lowercase() {
                let code = (ch as u8) - b'a' + 1;
                return format!("\x1b{}", std::char::from_u32(code as u32).unwrap());
            }
        } else {
            return format!("\x1b{}", base_key);
        }
    }

    format!("<{}>", spec)
}

fn parse_type_content(input: &str) -> String {
    let mut result = String::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        if remaining.starts_with("\\<") || remaining.starts_with("\\>") {
            result.push_str(&remaining[1..2]);
            remaining = &remaining[2..];
        } else if remaining.starts_with('<') {
            match parse_special_key(remaining) {
                Ok((rest, key_seq)) => {
                    result.push_str(&key_seq);
                    remaining = rest;
                }
                Err(_) => {
                    result.push('<');
                    remaining = &remaining[1..];
                }
            }
        } else {
            result.push(remaining.chars().next().unwrap());
            remaining = &remaining[remaining.chars().next().unwrap().len_utf8()..];
        }
    }

    result
}

fn parse_type(input: &str) -> IResult<&str, Command> {
    let (input, _) = char('$')(input)?;
    let (input, _) = space0(input)?;
    let (input, text) = not_line_ending(input)?;

    let processed_text = parse_type_content(text);
    Ok((input, Command::Type(processed_text)))
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
                commands.push(cmd);
            }
            Ok((_, None)) => {}
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
        let input = "$ echo hello<ret>";
        let result = parse_type(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        if let Command::Type(text) = cmd {
            assert_eq!(text, "echo hello\r");
        } else {
            panic!("Expected Type command");
        }
    }

    #[test]
    fn test_parse_type_with_ctrl() {
        let input = "$ <C-c>";
        let result = parse_type(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        if let Command::Type(text) = cmd {
            assert_eq!(text, "\x03"); // Ctrl-C
        } else {
            panic!("Expected Type command");
        }
    }

    #[test]
    fn test_parse_type_with_escaped() {
        let input = r"$ \<not a key\>";
        let result = parse_type(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        if let Command::Type(text) = cmd {
            assert_eq!(text, "<not a key>");
        } else {
            panic!("Expected Type command");
        }
    }

    #[test]
    fn test_parse_script() {
        let input = r#"@ speed:0.2
@ jitter:0.02
# This is a comment
$ echo hello
@ wait:1.0
$ ls -la
"#;
        let result = parse_script(input);
        if let Err(e) = &result {
            eprintln!("Parse error: {}", e);
        }
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.commands.len(), 5);
    }

    #[test]
    fn test_parse_alt_with_special_keys() {
        // Test Alt-Enter
        let input = "$ <A-ret>";
        let result = parse_type(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        if let Command::Type(text) = cmd {
            assert_eq!(text, "\x1b\r"); // ESC + carriage return
        } else {
            panic!("Expected Type command");
        }

        // Test Alt-space
        let input = "$ <A-space>";
        let result = parse_type(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        if let Command::Type(text) = cmd {
            assert_eq!(text, "\x1b "); // ESC + space
        } else {
            panic!("Expected Type command");
        }
    }

    #[test]
    fn test_parse_ctrl_with_special_keys() {
        // Test Ctrl-space
        let input = "$ <C-space>";
        let result = parse_type(input);
        assert!(result.is_ok());
        let (_, cmd) = result.unwrap();
        if let Command::Type(text) = cmd {
            assert_eq!(text, "\x00"); // Ctrl-space
        } else {
            panic!("Expected Type command");
        }
    }
}
