# lexer

Verilog-A lexical analyzer for the OpenVAF compiler.

## Overview

The `lexer` crate provides a low-level tokenizer for Verilog-A source code. It converts raw source text into a stream of tokens that are consumed by the preprocessor and parser.

## Features

- **Zero-copy tokenization** - Works directly on source text without allocation
- **UTF-8 support** - Handles Unicode identifiers and strings
- **Error recovery** - Produces tokens even for malformed input
- **Preprocessor-aware** - Recognizes compiler directives for macro handling

## Token Types

The lexer recognizes all Verilog-A token types:

### Literals
- Integer literals (`123`, `8'b1010`, `32'hDEAD`)
- Real literals (`1.5`, `2.3e-4`, `1.0M` with SI scale factors)
- String literals (`"hello world"`)

### Identifiers
- Simple identifiers (`module_name`)
- Escaped identifiers (`\special.name`)
- System identifiers (`$display`, `$temperature`)

### Operators
- Arithmetic: `+`, `-`, `*`, `/`, `%`, `**`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `&&`, `||`, `!`
- Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`, `<<<`, `>>>`
- Contribution: `<+`

### Delimiters
- Parentheses, brackets, braces
- Attribute markers: `(*`, `*)`
- Array literals: `'{`

### Comments
- Line comments: `// comment`
- Block comments: `/* comment */`

### Compiler Directives
- `` `define ``, `` `ifdef ``, `` `include ``, etc.

## Usage

```rust
use lexer::Lexer;

let source = "module test; endmodule";
let lexer = Lexer::new(source);

for token in lexer {
    println!("{:?}: {:?}", token.kind, &source[token.range()]);
}
```

## SI Scale Factors

The lexer recognizes Verilog-A SI scale factors in real numbers:

| Factor | Scale |
|--------|-------|
| T | 10^12 |
| G | 10^9 |
| M | 10^6 |
| K/k | 10^3 |
| m | 10^-3 |
| u | 10^-6 |
| n | 10^-9 |
| p | 10^-12 |
| f | 10^-15 |
| a | 10^-18 |

## Dependencies

- `tokens` - Token kind definitions shared with parser
