# Codebase Overview

This guide helps developers navigate the OpenVAF codebase and understand how the different components fit together.

## Project Structure

```
OpenVAF/
├── openvaf/          # Main compiler crates
├── lib/              # Shared utility libraries
├── verilogae/        # Python bindings for parameter extraction
├── melange/          # Experimental circuit simulator (early development)
├── sourcegen/        # Code generation utilities
├── xtask/            # Build automation tasks
├── integration_tests/# Real-world model tests
└── docs/             # This documentation
```

## Compilation Pipeline

The OpenVAF compiler follows a multi-stage pipeline similar to rustc:

```
Source → Lexer → Parser → HIR → MIR → LLVM IR → Native Code
           ↓        ↓       ↓      ↓
        tokens   syntax   typed  optimized
                  tree     IR      IR
```

### Stage 1: Lexing & Parsing

| Crate | Purpose |
|-------|---------|
| `lexer` | Tokenizes Verilog-A source into tokens |
| `tokens` | Token type definitions |
| `parser` | Builds concrete syntax tree using `rowan` |
| `syntax` | CST node types and source mapping |
| `preprocessor` | Handles \`include and macro expansion |

### Stage 2: HIR (High-Level IR)

| Crate | Purpose |
|-------|---------|
| `hir_def` | HIR definitions and name resolution |
| `hir_ty` | Type inference and checking |
| `hir` | HIR query database (uses `salsa`) |
| `hir_lower` | Lowers HIR to MIR |

### Stage 3: MIR (Mid-Level IR)

| Crate | Purpose |
|-------|---------|
| `mir` | Core MIR data structures (SSA form) |
| `mir_build` | Constructs MIR from HIR |
| `mir_opt` | MIR-level optimizations |
| `mir_autodiff` | Automatic differentiation for Jacobians |
| `mir_reader` | MIR parsing for tests/debugging |
| `mir_interpret` | MIR interpreter for constant evaluation |

### Stage 4: Code Generation

| Crate | Purpose |
|-------|---------|
| `mir_llvm` | LLVM IR generation from MIR |
| `linker` | Links generated code into `.osdi` libraries |
| `osdi` | OSDI API structures and code generation |
| `sim_back` | Simulation-specific transformations |
| `target` | Platform-specific codegen configurations |

### Support Crates

| Crate | Purpose |
|-------|---------|
| `basedb` | Base database with file system and VFS |
| `vfs` | Virtual file system for source management |
| `openvaf` | Top-level compilation API |
| `openvaf-driver` | CLI binary entry point |

## Utility Libraries (`lib/`)

| Crate | Purpose |
|-------|---------|
| `arena` | Arena allocator for HIR/MIR nodes |
| `bforest` | B-tree forest data structure |
| `bitset` | Bit set implementation |
| `list_pool` | Pool allocator for linked lists |
| `typed_indexmap` | Type-safe indexed collections |
| `stdx` | Standard library extensions |
| `paths` | Path handling utilities |
| `mini_harness` | Custom test runner |
| `base_n` | Base-N encoding utilities |
| `workqueue` | Work queue for parallel processing |

## Key Dependencies

- **salsa** - Incremental compilation framework (custom fork)
- **rowan** - Concrete syntax tree library
- **llvm-sys** - LLVM C API bindings (supports LLVM 18-21)
- **camino** - UTF-8 path handling

## Other Components

### VerilogAE (`verilogae/`)

Python bindings for OpenVAF, primarily used for parameter extraction in model development workflows.

| Directory | Purpose |
|-----------|---------|
| `verilogae/` | Core Rust library |
| `verilogae_ffi/` | C/C++ FFI layer (cbindgen) |
| `verilogae_py/` | PyO3-based Python bindings |

Build with: `cargo xtask verilogae build`

### Melange (`melange/`)

> **Status: Experimental / Early Development**

Melange is an experimental circuit simulator that leverages OpenVAF for compact model support. It focuses on providing an ergonomic API in Python and Rust instead of traditional netlist formats.

**Current limitations:**
- Python API not complete
- Most features still in development
- Tests are disabled (`#[cfg(all(test, not(windows)))]` commented out)

### Sourcegen (`sourcegen/`)

Code generation utilities for generating Rust code from OSDI header files. Used to keep the OSDI bindings in sync with the header definitions.

### Xtask (`xtask/`)

Build automation using the [xtask pattern](https://github.com/matklad/cargo-xtask). Available commands:

```bash
# VerilogAE Python package
cargo xtask verilogae build     # Build Python wheel
cargo xtask verilogae test      # Run tests
cargo xtask verilogae publish   # Publish to PyPI

# MSVC runtime generation (Windows)
cargo xtask gen-msvcrt
```

## Incomplete or Experimental Features

Based on code analysis, the following areas have known limitations or are under development:

### Known TODOs

- **Arrays** - Limited array support in some contexts
- **Hidden state** - Not fully implemented
- **Complex noise power** - Partial implementation
- **Call derivatives** - Incomplete in autodiff
- **Endianness** - Currently assumes little-endian

### Disabled/Commented Code

- `melange/core/src/lib.rs` - Tests commented out
- `xtask/src/main.rs` - `mod vendor` and `mod cache` commented out
- Various `#[allow(dead_code)]` annotations in `osdi/src/metadata.rs`

## Database Architecture

OpenVAF uses `salsa` for incremental compilation with two main database traits:

1. **BaseDB** (`basedb`) - File system, VFS, parsing, linting
2. **CompilationDB** (`hir`) - Extends BaseDB with semantic analysis

Queries are memoized and automatically invalidated when inputs change.

## File Naming Conventions

- `*.va` - Verilog-A source files
- `*.osdi` - Compiled OSDI dynamic libraries
- `*.snap` - Test snapshot files
- `*.body`, `*.item_tree`, `*.def_map` - Test artifacts

## Getting Started with Development

1. **Read the crate READMEs** - Each `openvaf/*` crate has a README with details
2. **Explore with `--dump-*` flags** - Use `openvaf-r --dump-mir` etc. to see intermediate representations
3. **Run tests** - Start with `cargo test` to understand the test infrastructure
4. **Use the LSP** - rust-analyzer works well with the codebase

## Debugging Tips

```bash
# Disable parallelization for easier debugging
RAYON_NUM_THREADS=1 cargo test

# Dump intermediate representations
openvaf-r --dump-unopt-mir model.va  # Unoptimized MIR
openvaf-r --dump-mir model.va        # Optimized MIR
openvaf-r --dump-unopt-ir model.va   # Unoptimized LLVM IR
openvaf-r --dump-ir model.va         # Optimized LLVM IR
```

See the VS Code debug configurations in `.vscode/launch-openvaf-r.json` for debugger setup.
