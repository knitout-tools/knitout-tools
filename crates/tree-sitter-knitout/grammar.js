/**
 * @file Knitout (.k) is a file format that can represent low-level knitting machine instructions in a machine-independent way.
 * @author Nat Hurtig <nhurtig@acm.org>
 * @license MIT
 */

export default grammar({
  name: "knitout",

  // Spaces and tabs may appear between any two tokens on a line.
  extras: _$ => [
    /[ \t]/
  ],

  rules: {
    // A knitout file is a flat sequence of lines separated by newlines.
    // No magic string or specific structure is required at the file level;
    // later layers is responsible for finding and validating the magic line, making sure headers come before operations, etc.
    //
    // Structure: optional leading blank lines, then optionally a first line
    // followed by zero or more (newline, optional-line) pairs. This guarantees
    // that two $.line nodes are always separated by at least one $._newline, so
    // tokens from adjacent lines can never bleed into each other.
    source_file: $ => seq(
      repeat($._newline),
      optional(seq(
        $.line,
        repeat(seq($._newline, optional($.line)))
      ))
    ),

    _newline: _$ => /\r?\n/,

    // Between two newlines, a line is one of four things.
    // Disambiguation is purely by the lexer's longest-match rule:
    //   ";!"  (2 chars) beats ";" (1 char)  => directive wins over comment
    //   ";;"  (2 chars) beats ";" (1 char)  => header wins over comment
    //   ";"   (1 char)  fallback            => plain comment
    //   anything else                       => operation line (possibly w/ comment)
    line: $ => choice(
      $.tag_comment,
      $.header,
      $.comment,
      $.op_line,
    ),

    // A ;; header line: ;;Machine: Shima Stoll, or even ;;NoColon or just ;;
    // The grammar captures everything after ";;" as raw content and leaves
    // name/colon/value parsing to the weak AST layer. This means no header
    // line ever produces a MISSING or ERROR node, which is important for LSP
    // live-editing scenarios where the colon may not yet be typed.
    header: $ => seq(
      ";;",
      field("content", optional($.line_content))
    ),

    // A ; comment line (or just a lone ;).
    comment: $ => seq(";", field("text", optional($.line_content))),

    // An operation line: opcode followed by zero or more argument tokens,
    // optionally followed by a trailing comment.
    // All semantic interpretation of arguments (direction, needle, carrier, etc.)
    // is handled by the weak AST layer.
    op_line: $ => seq(
      field("operation", $.operation),
      optional(choice(
        field("tag_comment", $.tag_comment),
        field("comment", $.comment),
      ))
    ),

    // An operation is just an opcode token followed by zero or more argument tokens.
    operation: $ => seq(
      field("opcode", $.opcode),
      repeat(field("argument", $.argument))
    ),

    // A ;!source: comment trailing an operation. The ";!source:" token is nine
    // characters (including the space), so the lexer always prefers it over the
    // single-character ";" for comment.
    tag_comment: $ => seq(
      ";!",
      field("source", optional($.line_content))
    ),

    opcode: _$ => /[^\s;]+/,
    tag: _$ => /[^\s]+/,
    argument: _$ => /[^\s;]+/,
    line_content: _$ => /[^\n]+/,
  }
});
