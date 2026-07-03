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
(extension_header
  name: (extension_header_name) @property
  ": " @punctuation.delimiter
  value: (header_value) @string)

; ---------------------------------------------------------------------------
; Specific headers
; ---------------------------------------------------------------------------
(yarn_header
  "Yarn-" @property
  ":" @punctuation.delimiter
  value: (header_value) @constant)

(machine_header
  "Machine" @property
  ":" @punctuation.delimiter
  value: (header_value) @constant)

(gauge_header
  "Gauge" @property
  ":" @punctuation.delimiter)

(width_header
  "Width" @property
  ":" @punctuation.delimiter)

(position_header
  "Position" @property
  ":" @punctuation.delimiter)
(position_value) @constant.builtin

(carriers_header
  "Carriers" @property
  ":" @punctuation.delimiter)

(rail_definition
  left: (rail_side)
  ";" @punctuation.delimiter
  right: (rail_side))

(rail_side
  "," @punctuation.delimiter)

(carrier) @variable

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
(rational) @number
(carrier) @variable

; ---------------------------------------------------------------------------
; Extensions (x-whatever ...)
; ---------------------------------------------------------------------------
[
 "x-stitch-number"
 "x-speed-number"
 "x-sub-roller-number"
 "x-presser-mode"
 "x-xfer-stitch-number"
 "x-vis-color"
 "x-loop-in"
 "x-yarn-in"
 "x-yarn-out"
 "x-roller-advance"
 "x-add-roller-advance"
 "x-carrier-spacing"
 "x-carrier-stopping-distance"
 "x-xfer-style"
] @function.macro
(nonneg_integer) @number
(nonneg_half_integer) @number

(x_end_pass) @function.macro
(x_end) @function.macro

(color) @constant
(presser_mode) @constant.builtin
(xfer_style) @constant.builtin

(extension_name) @function.macro
(extension_argument) @string

