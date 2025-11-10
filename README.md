# typecast

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
git clone https://github.com/waddie/typecast.git
cd typecast
cargo build --release
```

## Usage

Create a script file (`.qp` extension) with your keyboard sequences:

```
@ speed:0.1
@ jitter:0.03

# This is a comment
$ echo "Hello from typecast!"<ret>

@ wait:1.0

$ ls -la<ret>

@ wait:0.5

$ # Type commands with special keys
$ vim example.txt<ret>
$ iHello World!<esc>:wq<ret>
```

Run the script:

```sh
typecast script.qp
```

By default, typecast uses your current shell (`$SHELL`). To use a different shell:

```sh
typecast --shell /bin/bash script.qp
```

Or use the shell directive in your script:

```qp
@ shell:/bin/bash
$ echo "Running in: $SHELL"<ret>
```

Record with asciinema:

```sh
asciinema rec demo.cast -c "typecast script.qp"
```

## Script Format

### Directives (@ lines)

- `@ speed:N` - Set time between keystrokes in seconds (default: 0.1)
- `@ jitter:N` - Set random variation as fraction of speed (default: 0.0)
- `@ wait:N` - Pause for N seconds before continuing
- `@ shell:PATH` - Set shell to use (defaults to `$SHELL`, must come before any typing commands)

### Comments (# lines)

Lines starting with `#` are ignored.

### Typing ($ lines)

Lines starting with `$` are typed into the terminal:

```
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

## License

GNU AGPL v3 - See [LICENSE.md](LICENSE.md)
