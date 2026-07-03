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
    magic: $ => seq(
      ';',
      token.immediate('!knitout-'),
      field('version', $.version)
    ),

    version: _$ => token.immediate(/\d+/),

    // Each line in the comment header begins with the string ";;", followed by the header name, followed by the string ": ", followed by the header value.
    header: $ => seq(";;", choice(
      $.yarn_header,
      $.machine_header,
      $.gauge_header,
      $.width_header,
      $.position_header,
      $.carriers_header,
      $.extension_header
    )),

    yarn_header: $ => seq(
      "Yarn-",
      field("carrier", $.carrier),
      ":",
      field("value", $.header_value)
    ),

    machine_header: $ => seq(
      "Machine",
      ":",
      field("value", $.header_value)
    ),

    gauge_header: $ => seq(
      "Gauge",
      ":",
      field("value", $.nonneg_integer)
    ),

    width_header: $ => seq(
      "Width",
      ":",
      field("value", $.nonneg_integer)
    ),

    position_header: $ => seq(
      "Position",
      ":",
      field("value", $.position_value)
    ),

    position_value: _$ => choice(
      "Left",
      "Center",
      "Right",
      "Keep"
    ),

    nonneg_integer: _$ => /[+]?\d+/,

    carriers_header: $ => seq(
      "Carriers",
      ":",
      field("value", optional($.carriers_value))
    ),

    carriers_value: $ => repeat1(
      choice(
        $.rail_definition,
        $.carrier
      )
    ),

    rail_definition: $ => seq(
      field("left", $.rail_side),
      token.immediate(";"),
      field("right", $.rail_side)
    ),

    rail_side: $ => seq(
      $.carrier,
      repeat(seq(token.immediate(","), $.carrier))
    ),

    extension_header: $ => seq(
      field("name", $.extension_header_name),
      ": ",
      field("value", $.header_value)
    ),
    extension_header_name: _ => /X-[^:\n]+/,
    header_value: _ => /[^\n]+/,

    // After the comment header, all lines in the file contain an (optional) operation followed by an (optional) comment. The operation is everything up to the first ';' character, the comment is everything after
    line: $ => choice(
      field("comment", $.comment),
      field("operation", $._operation),
      seq(
        field("operation", $._operation),
        field("source_comment", $.source_comment),
      ),
      seq(
        field("operation", $._operation),
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

    _operation: $ => choice(
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

      $.extension,
    ),

    // Parameters
    direction: _ => choice("+", "-"),
    bed: _ => choice("f", "b"),
    slider: _ => token.immediate("s"), // token.immediate disallows preceding whitespace
    needle_index: _$ => token.immediate(/[+-]?\d+/),
    needle: $ => seq(
      field("bed", $.bed),
      field("slider", optional($.slider)),
      field("index", $.needle_index)
    ),
    // Carrier names are strings that do not contain the ' '(space), ','(comma), or ';'(semicolon) character
    carrier: _ => /[^ ,;:\n]+/,
    pos_carrier_set: $ => repeat1($.carrier),
    // possibly negative or for some reason marked positive. Either positive digits followed by possibly . with digits, or just a dot followed by positive digits
    rational: _ => /[+-]?(\d+(\.\d*)?|\.\d+)/,

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
      field("before_pull", $.rational),
      field("after_pull", $.rational)
    ),

    rack: $ => seq(
      "rack",
      field("offset", $.rational)
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


    extension: $ => choice(
      $.x_stitch_number,
      $.x_speed_number,
      $.x_sub_roller_number,
      $.x_presser_mode,
      $.x_xfer_stitch_number,
      $.x_end_pass,
      $.x_vis_color,
      $.x_loop_in,
      $.x_yarn_in,
      $.x_yarn_out,
      $.x_end,
      $.x_roller_advance,
      $.x_add_roller_advance,
      $.x_carrier_spacing,
      $.x_carrier_stopping_distance,
      $.x_xfer_style,
      $.unknown_extension,
    ),

    unknown_extension: $ => seq(
      field("name", $.extension_name),
      repeat($.extension_argument)
    ),

    x_stitch_number: $ => seq(
      "x-stitch-number",
      field("number", $.nonneg_integer)
    ),

    x_speed_number: $ => seq(
      "x-speed-number",
      field("number", $.nonneg_integer)
    ),

    x_sub_roller_number: $ => seq(
      "x-sub-roller-number",
      field("number", $.nonneg_integer)
    ),

    x_presser_mode: $ => seq(
      "x-presser-mode",
      field("mode", $.presser_mode)
    ),
    presser_mode: _$ => choice(
      "off",
      "auto",
      "on"
    ),

    x_xfer_stitch_number: $ => seq(
      "x-xfer-stitch-number",
      field("number", $.nonneg_integer)
    ),

    x_end_pass: _$ => "x-end-pass",

    x_vis_color: $ => seq(
      "x-vis-color",
      field("color", $.color),
      field("carriers", $.pos_carrier_set)
    ),
    color: _$ => /[^\s]+/,

    x_loop_in: $ => seq(
      "x-loop-in",
      field("direction", $.direction),
      field("needle", $.needle),
      field("carriers", $.pos_carrier_set)
    ),

    x_yarn_in: $ => seq(
      "x-yarn-in",
      field("direction", $.direction),
      field("needle", $.needle),
      field("carriers", $.pos_carrier_set)
    ),

    x_yarn_out: $ => seq(
      "x-yarn-out",
      field("direction", $.direction),
      field("needle", $.needle),
      field("carriers", $.pos_carrier_set)
    ),

    x_end: _$ => "x-end",

    x_roller_advance: $ => seq(
      "x-roller-advance",
      field("amount", $.rational)
    ),

    x_add_roller_advance: $ => seq(
      "x-add-roller-advance",
      field("amount", $.rational)
    ),

    x_carrier_spacing: $ => seq(
      "x-carrier-spacing",
      field("amount", $.nonneg_integer)
    ),

    x_carrier_stopping_distance: $ => seq(
      "x-carrier-stopping-distance",
      field("amount", $.nonneg_half_integer)
    ),
    nonneg_half_integer: _$ => /[+]?\d*(\.5)/,

    x_xfer_style: $ => seq(
      "x-xfer-style",
      field("style", $.xfer_style)
    ),
    xfer_style: _$ => choice(
      "four-pass",
      "two-pass"
    ),

    // no whitespace or comments
    extension_name: _ => /x-[^\s;]+/,
    extension_argument: _ => /[^\s;]+/,
  }
});
