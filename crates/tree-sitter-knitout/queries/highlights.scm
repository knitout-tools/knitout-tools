; highlights.scm — knitout (.k)
;
; The grammar is structural (every operation is opcode + argument*).
; Semantic colours are recovered with positional anchors (.) and string
; predicates (#eq? / #any-of? / #match?) on the flat node tree.
;
; Priority rule: among patterns whose first capture falls at the same byte
; offset, the pattern with the HIGHER index (later in this file) wins.
; A pattern whose first capture starts at a LATER byte beats all patterns
; whose first capture starts earlier, regardless of pattern index.
;
; Consequence: "(operation (argument) @variable)" has its first capture at
; the argument's byte offset (later than the operation start), so it beats
; operation-context number/type patterns whose first capture is the opcode
; at byte 0.  The fix is to avoid catch-all argument patterns entirely and
; enumerate carrier positions explicitly per opcode, always including the
; opcode capture (". (opcode) @_name") so the match's first capture is
; anchored at byte 0 of the operation.

; ---------------------------------------------------------------------------
; Opcode — fallback (overridden by later patterns)
; ---------------------------------------------------------------------------
(opcode) @function.builtin

; ---------------------------------------------------------------------------
; Directive lines  ;!knitout-N
; ---------------------------------------------------------------------------
(directive) @comment
(directive
  content: (line_content) @keyword)

; ---------------------------------------------------------------------------
; Header lines  ;;Name: value
; ---------------------------------------------------------------------------
(header) @comment
(header
  content: (line_content) @attribute)

; ---------------------------------------------------------------------------
; Comments
; ---------------------------------------------------------------------------
(comment) @comment
(comment
  text: (line_content) @comment)

; ---------------------------------------------------------------------------
; Source comments  ;!source: path:line
; ---------------------------------------------------------------------------
(source_comment) @comment
(source_comment ";!source:" @keyword)

; ---------------------------------------------------------------------------
; Opcode — known extensions (listed explicitly + ^x- catch-all)
; ---------------------------------------------------------------------------
((opcode) @function.macro
 (#match? @function.macro "^x-"))

((opcode) @function.macro
 (#any-of? @function.macro
   "x-stitch-number" "x-speed-number" "x-sub-roller-number"
   "x-presser-mode" "x-xfer-stitch-number" "x-vis-color"
   "x-loop-in" "x-yarn-in" "x-yarn-out"
   "x-roller-advance" "x-add-roller-advance"
   "x-carrier-spacing" "x-carrier-stopping-distance"
   "x-xfer-style" "x-end-pass" "x-end"))

; ---------------------------------------------------------------------------
; Opcode — standard operations
; ---------------------------------------------------------------------------
((opcode) @function.builtin
 (#any-of? @function.builtin
   "in" "inhook" "releasehook" "out" "outhook"
   "stitch" "rack"
   "knit" "tuck" "split" "drop" "amiss" "xfer" "miss"
   "pause"))

; ---------------------------------------------------------------------------
; Arguments — direction (+/-) as first arg
; ---------------------------------------------------------------------------
(operation
  . (opcode) @function.builtin
  . (argument) @operator
  (#any-of? @function.builtin "knit" "tuck" "miss" "split"))

(operation
  . (opcode) @function.macro
  . (argument) @operator
  (#any-of? @function.macro "x-loop-in" "x-yarn-in" "x-yarn-out"))

; ---------------------------------------------------------------------------
; Arguments — needle  (bed + optional slider + index, e.g. f5 bs-3)
; ---------------------------------------------------------------------------
; knit/tuck/miss  →  arg[1]  (after direction)
(operation
  . (opcode) @function.builtin
  . (argument)
  . (argument) @type
  (#any-of? @function.builtin "knit" "tuck" "miss"))

; x-loop-in/x-yarn-in/x-yarn-out  →  arg[1]
(operation
  . (opcode) @function.macro
  . (argument)
  . (argument) @type
  (#any-of? @function.macro "x-loop-in" "x-yarn-in" "x-yarn-out"))

; drop/amiss  →  arg[0]
(operation
  . (opcode) @function.builtin
  . (argument) @type
  (#any-of? @function.builtin "drop" "amiss"))

; xfer: source needle (arg[0])
(operation
  . (opcode) @function.builtin
  . (argument) @type
  (#eq? @function.builtin "xfer"))

; xfer: dest needle (arg[1])
(operation
  . (opcode) @function.builtin
  . (argument)
  . (argument) @type
  (#eq? @function.builtin "xfer"))

; split: source needle (arg[1], after direction)
(operation
  . (opcode) @function.builtin
  . (argument)
  . (argument) @type
  (#eq? @function.builtin "split"))

; split: dest needle (arg[2])
(operation
  . (opcode) @function.builtin
  . (argument)
  . (argument)
  . (argument) @type
  (#eq? @function.builtin "split"))

; ---------------------------------------------------------------------------
; Arguments — rack (one float) and stitch (two floats)
; ---------------------------------------------------------------------------
(operation
  . (opcode) @function.builtin
  . (argument) @number
  (#any-of? @function.builtin "rack" "stitch"))

(operation
  . (opcode) @function.builtin
  . (argument)
  . (argument) @number
  (#eq? @function.builtin "stitch"))

; ---------------------------------------------------------------------------
; Arguments — carrier variables, enumerated per opcode
;
; Each pattern includes ". (opcode) @_name" so the first capture lands at
; byte 0 of the operation.  The trailing non-anchored "(argument) @variable"
; generates one match per qualifying argument child.
; ---------------------------------------------------------------------------

; in / out / inhook / outhook / releasehook — carriers at each argument position
; Fully-anchored per position because tree-sitter's non-anchored matching
; generates only one match (the last child) per operation node.
(operation . (opcode) @function.builtin . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))
(operation . (opcode) @function.builtin . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) . (argument) @variable
  (#any-of? @function.builtin "in" "out" "inhook" "outhook" "releasehook"))

; knit / tuck / miss — arg[2+]  (arg[0]=direction, arg[1]=needle)
(operation
  . (opcode) @function.builtin
  . (argument)
  . (argument)
  (argument) @variable
  (#any-of? @function.builtin "knit" "tuck" "miss"))

; split — arg[3+]  (arg[0]=direction, arg[1]=src needle, arg[2]=dst needle)
(operation
  . (opcode) @function.builtin
  . (argument)
  . (argument)
  . (argument)
  (argument) @variable
  (#eq? @function.builtin "split"))

; x-loop-in / x-yarn-in / x-yarn-out — arg[2+]
(operation
  . (opcode) @function.macro
  . (argument)
  . (argument)
  (argument) @variable
  (#any-of? @function.macro "x-loop-in" "x-yarn-in" "x-yarn-out"))

; x-vis-color — arg[1+]  (arg[0] is the colour literal)
(operation
  . (opcode) @function.macro
  . (argument)
  (argument) @variable
  (#eq? @function.macro "x-vis-color"))

; ---------------------------------------------------------------------------
; Arguments — unknown x- extensions: all arguments are variables
;
; Placed before the specific x- argument patterns so that known extensions'
; types win via higher pattern index.
; ---------------------------------------------------------------------------
(operation
  . (opcode) @function.macro
  (argument) @variable
  (#match? @function.macro "^x-"))

; ---------------------------------------------------------------------------
; Arguments — single-number known extensions
; (appear after the x- catch-all so they take priority)
; ---------------------------------------------------------------------------
(operation
  . (opcode) @function.macro
  . (argument) @number
  (#any-of? @function.macro
    "x-stitch-number" "x-speed-number" "x-sub-roller-number"
    "x-xfer-stitch-number" "x-roller-advance" "x-add-roller-advance"
    "x-carrier-spacing" "x-carrier-stopping-distance"))

; ---------------------------------------------------------------------------
; Arguments — x-presser-mode and x-xfer-style values
; ---------------------------------------------------------------------------
(operation
  . (opcode) @function.macro
  . (argument) @constant.builtin
  (#any-of? @function.macro "x-presser-mode" "x-xfer-style"))

; ---------------------------------------------------------------------------
; Arguments — x-vis-color: first arg is a colour literal
; ---------------------------------------------------------------------------
(operation
  . (opcode) @function.macro
  . (argument) @constant
  (#eq? @function.macro "x-vis-color"))
