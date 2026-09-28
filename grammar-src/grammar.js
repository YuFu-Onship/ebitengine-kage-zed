/**
 * Tree-sitter grammar for Kage, the shader language of Ebitengine.
 *
 * Kage is a Go-like language, so this grammar follows the structure of
 * tree-sitter-go, reduced to the subset that Kage supports (no structs,
 * pointers, channels, maps, switch/select statements, etc.).
 */

module.exports = grammar({
  name: 'kage',

  word: $ => $.identifier,

  extras: $ => [
    /\s/,
    $.comment,
  ],

  rules: {
    source_file: $ => repeat(choice(
      $.package_clause,
      $.const_declaration,
      $.var_declaration,
      $.function_declaration,
    )),

    package_clause: $ => seq(
      'package',
      field('name', $.identifier),
    ),

    const_declaration: $ => seq(
      'const',
      commaSep1($.const_spec),
    ),

    const_spec: $ => seq(
      field('name', $.identifier),
      optional(seq('=', field('value', $._expression))),
    ),

    var_declaration: $ => seq(
      'var',
      $.var_spec,
    ),

    var_spec: $ => seq(
      field('name', commaSep1($.identifier)),
      field('type', $._type),
      optional(seq('=', field('value', $._expression))),
    ),

    function_declaration: $ => seq(
      'func',
      field('name', $.identifier),
      field('parameters', $.parameter_list),
      optional(field('result', $._type)),
      field('body', $.block),
    ),

    parameter_list: $ => seq(
      '(',
      optional(seq(commaSep1($.parameter_declaration), optional(','))),
      ')',
    ),

    parameter_declaration: $ => seq(
      optional(field('name', $.identifier)),
      field('type', $._type),
    ),

    _type: $ => choice(
      $.type_identifier,
      $.array_type,
    ),

    type_identifier: $ => $.identifier,

    array_type: $ => seq(
      '[',
      optional(field('length', choice($.identifier, $.number))),
      ']',
      field('element', $._type),
    ),

    block: $ => seq(
      '{',
      repeat($._statement),
      '}',
    ),

    _statement: $ => choice(
      $.short_var_declaration,
      $.assignment_statement,
      $.inc_dec_statement,
      $.var_declaration,
      $.return_statement,
      $.if_statement,
      $.for_statement,
      $.expression_statement,
      $.block,
    ),

    inc_dec_statement: $ => prec.left(seq(
      field('operand', $._lhs_expression),
      field('operator', choice('++', '--')),
    )),

    expression_statement: $ => $._expression,

    return_statement: $ => prec.right(seq(
      'return',
      optional(commaSep1($._expression)),
    )),

    if_statement: $ => prec.right(seq(
      'if',
      field('condition', $._expression),
      field('consequence', $.block),
      optional(field('alternative', $.else_clause)),
    )),

    else_clause: $ => seq(
      'else',
      choice($.if_statement, $.block),
    ),

    for_statement: $ => seq(
      'for',
      optional(choice($.for_clause, $._expression)),
      field('body', $.block),
    ),

    for_clause: $ => seq(
      optional(field('init', choice($.short_var_declaration, $.assignment_statement, $._expression))),
      ';',
      optional(field('condition', $._expression)),
      ';',
      optional(field('update', choice($.assignment_statement, $.inc_dec_statement, $._expression))),
    ),

    short_var_declaration: $ => prec(1, seq(
      field('left', commaSep1($.identifier)),
      ':=',
      field('right', commaSep1($._expression)),
    )),

    assignment_statement: $ => prec.left(seq(
      field('left', commaSep1($._lhs_expression)),
      field('operator', $._assignment_operator),
      field('right', commaSep1($._expression)),
    )),

    _assignment_operator: $ => choice(
      '=',
      '+=',
      '-=',
      '*=',
      '/=',
      '%=',
      '&=',
      '|=',
      '^=',
      '<<=',
      '>>=',
      '&^=',
    ),

    _lhs_expression: $ => choice(
      $.identifier,
      $.selector_expression,
      $.index_expression,
    ),

    _expression: $ => choice(
      $.unary_expression,
      $.binary_expression,
      $._primary_expression,
    ),

    _primary_expression: $ => choice(
      $.identifier,
      $.parenthesized_expression,
      $.call_expression,
      $.selector_expression,
      $.index_expression,
      $.number,
      $.rune_literal,
      $.string,
      $.true_literal,
      $.false_literal,
      $.nil_literal,
    ),

    parenthesized_expression: $ => seq(
      '(',
      $._expression,
      ')',
    ),

    call_expression: $ => prec(6, seq(
      field('function', $._expression),
      field('arguments', $.argument_list),
    )),

    argument_list: $ => seq(
      '(',
      optional(seq(commaSep1($._expression), optional(','))),
      ')',
    ),

    selector_expression: $ => prec(6, seq(
      field('operand', $._expression),
      '.',
      field('field', $.identifier),
    )),

    index_expression: $ => prec(6, seq(
      field('operand', $._expression),
      '[',
      field('index', $._expression),
      ']',
    )),

    unary_expression: $ => prec(7, seq(
      field('operator', choice('!', '-', '+', '^')),
      field('operand', $._expression),
    )),

    binary_expression: $ => choice(
      prec.left(1, seq(field('left', $._expression), field('operator', '||'), field('right', $._expression))),
      prec.left(2, seq(field('left', $._expression), field('operator', '&&'), field('right', $._expression))),
      prec.left(3, seq(field('left', $._expression), field('operator', choice('==', '!=', '<', '<=', '>', '>=')), field('right', $._expression))),
      prec.left(4, seq(field('left', $._expression), field('operator', choice('+', '-', '|', '^')), field('right', $._expression))),
      prec.left(5, seq(field('left', $._expression), field('operator', choice('*', '/', '%', '<<', '>>', '&', '&^')), field('right', $._expression))),
    ),

    number: $ => token(choice(
      /0[xX][0-9a-fA-F_]+/,
      /0[bB][01_]+/,
      /(?:\d[\d_]*\.?[\d_]*|\.\d[\d_]*)(?:[eEpP][+-]?\d[\d_]*)?/,
    )),

    rune_literal: $ => token(seq(
      "'",
      optional(choice(/\\./, /[^\\'\n]/)),
      "'",
    )),

    string: $ => token(seq(
      '"',
      repeat(choice(/\\./, /[^"\\\n]/)),
      '"',
    )),

    true_literal: $ => 'true',
    false_literal: $ => 'false',
    nil_literal: $ => 'nil',

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    comment: $ => choice(
      $.line_comment,
      $.block_comment,
    ),

    line_comment: $ => token(seq('//', /[^\n]*/)),

    block_comment: $ => token(seq(
      '/*',
      /[^*]*\*+([^/*][^*]*\*+)*/,
      '/',
    )),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}
