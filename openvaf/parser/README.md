# parser

Verilog-A parser for the OpenVAF compiler.

## Overview

The `parser` crate implements a hand-written recursive descent parser for Verilog-A. It operates on a token stream from the preprocessor and produces parse events that are consumed by a tree builder.

## Design

The parser uses an abstract interface that decouples parsing logic from tree construction:

- **TokenSource** - Provides tokens to the parser
- **Output** - Receives parse events (start node, add token, finish node, error)

This allows the same parser to work with different tree representations.

## Grammar Coverage

The parser handles the full Verilog-A grammar including:

### Module Structure
- Module declarations
- Port declarations (input, output, inout)
- Parameter declarations
- Variable declarations (real, integer, string)

### Analog Blocks
- `analog` blocks
- Branch contributions (`<+`)
- Conditional statements (`if`, `case`)
- Loop statements (`for`, `while`, `repeat`)
- Event control (`@`)

### Expressions
- Arithmetic, logical, relational operators
- System functions (`$temperature`, `$vt`, etc.)
- Analog operators (`ddt`, `idt`, `ddx`, etc.)
- Array subscripts
- Function calls

### Types and Declarations
- Nature and discipline definitions
- Branch declarations
- Alias parameters
- Attributes (`(* attr = value *)`)

## API

### Main Entry Point

```rust
pub fn parse(tokens: &[SyntaxKind]) -> Output;
```

### Output Events

```rust
pub enum Step {
    Token { kind: SyntaxKind },
    Enter { kind: SyntaxKind },
    Exit,
    Error { error: SyntaxError },
}

pub struct Output {
    pub steps: Vec<Step>,
}
```

### Error Handling

The parser produces detailed syntax errors:

```rust
pub struct Error {
    pub expected: pretty::List<Vec<Token>>,
    pub found: Token,
}
```

## Token Sets

The parser uses token sets for efficient lookahead:

```rust
macro_rules! T {
    [;] => { SyntaxKind::SEMICOLON };
    [module] => { SyntaxKind::MODULE_KW };
    // ...
}
```

## Recovery

The parser includes error recovery to continue parsing after errors:

- Synchronizes at statement boundaries
- Handles missing semicolons gracefully
- Reports multiple errors per compilation

## Dependencies

- `tokens` - Token definitions shared with lexer
- `stdx` - Standard library extensions
- `drop_bomb` - Debug assertions for parser state
