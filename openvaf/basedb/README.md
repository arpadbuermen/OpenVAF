# basedb

Core database infrastructure for the OpenVAF Verilog-A compiler, providing the foundation for incremental compilation via the [salsa](https://github.com/salsa-rs/salsa) framework.

## Overview

`basedb` defines the `BaseDB` trait, which is the foundational query group for OpenVAF's compilation database. It provides essential services including:

- **File management** via integration with the virtual file system (`vfs`)
- **Parsing and preprocessing** of Verilog-A source files
- **Line index computation** for mapping byte offsets to line/column positions
- **Lint registry and configuration** with attribute-based overrides
- **AST ID mapping** for stable references to syntax nodes
- **Diagnostic infrastructure** for error and warning reporting

## Key Components

### BaseDB Trait

The central trait that downstream crates extend for semantic analysis:

```rust
pub trait BaseDB: VfsStorage + salsa::Database {
    fn parse(&self, root_file: FileId) -> Parse<SourceFile>;
    fn preprocess(&self, root_file: FileId) -> Preprocess;
    fn file_text(&self, file: FileId) -> Result<Arc<str>, FileReadError>;
    fn line_index(&self, file_id: FileId) -> Arc<LineIndex>;
    fn lint_registry(&self) -> Arc<LintRegistry>;
    fn ast_id_map(&self, root_file: FileId) -> Arc<AstIdMap>;
    // ... and more
}
```

### Modules

- **`ast_id_map`** - Provides stable IDs (`AstId<N>`) for syntax nodes that survive incremental recompilation. Uses breadth-first traversal to ensure parent IDs are always lower than child IDs.

- **`diagnostics`** - Diagnostic reporting infrastructure built on `codespan-reporting`. Defines the `Diagnostic` trait for building error reports with source locations and lint integration.

- **`line_index`** - Maps byte offsets (`TextSize`) to line/column positions. Handles UTF-16 encoding for LSP compatibility.

- **`lints`** - Configurable lint system with three severity levels (`Allow`, `Warn`, `Deny`). Includes builtin lints and supports plugin lints.

- **`lint_attrs`** - Parses lint attribute annotations from source code to override lint levels on specific items.

## Builtin Lints

| Lint | Default | Description |
|------|---------|-------------|
| `macro_overwritten` | Warn | Macro redefinition |
| `lint_not_found` | Deny | Unknown lint name in attribute |
| `lint_level_overwrite` | Warn | Lint level change via attribute |
| `non_standard_code` | Warn | Non-standard Verilog-A constructs |
| `vams_keyword_compat` | Warn | VAMS keyword compatibility |
| `non_standard_analog_operator` | Deny | Non-standard analog operators |
| `const_simparam` | Allow | Constant simparam usage |
| `variant_const_simparam` | Warn | Variant constant simparam |
| `port_without_direction` | Deny | Port missing direction |
| `trivial_probe` | Warn | Trivial probe usage |

## Standard Flags

The following preprocessor flags are defined by default:

- `__OPENVAF__`
- `__VAMS__`
- `__VAMS_COMPACT_MODELING__`

## Dependencies

- `salsa` - Incremental computation framework
- `vfs` - Virtual file system abstraction
- `syntax` - Verilog-A syntax tree and source mapping
- `arena` - Arena allocator for AST nodes
- `codespan-reporting` - Diagnostic rendering

## Usage

This crate is not intended for direct use. It serves as the foundation for `hir` and other higher-level compiler crates that build upon `BaseDB` to provide semantic analysis.
