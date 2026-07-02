/**
 * @file Knitout (.k) is a file format that can represent low-level knitting machine instructions in a machine-independent way.
 * @author Nat Hurtig <nhurtig@acm.org>
 * @license MIT
 */

export default grammar({
  name: "knitout",

  // The stuff that can appear between anything. Comments are NOT included here in knitout. Neither are newlines. Just space and tab.
  extras: _$ => [
    /[ \t]/
  ],

  rules: {
    source_file: $ => seq(
      repeat($._newline), // Allow arbitrary _newlines at the start
      field("magic", $.magic), // Knitout files begin with a magic string followed by a linefeed (executive decision: allow the _newline to be optional).
      repeat(choice(seq($._newline, choice($.header, $.line)), $._newline)),
    ),

    // knitout files use the LF character to terminate lines
    _newline: _$ => /\r?\n/,

    // Knitout files begin with a magic string ;!knitout-VERSION followed by a linefeed. VERSION is an integer indicating the current version.
    magic: _$ => /;!knitout-\d+/,


    // Each line in the comment header begins with the string ";;", followed by the header name, followed by the string ": ", followed by the header value.
    header: $ => seq(
      ";;",
      field("name", $.header_name),
      ": ",
      field("value", $.header_value)
    ),
    header_name: _ => /[^:\n]+/,
    header_value: _ => /[^\n]+/,

    // After the comment header, all lines in the file contain an (optional) operation followed by an (optional) comment. The operation is everything up to the first ';' character, the comment is everything after
    line: $ => choice(
      field("comment", $.comment),
      field("operation", $.operation),
      seq(
        field("operation", $.operation),
        field("source_comment", $.source_comment),
      ),
      seq(
        field("operation", $.operation),
        field("comment", $.comment),
      )),

    comment: $ => seq(
      ";",
      field("text", optional($.comment_text))
    ),

    // To aid in debugging, a special type of comment is provided that identifies the source code location corresponding to the current line in the knitout file. These "source comments" always begin with the string "!source: ". Anything after this is considered the source location. 
    source_comment: $ => seq(
      ";",
      token(prec(1, "!source: ")), // token precedence gobbles this up before comments
      field("source", optional($.comment_text))
    ),

    comment_text: _ => /[^\n]+/,

    operation: $ => choice(
      $.in,
      $.inhook,
      $.releasehook,
      $.out,
      $.outhook,

      $.stitch,
      $.rack,

      $.knit,
      $.tuck,
      $.split,
      $.drop,
      $.amiss,
      $.xfer,
      $.miss,

      $.pause,

      $.extension
    ),

    // Parameters
    direction: _ => choice("+", "-"),
    bed: _ => choice("f", "b"),
    slider: _ => token.immediate("s"), // token.immediate disallows preceding whitespace
    needle_index: _ => token.immediate(/-?\d+/),
    needle: $ => seq(
      field("bed", $.bed),
      field("slider", optional($.slider)),
      field("index", $.needle_index)
    ),
    // Carrier names are strings that do not contain the ' '(space), ','(comma), or ';'(semicolon) character
    carrier: _ => /[^ ,;\n]+/,
    pos_carrier_set: $ => repeat1($.carrier),
    // possibly negative. Either positive digits followed by possibly . with digits, or just a dot followed by positive digits
    floating_point: _ => /-?(\d+(\.\d*)?|\.\d+)/,

    in: $ => seq(
      "in",
      field("carriers", $.pos_carrier_set)
    ),

    inhook: $ => seq(
      "inhook",
      field("carriers", $.pos_carrier_set)
    ),

    releasehook: $ => seq(
      "releasehook",
      field("carriers", $.pos_carrier_set)
    ),

    out: $ => seq(
      "out",
      field("carriers", $.pos_carrier_set)
    ),

    outhook: $ => seq(
      "outhook",
      field("carriers", $.pos_carrier_set)
    ),

    stitch: $ => seq(
      "stitch",
      field("loop_pull", $.floating_point),
      field("tail_pull", $.floating_point)
    ),

    rack: $ => seq(
      "rack",
      field("offset", $.floating_point)
    ),

    knit: $ => seq(
      "knit",
      field("direction", $.direction),
      field("needle", $.needle),
      field("carriers", optional($.pos_carrier_set))
    ),

    tuck: $ => seq(
      "tuck",
      field("direction", $.direction),
      field("needle", $.needle),
      field("carriers", optional($.pos_carrier_set))
    ),

    miss: $ => seq(
      "miss",
      field("direction", $.direction),
      field("needle", $.needle),
      field("carriers", optional($.pos_carrier_set))
    ),

    split: $ => seq(
      "split",
      field("direction", $.direction),
      field("source", $.needle),
      field("destination", $.needle),
      field("carriers", optional($.pos_carrier_set))
    ),

    xfer: $ => seq(
      "xfer",
      field("source", $.needle),
      field("destination", $.needle)
    ),

    drop: $ => seq(
      "drop",
      field("needle", $.needle)
    ),

    amiss: $ => seq(
      "amiss",
      field("needle", $.needle)
    ),

    pause: _ => "pause",

    extension: $ => seq(
      field("name", $.extension_name),
      repeat($.extension_argument)
    ),

    // no whitespace or comments
    extension_name: _ => /x-[^\s;]+/,
    extension_argument: _ => /[^\s;]+/,
  }
});
