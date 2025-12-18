# preprocessor

Verilog-A preprocessor for the OpenVAF compiler.

## Overview

The `preprocessor` crate handles Verilog-A preprocessing, including macro expansion, conditional compilation, and file inclusion. It transforms raw source text into a token stream for the parser.

## Features

- **Macro definitions** - `` `define `` with parameters
- **Conditional compilation** - `` `ifdef ``, `` `ifndef ``, `` `else ``, `` `endif ``
- **File inclusion** - `` `include `` directive
- **Source mapping** - Tracks original source locations

## Supported Directives

| Directive | Description |
|-----------|-------------|
| `` `define `` | Define a macro |
| `` `undef `` | Undefine a macro |
| `` `ifdef `` | Conditional if defined |
| `` `ifndef `` | Conditional if not defined |
| `` `else `` | Else branch |
| `` `elsif `` | Else-if branch |
| `` `endif `` | End conditional |
| `` `include `` | Include file |
| `` `resetall `` | Reset all directives |

## API

### Main Entry Point

```rust
pub fn preprocess(sources: &dyn SourceProvider, file: FileId) -> Preprocess;
```

### Result Structure

```rust
pub struct Preprocess {
    pub ts: Arc<Vec<Token>>,           // Token stream
    pub sm: Arc<SourceMap>,            // Source mapping
    pub diagnostics: Arc<Diagnostics>, // Preprocessing errors
}
```

### Token Structure

```rust
pub struct Token {
    pub span: CtxSpan,              // Location in source context
    pub kind: SyntaxKind,           // Token type
}
```

### Source Provider Trait

```rust
pub trait SourceProvider {
    fn include_dirs(&self, root_file: FileId) -> Arc<[VfsPath]>;
    fn macro_flags(&self, file_root: FileId) -> Arc<[Arc<str>]>;
    fn file_text(&self, file: FileId) -> Result<Arc<str>, FileReadError>;
    fn file_path(&self, file: FileId) -> VfsPath;
    fn file_id(&self, path: VfsPath) -> FileId;
}
```

## Source Mapping

The preprocessor maintains a source map that tracks:

- Original file and position for each token
- Macro expansion contexts
- Include file boundaries

This enables accurate error messages pointing to original source locations.

```rust
pub struct SourceMap {
    // Maps expanded positions back to original sources
}

pub struct CtxSpan {
    pub range: TextRange,
    pub ctx: SourceContext,
}
```

## Macro Handling

### Simple Macros

```verilog
`define PI 3.14159
`define TWO_PI (2.0 * `PI)
```

### Parameterized Macros

```verilog
`define MAX(a, b) ((a) > (b) ? (a) : (b))
`define CLAMP(x, lo, hi) `MAX(`MIN(x, hi), lo)
```

## Diagnostics

Preprocessing errors include:

- `FileNotFound` - Include file not found
- `InvalidTextFormat` - Encoding issues
- `UndefinedMacro` - Reference to undefined macro
- `MacroRecursion` - Circular macro expansion

## Include Search

Files are searched in order:

1. Directory of the including file
2. Directories from `-I` command-line options
3. Standard library paths

## Dependencies

- `vfs` - Virtual file system for file access
- `lexer` - Token generation
- `tokens` - Token type definitions
