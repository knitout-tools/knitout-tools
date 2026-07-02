; highlights.scm — knitout (.k)
;
; Node-shape reminder for this grammar:
;   - Rules whose ENTIRE body is a bare terminal (magic, pause, direction,
;     bed, slider, needle_index, carrier, floating_point, extension_name,
;     extension_argument, header_name, header_value, comment_text) become
;     leaf NAMED nodes — match them with (rule_name), not a string literal.
;   - Rules that wrap a keyword in seq(...) alongside fields (in, inhook,
;     releasehook, out, outhook, stitch, rack, knit, tuck, split, drop,
;     amiss, xfer, miss, extension) keep the keyword as an ANONYMOUS child
;     token — match those with a quoted string literal.

; ---------------------------------------------------------------------------
; Magic line: ;!knitout-N
; ---------------------------------------------------------------------------
(magic) @constant.builtin

; ---------------------------------------------------------------------------
; Header lines: ;;Name: value
; ---------------------------------------------------------------------------
";;" @punctuation.special
(header
  name: (header_name) @property
  ": " @punctuation.delimiter
  value: (header_value) @string)

; ---------------------------------------------------------------------------
; Comments
; ---------------------------------------------------------------------------
(comment) @comment
(source_comment) @comment
(comment_text) @comment
"!source: " @comment

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
] @keyword

; pause / direction / bed / slider are bare-terminal leaf nodes, not
; anonymous children — match them directly.
(pause) @keyword
(direction) @operator
(bed) @type.builtin
(slider) @type.builtin.modifier

; ---------------------------------------------------------------------------
; Needles, indices, carriers, numbers
; ---------------------------------------------------------------------------
(needle_index) @number
(floating_point) @number
(carrier) @variable.special

; ---------------------------------------------------------------------------
; Extensions (x-whatever ...)
; ---------------------------------------------------------------------------
(extension_name) @function.macro
(extension_argument) @string

; ---------------------------------------------------------------------------
; Field-level fallbacks (helps themes that key off field names too)
; ---------------------------------------------------------------------------
(needle bed: (bed) @type.builtin)
(needle slider: (slider) @type.builtin.modifier)
(needle index: (needle_index) @number)
