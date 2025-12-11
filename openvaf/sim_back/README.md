# sim_back

Simulation backend for the OpenVAF Verilog-A compiler.

## Overview

The `sim_back` crate transforms compiled MIR into simulator-ready form. It handles the circuit-simulation-specific transformations including DAE (Differential-Algebraic Equation) system construction, automatic differentiation, and node collapse optimization.

## Key Components

### ModuleInfo

Information about a compiled Verilog-A module:

```rust
pub struct ModuleInfo {
    pub module: Module,
    pub params: IndexMap<ParamId, ParamInfo>,
    pub op_vars: IndexMap<VarId, OpVarInfo>,
    // ...
}
```

### CompiledModule

Complete compiled representation:

```rust
pub struct CompiledModule<'a> {
    pub info: &'a ModuleInfo,
    pub dae_system: DaeSystem,
    pub eval: Function,
    pub intern: HirInterner,
    pub init: Initialization,
    pub model_param_setup: Function,
    pub model_param_intern: HirInterner,
    pub node_collapse: NodeCollapse,
}
```

### DaeSystem

The differential-algebraic equation system:

```rust
pub struct DaeSystem {
    pub unknowns: Vec<SimUnknownKind>,
    pub residuals: Vec<Residual>,
    pub jacobian_entries: Vec<JacobianEntry>,
    // ...
}
```

### SimUnknownKind

Types of simulation unknowns:

```rust
pub enum SimUnknownKind {
    KirchoffLaw(Node),          // Node voltage
    Current(CurrentKind),        // Branch current
    Implicit(ImplicitEquation),  // Implicit equation variable
}
```

## Compilation Pipeline

1. **Module collection** - Gather all modules from HIR
2. **MIR construction** - Build initial MIR from HIR
3. **Topology analysis** - Analyze circuit topology
4. **DAE construction** - Build equation system
5. **Jacobian derivation** - Compute partial derivatives via autodiff
6. **Sparsification** - Identify sparse Jacobian structure
7. **Setup/eval split** - Separate initialization from evaluation
8. **Node collapse** - Optimize node topology

## Automatic Differentiation

Uses `mir_autodiff` to generate Jacobian computation:

```rust
// Jacobian entry: ∂I_i/∂V_j
pub struct JacobianEntry {
    pub row: SimUnknownKind,    // Current/residual
    pub col: SimUnknownKind,    // Voltage/unknown
    pub value: Value,           // MIR value for derivative
}
```

## Initialization

Separates computations into:

- **Model setup** - Once per model instantiation
- **Instance setup** - Once per instance
- **Evaluation** - Each simulation step

```rust
pub struct Initialization {
    pub func: Function,
    pub intern: HirInterner,
    pub cached_vals: HashMap<Value, CacheSlot>,
    pub cache_slots: TiVec<CacheSlot, (GvnClass, Type)>,
}
```

## Node Collapse

Optimizes internal nodes that can be eliminated:

```rust
pub struct NodeCollapse {
    pub collapsible_pairs: Vec<(NodeId, NodeId)>,
    // ...
}
```

## Usage

```rust
use sim_back::{collect_modules, CompiledModule};

// Collect modules from database
let modules = collect_modules(&db, false, &mut sink)?;

// Compile each module
for module in &modules {
    let compiled = CompiledModule::new(
        &db,
        module,
        &mut literals,
        dump_unopt_mir,
        dump_mir,
    );
}
```

## Dependencies

- `hir` - High-level IR access
- `hir_lower` - HIR to MIR lowering
- `mir` - MIR representation
- `mir_autodiff` - Automatic differentiation
- `mir_opt` - MIR optimization
- `mir_build` - MIR construction
