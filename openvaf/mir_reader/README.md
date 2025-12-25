# mir_reader

MIR text format parser for the OpenVAF compiler.

## Overview

The `mir_reader` crate provides a parser for MIR (Mid-Level Intermediate Representation) in text format. This is primarily used for testing - test cases can be written as text and parsed into MIR for verification.

## Features

- **Human-readable format** - MIR text format is easy to read and write
- **Bidirectional** - Works with MIR printing for round-trip testing
- **Error reporting** - Clear error messages for malformed input

## Text Format

### Function Definition

```
fn @function_name(v0: real, v1: int) -> real {
    block0:
        v2 = fadd v0, v0
        v3 = fmul v2, v1
        exit
}
```

### Instructions

```
; Arithmetic
v2 = fadd v0, v1
v3 = fsub v0, v1
v4 = fmul v0, v1
v5 = fdiv v0, v1

; Comparisons
v6 = flt v0, v1
v7 = feq v0, v1

; Control flow
br v6, block1, block2
jmp block3
exit

; Phi nodes
v8 = phi [block0: v0, block1: v1]

; Constants
v9 = const 3.14159
v10 = iconst 42
```

### Types

- `real` - 64-bit floating point
- `int` - 32-bit integer
- `bool` - Boolean
- `str` - String (interned)

## API

### Parsing Functions

```rust
pub fn parse_function(src: &str, literals: &mut Rodeo) -> ParseResult<Function>;

pub fn parse_functions(src: &str, literals: &mut Rodeo) -> ParseResult<Vec<Function>>;
```

### Error Types

```rust
pub enum ParseError {
    LexError(LexError),
    UnexpectedToken { expected: &'static str, found: Token },
    UndefinedValue(String),
    UndefinedBlock(String),
    // ...
}

pub type ParseResult<T> = Result<T, ParseError>;
```

## Usage

```rust
use mir_reader::parse_function;
use lasso::Rodeo;

let src = r#"
fn @test(v0: real) -> real {
    block0:
        v1 = fmul v0, v0
        exit
}
"#;

let mut literals = Rodeo::new();
let func = parse_function(src, &mut literals)?;
```

## Testing Workflow

1. Write expected MIR as text
2. Parse into `Function`
3. Run optimization/transformation
4. Print result and compare with expected output

```rust
use mir_reader::parse_function;
use expect_test::expect;

let func = parse_function(input, &mut literals)?;
optimize(&mut func);

let expected = expect![[r#"
    fn @test() {
        block0:
            exit
    }
"#]];
expected.assert_eq(&func.print(&literals));
```

## Dependencies

- `mir` - Target MIR representation
- `bforest` - B-tree forest for phi edges
- `lasso` - String interner for literals
