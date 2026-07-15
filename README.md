# quipu

Yet another terminal keyboard entry scripting tool – good for recording terminal demos with tools like asciinema.

## Features

- Script keyboard sequences for terminal demos
- Control typing speed and add natural jitter
- Support for special keys and modifier combinations (Ctrl, Alt, Shift)
- Works with terminal recording tools like asciinema

## Installation

```sh
cargo install --path .
```

Or build from source:

```sh
git clone https://github.com/waddie/quipu.git
cd quipu
cargo build --release
```

## Usage

Create a script file (`.qp` extension) with your keyboard sequences:

```quipu
@ speed:0.1
@ jitter:0.03

# This is a comment
$ echo "Hello from quipu!"<ret>

@ wait:1.0

$ ls -la<ret>

@ wait:0.5

$ # Type commands with special keys
$ vim example.txt<ret>
$ iHello World!<esc>:wq<ret>
```

Run the script:

```sh
quipu script.qp
```

Pass `-q`/`--quiet` to suppress the informational status messages.

By default, quipu uses your current shell (`$SHELL`). To use a different shell:

```sh
quipu --shell /bin/bash script.qp
```

Or use the shell directive in your script:

```quipu
@ shell:/bin/bash
$ echo "Running in: $SHELL"<ret>
```

Record with asciinema:

```sh
asciinema rec demo.cast -c "quipu script.qp"
```

## Script Format

### Directives (@ lines)

- `@ speed:N` - Set time between keystrokes in seconds (default: 0.1)
- `@ jitter:N` - Set random variation as fraction of speed (default: 0.0)
- `@ wait:N` - Pause for N seconds before continuing
- `@ shell:PATH` - Set shell to use (defaults to `$SHELL`, must come before any typing commands; a `--shell` CLI argument takes priority)
- `@ size:COLS:ROWS` - Set terminal size (default: 80x24, must come before any typing commands)
- `@ capture:PATH` - Capture the current screen to `PATH`, including escape codes. Consider a preceding `wait` to allow the screen to settle.

Directive values must be non-negative numbers.

### Comments (# lines)

Lines starting with `#` are ignored.

### Typing ($ lines)

Lines starting with `$` are typed into the terminal:

```quipu
$ echo "regular text"
```

### Special Keys

Use angle brackets for special keys:

**Basic keys**:

- `<esc>` - Escape
- `<ret>`, `<return>`, `<enter>` - Return/Enter
- `<space>` - Space
- `<tab>` - Tab
- `<backspace>`, `<bs>` - Backspace

**Function keys**:

- `<F1>` through `<F12>`

**Arrow keys**:

- `<up>`, `<down>`, `<left>`, `<right>`

**Navigation**:

- `<home>`, `<end>`
- `<pageup>`, `<pagedown>`
- `<insert>`, `<delete>`

### Modifier Keys

Use modifier prefixes with a dash:

- `<C-x>` or `<Ctrl-x>` - Ctrl+X
- `<A-x>` or `<Alt-x>` - Alt+X
- `<S-x>` or `<Shift-x>` - Shift+X
- `<C-S-x>` - Ctrl+Shift+X
- `<S-tab>` - Backtab

Examples:

```
$ <C-c>           # Send Ctrl-C
$ <C-d>           # Send Ctrl-D (EOF)
$ <A-f>           # Alt-F (forward word in bash)
$ <C-X><C-S>      # Ctrl-X Ctrl-S (save in emacs)
```

### Escaping

Use backslash to escape angle brackets:

```
$ echo "Literal \<angle\> brackets"
```

An unrecognized `<...>` sequence is a parse error, so unescaped angle bracket
pairs must be escaped. A lone `<` with no `>` later on the line (e.g. shell
redirection `cat < file`) is typed literally.

## License

GNU AGPL v3 - See [LICENSE.md](LICENSE.md)
