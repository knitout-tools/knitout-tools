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

; headers

(header
  ";;" @comment)
(header
  (line_content) @attribute)


(comment) @comment

(tag_comment
  ";!" @comment)
(tag_comment
  (line_content) @tag)

(opcode) @function

; IN/OUT/INHOOK/OUTHOOK/RELEASEHOOK

((operation
   opcode: (opcode) @function
   (argument)* @variable)
  (#any-of? @function "in" "out" "inhook" "outhook" "releasehook"))

; STITCH

((operation
   opcode: (opcode) @function
   (argument) @number)
  (#any-of? @function "stitch"))

((operation
   opcode: (opcode) @function
   (argument) @number
   (argument) @number)
  (#any-of? @function "stitch"))

; 1 NUMBER

((operation
   opcode: (opcode) @function
   (argument) @number)
  (#any-of? @function "rack" "x-stitch-number" "x-speed-number" "x-sub-roller-number" "x-xfer-stitch-number" "x-roller-advance" "x-add-roller-advance" "x-carrier-spacing" "x-carrier-stopping-distance"))

; DIR, NEEDLE, CARRIERS

((operation
   opcode: (opcode) @function
   .
   (argument) @operator)
  (#any-of? @function "knit" "tuck" "miss" "x-loop-in" "x-yarn-in" "x-yarn-out"))

((operation
   opcode: (opcode) @function
   .
   (argument) @operator
   .
   (argument) @type
   .
   (argument)* @variable)
  (#any-of? @function "knit" "tuck" "miss" "x-loop-in" "x-yarn-in" "x-yarn-out"))

; SPLIT

((operation opcode: (opcode) @function)
  (#any-of? @function "split"))

((operation
   opcode: (opcode) @function
   .
   (argument) @operator)
  (#any-of? @function "split"))


((operation
   opcode: (opcode) @function
   .
   (argument) @operator
   .
   (argument) @type
   .
   (argument) @type)
  (#any-of? @function "split"))


((operation
   opcode: (opcode) @function
   .
   (argument) @operator
   .
   (argument) @type
   .
   (argument) @type
   .
   (argument)* @variable)
  (#any-of? @function "split"))

; DROP/AMISS

((operation
   opcode: (opcode) @function
   (argument) @type)
  (#any-of? @function "drop" "amiss"))

; XFER

((operation
   opcode: (opcode) @function
   (argument) @type)
  (#any-of? @function "xfer"))

((operation
   opcode: (opcode) @function
   (argument) @type
   (argument) @type)
  (#any-of? @function "xfer"))

; KEYWORD

((operation
   opcode: (opcode) @function
   (argument) @constant.builtin)
  (#any-of? @function "x-presser-mode" "x-xfer-style"))

; STRING, CARRIERS*

((operation
   opcode: (opcode) @function
   .
   (argument) @string)
  (#any-of? @function "x-vis-color"))

((operation
   opcode: (opcode) @function
   .
   (argument) @string
   .
   (argument)* @variable)
  (#any-of? @function "x-vis-color"))


