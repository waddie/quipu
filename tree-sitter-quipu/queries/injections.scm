; Language injections for quipu scripts

; Inject comment highlighting into comment nodes
((comment) @injection.content
 (#set! injection.language "comment"))

((inline_comment) @injection.content
 (#set! injection.language "comment"))
