; highlights.scm — knitout (.k)
;
; Node-shape reminder for this grammar:
;   - Rules whose ENTIRE body is a bare terminal (e.g., pause) become
;     leaf NAMED nodes — match them with (rule_name), not a string literal.
;   - Rules that wrap a keyword in seq(...) alongside fields (e.g., in)
;     keep the keyword as an ANONYMOUS child
;     token — match those with a quoted string literal.

; types:
; @attribute
;
; @comment
;
; @constant
; @constant.builtin
;
; @constructor
;
; @embedded
;
; @function
; @function.builtin
;
; @keyword
;
; @module
;
; @number
;
; @operator
;
; @property
;
; @punctuation.bracket
; @punctuation.delimiter
;
; @string
; @string.special
;
; @tag
;
; @type
; @type.builtin
;
; @variable.builtin
; @variable.parameter

; ---------------------------------------------------------------------------
; Magic line: ;!knitout-N
; ---------------------------------------------------------------------------
(magic ";" @comment)
(magic "!knitout-" @keyword)
(version) @number


; ---------------------------------------------------------------------------
; Header lines: ;;Name: value
; ---------------------------------------------------------------------------
";;" @comment
(header
  name: (header_name) @property
  ": " @punctuation.delimiter
  value: (header_value) @constant)

; ---------------------------------------------------------------------------
; Comments
; ---------------------------------------------------------------------------
(comment) @comment
(source_comment ";" @comment)
(source_comment "!source: " @keyword)
(comment_text) @comment

; ---------------------------------------------------------------------------
; Operation keywords (anonymous tokens inside their operation node)
; ---------------------------------------------------------------------------
[
  "in"
  "inhook"
  "releasehook"
  "out"
  "outhook"
  "stitch"
  "rack"
  "knit"
  "tuck"
  "split"
  "drop"
  "amiss"
  "xfer"
  "miss"
] @function.builtin

; pause / direction / bed / slider are bare-terminal leaf nodes, not
; anonymous children — match them directly.
(pause) @function.builtin
(direction) @operator
(bed) @type
(slider) @type

; ---------------------------------------------------------------------------
; Needles, indices, carriers, numbers
; ---------------------------------------------------------------------------
(needle_index) @number
(floating_point) @number
(carrier) @variable

; ---------------------------------------------------------------------------
; Extensions (x-whatever ...)
; ---------------------------------------------------------------------------
(extension_name) @function.macro
(extension_argument) @string

(extension
  name: (extension_name) @function
  .
  (extension_argument) @number
  .
  (#eq? @function "x-speed-number")
  (#match? @number "^[+-]?\\d+$"))

