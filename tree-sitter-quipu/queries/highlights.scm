; Directives
;-----------

"@" @keyword.directive

(speed_directive
  "speed:" @keyword.directive
  (float) @constant.numeric.float)

(jitter_directive
  "jitter:" @keyword.directive
  (float) @constant.numeric.float)

(wait_directive
  "wait:" @keyword.directive
  (float) @constant.numeric.float)

(shell_directive
  "shell:" @keyword.directive
  (shell_path) @string.special.path)

(size_directive
  "size:" @keyword.directive
  (integer) @constant.numeric.integer
  (integer) @constant.numeric.integer)

; Comments
;---------

(comment) @comment
(comment_text) @comment
(inline_comment) @comment

"#" @punctuation.special

; Type commands
;-------------

"$" @keyword.directive

(text) @string

; Special keys
;-------------

(special_key
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

(key_spec) @constant.builtin

; Named keys
[
  "esc"
  "space"
  "ret"
  "return"
  "enter"
  "tab"
  "backspace"
  "bs"
  "up"
  "down"
  "left"
  "right"
  "home"
  "end"
  "pageup"
  "pgup"
  "pagedown"
  "pgdn"
  "insert"
  "ins"
  "delete"
  "del"
] @constant.builtin.boolean

; Function keys
(key_spec
  (modifier_combo) @keyword.operator)

; Escaped brackets
;----------------

(escaped_bracket) @constant.character.escape

; Operators
;----------

":" @operator

; Numbers
;--------

(float) @constant.numeric.float
(integer) @constant.numeric.integer
