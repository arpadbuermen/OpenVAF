# tokens

Token definitions for the OpenVAF Verilog-A compiler.

## Overview

The `tokens` crate defines the token kinds used throughout the OpenVAF compiler. It provides the shared vocabulary for the lexer, preprocessor, parser, and syntax tree.

## Token Categories

### Lexer Tokens

Low-level tokens from the lexer:

```rust
pub mod lexer {
    pub enum TokenKind {
        // Whitespace and comments
        Whitespace,
        LineComment,
        BlockComment { terminated: bool },

        // Identifiers
        SimpleIdent,
        EscapedIdent,
        SystemCallIdent,

        // Literals
        Literal { kind: LiteralKind },

        // Operators
        Plus, Minus, Star, Slash, Percent,
        Eq, Lt, Gt, Not,
        // ...

        // Delimiters
        OpenParen, CloseParen,
        OpenBrace, CloseBrace,
        OpenBracket, CloseBracket,
        // ...

        // Preprocessor
        CompilerDirective,
        Define { has_args: bool },
    }

    pub enum LiteralKind {
        Int,
        Float { has_scale_char: bool },
        Str { terminated: bool },
    }
}
```

### Parser/Syntax Tokens

Higher-level tokens for parsing and syntax trees:

```rust
pub mod parser {
    pub enum SyntaxKind {
        // Special
        ERROR,
        WHITESPACE,
        COMMENT,
        EOF,

        // Identifiers and literals
        IDENT,
        INT_NUMBER,
        STD_REAL_NUMBER,
        SI_REAL_NUMBER,
        STR_LIT,
        SYSFUN,

        // Keywords
        MODULE_KW,
        ENDMODULE_KW,
        INPUT_KW,
        OUTPUT_KW,
        INOUT_KW,
        PARAMETER_KW,
        ANALOG_KW,
        // ... many more

        // Compound nodes
        SOURCE_FILE,
        MODULE,
        PORT_DECL,
        PARAM_DECL,
        // ...
    }
}
```

## Token Macro

The `T!` macro provides convenient syntax for token matching:

```rust
macro_rules! T {
    [;] => { SyntaxKind::SEMICOLON };
    [,] => { SyntaxKind::COMMA };
    [module] => { SyntaxKind::MODULE_KW };
    [endmodule] => { SyntaxKind::ENDMODULE_KW };
    [input] => { SyntaxKind::INPUT_KW };
    // ...
}
```

Usage:
```rust
match token.kind {
    T![module] => parse_module(),
    T![;] => consume_semicolon(),
    _ => error(),
}
```

## Lexer to Parser Conversion

The `TokenKind::to_syntax` method converts lexer tokens to parser tokens:

```rust
impl TokenKind {
    pub fn to_syntax(self, src: &str) -> (Option<SyntaxKind>, Option<LexerErrorKind>);
}
```

This handles:
- Keyword recognition from identifiers
- Error reporting for unterminated strings/comments
- Filtering preprocessor directives

## Keywords

The crate defines all Verilog-A keywords:

```rust
impl SyntaxKind {
    pub fn from_keyword(s: &str) -> Option<SyntaxKind>;
}
```

Keywords include:
- Module structure: `module`, `endmodule`, `macromodule`
- Ports: `input`, `output`, `inout`
- Types: `real`, `integer`, `string`, `parameter`
- Control: `if`, `else`, `case`, `for`, `while`
- Analog: `analog`, `branch`, `nature`, `discipline`

## SI Scale Factors

Real number scale factors recognized:

| Suffix | Value |
|--------|-------|
| T | 10^12 |
| G | 10^9 |
| M | 10^6 |
| K, k | 10^3 |
| m | 10^-3 |
| u | 10^-6 |
| n | 10^-9 |
| p | 10^-12 |
| f | 10^-15 |
| a | 10^-18 |

## Dependencies

- `text-size` - Text position types
