; Kage (Ebitengine shader language) highlight queries

; --- keywords ---

[
  "package"
  "var"
  "const"
  "func"
  "if"
  "else"
  "for"
  "return"
] @keyword

[
  "&&"
  "||"
  "!"
] @keyword.operator

; --- operators ---

[
  "+"
  "-"
  "*"
  "/"
  "%"
  "^"
  "&"
  "|"
  "&^"
  "<<"
  ">>"
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "&^="
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  ":="
  "="
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  "."
  ";"
  ":"
] @punctuation.delimiter

; --- comments ---

(comment) @comment

; --- literals ---

(number) @number
(string) @string
(rune_literal) @string.special

; --- types ---

(type_identifier) @type

((identifier) @type.builtin
  (#match? @type.builtin "^(bool|int|float|vec[234]|ivec[234]|mat[234])$"))

; --- constants ---

((identifier) @constant.builtin
  (#match? @constant.builtin "^(true|false|nil)$"))

; --- functions ---

(function_declaration name: (identifier) @function)

(call_expression
  function: (identifier) @function.call)

((call_expression
  function: (identifier) @function.builtin)
  (#match? @function.builtin "^(sin|cos|tan|asin|acos|atan|atan2|pow|exp|log|exp2|log2|sqrt|inversesqrt|abs|sign|floor|ceil|fract|mod|min|max|clamp|mix|step|smoothstep|length|distance|dot|cross|normalize|faceforward|reflect|refract|transpose|dfdx|dfdy|fwidth|len|cap|discard|texelFetch|imageSrc[0-3]At|imageSrc[0-3]UnsafeAt|imageSrcTextureSize|imageDstTextureSize|imageSrcRegionOnTexture|imageDstRegionOnTexture)$"))

; --- members / swizzling ---

(selector_expression
  field: (identifier) @property)

; --- variables ---

(parameter_declaration
  name: (identifier) @variable.parameter)

(short_var_declaration
  left: (identifier) @variable)

(var_spec
  name: (identifier) @variable)

(const_spec
  name: (identifier) @variable)
