; highlights.scm — knitout (.k)
;
; With the weak grammar all semantic structure (operation type, argument
; meaning, header name/value) is resolved in the weak-AST layer, not here.
; Highlighting is therefore structural: mark line categories and let the LSP
; layer add semantic tokens on top.

; ---------------------------------------------------------------------------
; Directive lines: ;!knitout-2, ;!source: …
; ---------------------------------------------------------------------------
(directive
  content: (line_content) @keyword)
(directive) @comment

; ---------------------------------------------------------------------------
; Header lines: ;;Name: value
; ---------------------------------------------------------------------------
(header
  content: (line_content) @attribute)
(header) @comment

; ---------------------------------------------------------------------------
; Comment lines and trailing comments
; ---------------------------------------------------------------------------
(comment
  text: (line_content) @comment)
(comment) @comment

; ---------------------------------------------------------------------------
; Source comments trailing an operation: ;!source: …
; ---------------------------------------------------------------------------
(source_comment
  source: (line_content) @string.special)
(source_comment) @comment

; ---------------------------------------------------------------------------
; Operations
; ---------------------------------------------------------------------------
(opcode) @function.builtin
(argument) @variable
