/**
 * @file Tree-sitter grammar for quipu .qp scripts
 * @author Tom Waddington <tom@waddington.dev>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "quipu",

  extras: $ => [],

  rules: {
    source_file: $ => repeat($._line),

    _line: $ => seq(
      optional(choice(
        $.directive,
        $.comment,
        $.type_command
      )),
      '\n'
    ),

    // Directives start with @
    directive: $ => seq(
      '@',
      optional(/[ \t]+/),
      choice(
        $.speed_directive,
        $.jitter_directive,
        $.wait_directive,
        $.shell_directive,
        $.size_directive
      ),
      optional($.inline_comment)
    ),

    speed_directive: $ => seq(
      'speed:',
      $.float
    ),

    jitter_directive: $ => seq(
      'jitter:',
      $.float
    ),

    wait_directive: $ => seq(
      'wait:',
      $.float
    ),

    shell_directive: $ => seq(
      'shell:',
      $.shell_path
    ),

    size_directive: $ => seq(
      'size:',
      $.integer,
      ':',
      $.integer
    ),

    // Comments start with #
    comment: $ => seq(
      '#',
      optional($.comment_text)
    ),

    inline_comment: $ => seq(
      /[ \t]+/,
      '#',
      optional($.comment_text)
    ),

    comment_text: $ => /[^\n]+/,

    // Type commands start with $
    type_command: $ => seq(
      '$',
      optional(/[ \t]+/),
      optional($.type_content)
    ),

    type_content: $ => repeat1(choice(
      $.special_key,
      $.escaped_bracket,
      $.text
    )),

    // Special keys: <key>, <C-x>, <A-x>, <S-x>, <C-S-x>, etc.
    special_key: $ => seq(
      '<',
      $.key_spec,
      '>'
    ),

    key_spec: $ => choice(
      // Modifier combinations must be tried first
      $.modifier_combo,
      // Named keys
      'esc',
      'space',
      'ret', 'return', 'enter',
      'tab',
      'backspace', 'bs',
      // Function keys
      /F([1-9]|1[0-2])/,
      // Arrow keys
      'up', 'down', 'left', 'right',
      // Other special keys
      'home', 'end',
      'pageup', 'pgup', 'pagedown', 'pgdn',
      'insert', 'ins',
      'delete', 'del'
    ),

    modifier_combo: $ => token(seq(
      repeat1(choice(
        /[CcAaMmSs]-/,
        /Ctrl-/,
        /ctrl-/,
        /CTRL-/,
        /Alt-/,
        /alt-/,
        /ALT-/,
        /Meta-/,
        /meta-/,
        /META-/,
        /Shift-/,
        /shift-/,
        /SHIFT-/
      )),
      choice(
        /[a-zA-Z0-9]/,
        'esc', 'space', 'ret', 'return', 'enter',
        'tab', 'backspace', 'bs',
        /F([1-9]|1[0-2])/,
        'up', 'down', 'left', 'right',
        'home', 'end',
        'pageup', 'pgup', 'pagedown', 'pgdn',
        'insert', 'ins', 'delete', 'del',
        /[\[\]\\]/
      )
    )),

    // Escaped brackets
    escaped_bracket: $ => choice(
      /\\</,
      /\\>/
    ),

    // Regular text (anything except special characters)
    text: $ => /[^<\\\n]+/,

    // Primitives
    float: $ => /[+-]?([0-9]*[.])?[0-9]+/,

    integer: $ => /[0-9]+/,

    shell_path: $ => /[^\n#]+/
  }
});
