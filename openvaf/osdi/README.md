# osdi

OSDI (Open Simulator Device Interface) code generation for the OpenVAF compiler.

## Overview

The `osdi` crate generates OSDI 0.4 compatible dynamic libraries from compiled Verilog-A modules. OSDI is a standardized interface that allows circuit simulators to load and use device models without simulator-specific modifications.

## OSDI 0.4 Features

The generated libraries include:

- **Module descriptors** - Metadata about model structure
- **Parameter definitions** - Model and instance parameters with attributes
- **Node/terminal information** - Port and internal node descriptions
- **Jacobian structure** - Sparse matrix entry information
- **Evaluation functions** - Model evaluation and setup callbacks
- **Nature/discipline data** - Physical quantity definitions

## Exported Symbols

Generated libraries export these symbols:

| Symbol | Type | Description |
|--------|------|-------------|
| `OSDI_DESCRIPTORS` | Array | Module descriptor array |
| `OSDI_NUM_DESCRIPTORS` | u32 | Number of descriptors |
| `OSDI_VERSION_MAJOR` | u32 | Major version (0) |
| `OSDI_VERSION_MINOR` | u32 | Minor version (4) |
| `OSDI_DESCRIPTOR_SIZE` | u32 | Descriptor struct size |
| `OSDI_NATURES` | Array | Nature definitions (optional) |
| `OSDI_DISCIPLINES` | Array | Discipline definitions (optional) |
| `osdi_log` | Pointer | Logging callback (simulator-set) |

## Module Descriptor

Each module has a descriptor containing:

```c
struct OsdiDescriptor {
    const char* name;
    uint32_t num_nodes;
    uint32_t num_terminals;
    OsdiNode* nodes;
    uint32_t num_params;
    OsdiParam* params;
    // ... evaluation functions
    void (*setup_model)(void* model, void* sim_params);
    void (*setup_instance)(void* inst, void* model, double temp);
    void (*eval)(void* inst, void* model, double* voltages, ...);
    // ... jacobian info
};
```

## Code Generation

The crate generates separate object files for:

1. **Access functions** - Node voltage/current access
2. **Model setup** - Model parameter initialization
3. **Instance setup** - Instance-specific initialization
4. **Evaluation** - Core model evaluation with Jacobians

These are compiled in parallel using Rayon for performance.

## Usage

```rust
use osdi::compile;

let (object_paths, compiled_modules, literals) = osdi::compile(
    &db,
    &modules,
    &output_path,
    &target,
    &backend,
    emit,           // Generate object files
    opt_lvl,        // LLVM optimization level
    dump_mir,       // Debug: dump MIR
    dump_unopt_mir, // Debug: dump unoptimized MIR
    dump_ir,        // Debug: dump LLVM IR
    dump_unopt_ir,  // Debug: dump unoptimized LLVM IR
);
```

## Jacobian Computation

The generated code computes sparse Jacobian entries:

- Only non-zero partial derivatives are computed
- Automatic differentiation generates derivative code
- Supports both DC and transient (ddt) contributions
- Offset-based loading for harmonic balance

## Callback Functions

The generated libraries support callbacks for:

- **Limiting functions** - `$limit` and builtin limiters
- **Logging** - `$strobe`, `$display`, `$debug`
- **Parameter queries** - `$param_given`, `$port_connected`
- **Simulation info** - `$simparam`, `$temperature`

## Header Files

The OSDI C header files are in `header/`:

- `osdi_0_3.h` - OSDI 0.3 API (backward compatible)
- `osdi_0_4.h` - OSDI 0.4 API

## Dependencies

- `hir` - HIR for module information
- `sim_back` - Compiled module representation
- `mir_llvm` - LLVM code generation
- `llvm-sys` - LLVM bindings
- `target` - Target specifications
- `rayon-core` - Parallel compilation
