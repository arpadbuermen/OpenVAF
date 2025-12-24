# openvaf

Main compilation library for the OpenVAF Verilog-A compiler.

## Overview

The `openvaf` crate is the top-level compilation library that orchestrates the entire compilation pipeline from Verilog-A source files to OSDI-compatible dynamic libraries. It provides the public API for invoking the compiler.

## Features

- **Full compilation pipeline** - From source to native code
- **Caching support** - Content-addressed caching of compiled models
- **Multiple output modes** - Direct compilation or preprocessor expansion
- **Diagnostic reporting** - Rich error messages with source locations
- **Cross-compilation** - Support for multiple target platforms

## Compilation API

### Main Entry Points

```rust
pub fn compile(opts: &Opts) -> Result<CompilationTermination>;
pub fn expand(opts: &Opts) -> Result<CompilationTermination>;
```

### Compilation Options

```rust
pub struct Opts {
    pub dry_run: bool,
    pub defines: Vec<String>,
    pub codegen_opts: Vec<String>,
    pub lints: Vec<(String, LintLevel)>,
    pub input: Utf8PathBuf,
    pub output: CompilationDestination,
    pub include: Vec<AbsPathBuf>,
    pub opt_lvl: LLVMCodeGenOptLevel,
    pub target: Target,
    pub target_cpu: String,
    pub dump_mir: bool,
    pub dump_unopt_mir: bool,
    pub dump_ir: bool,
    pub dump_unopt_ir: bool,
    pub compilation_opts: CompilationOpts,
}
```

### Output Destinations

```rust
pub enum CompilationDestination {
    Path { lib_file: Utf8PathBuf },
    Cache { cache_dir: Utf8PathBuf },
}
```

### Compilation Result

```rust
pub enum CompilationTermination {
    Compiled { lib_file: Utf8PathBuf },
    FatalDiagnostic,
}
```

## Compilation Pipeline

1. **Preprocessing** - Macro expansion and include resolution
2. **Parsing** - Build syntax tree from preprocessed tokens
3. **HIR construction** - Build semantic representation
4. **Type checking** - Verify types and resolve overloads
5. **MIR lowering** - Convert to mid-level IR
6. **Optimization** - Run MIR optimization passes
7. **Autodiff** - Generate Jacobian computation code
8. **LLVM codegen** - Generate native code via LLVM
9. **Linking** - Link into OSDI dynamic library

## Caching

The compiler supports content-addressed caching:

```rust
// Cache compiled model based on content hash
CompilationDestination::Cache { cache_dir: "/path/to/cache".into() }
```

Cache keys include:
- Source file content hash
- Compiler options
- Target configuration

## Debug Output

Various debug dumps are available:

| Option | Description |
|--------|-------------|
| `dump_unopt_mir` | Dump MIR before optimization |
| `dump_mir` | Dump optimized MIR |
| `dump_unopt_ir` | Dump LLVM IR before optimization |
| `dump_ir` | Dump optimized LLVM IR |

## Re-exports

The crate re-exports commonly needed types:

```rust
pub use basedb::lints::{builtin as builtin_lints, LintLevel};
pub use hir::CompilationOpts;
pub use llvm_sys::target_machine::LLVMCodeGenOptLevel;
pub use paths::AbsPathBuf;
pub use target::host_triple;
pub use target::spec::{get_target_names, Target};
```

## Dependencies

- `basedb` - Core database infrastructure
- `hir` - High-level IR and semantic analysis
- `sim_back` - Simulation backend transformations
- `osdi` - OSDI code generation
- `mir_llvm` - LLVM code generation
- `linker` - Platform linking
- `target` - Target specifications
