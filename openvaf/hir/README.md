# hir

High-Level Intermediate Representation (HIR) for the OpenVAF Verilog-A compiler.

## Overview

The `hir` crate provides the public API for semantic analysis of Verilog-A code. It defines the `CompilationDB` trait which extends `BaseDB` with HIR-level queries, serving as the central database for the compiler's semantic analysis phase.

## Key Components

### CompilationDB

The main database trait that provides access to:

- Module definitions and their contents
- Type information for expressions
- Parameter declarations and their properties
- Node and branch definitions
- Diagnostic collection

### CompilationUnit

Represents a complete Verilog-A compilation unit (source file with its includes):

```rust
pub trait CompilationUnit {
    fn root_file(&self) -> FileId;
    fn preprocess(&self, db: &dyn BaseDB) -> Preprocess;
    fn parse(&self, db: &dyn BaseDB) -> Parse<SourceFile>;
}
```

### HIR Types

The crate exposes various HIR item types:

- **Module** - Verilog-A module definition
- **Variable** - Variable declarations
- **Parameter** - Model and instance parameters
- **Node** - Electrical nodes (ports and internal)
- **Branch** - Named branches between nodes
- **Nature/Discipline** - Analog nature and discipline definitions
- **Function** - User-defined analog functions

### CompilationOpts

Configuration options for compilation:

```rust
pub struct CompilationOpts {
    pub allow_lints: Vec<String>,
    pub warn_lints: Vec<String>,
    pub deny_lints: Vec<String>,
}
```

## Architecture

This crate integrates several sub-crates:

- `hir_def` - HIR definitions and name resolution
- `hir_ty` - Type inference and checking
- `hir_lower` - Lowering from HIR to MIR

## Usage

```rust
use hir::CompilationDB;

// Create a compilation database from filesystem
let db = CompilationDB::new_fs(
    input_path,
    &include_dirs,
    &defines,
    &lints,
    &compilation_opts,
)?;

// Access compilation unit
let cu = db.compilation_unit();

// Get modules defined in the compilation unit
let modules = cu.modules(&db);
```

## Dependencies

- `basedb` - Base database infrastructure
- `hir_def` - HIR definition structures
- `hir_ty` - Type system
- `syntax` - Syntax tree types
- `vfs` - Virtual file system
- `salsa` - Incremental computation
