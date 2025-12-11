# hir_lower

HIR to MIR lowering for the OpenVAF Verilog-A compiler.

## Overview

The `hir_lower` crate transforms the High-Level Intermediate Representation (HIR) into the Mid-Level Intermediate Representation (MIR). This is a critical compilation phase that:

- Converts high-level Verilog-A constructs into lower-level operations
- Resolves types and generates type-appropriate operations
- Handles control flow lowering
- Manages the parameter/output interning for simulation

## Key Components

### HirInterner

The central structure for tracking HIR to MIR mappings:

```rust
pub struct HirInterner {
    pub params: IndexMap<ParamKind, Value>,
    pub outputs: IndexMap<PlaceKind, Option<Value>>,
    pub callbacks: IndexSet<CallBackKind>,
    pub tagged_reads: IndexMap<Value, VarId>,
    pub implicit_equations: Vec<ImplicitEquationKind>,
}
```

### ParamKind

Represents different kinds of simulation parameters:

- `Param(ParamId)` - Model/instance parameters
- `Voltage { hi, lo }` - Node voltage access
- `Current(CurrentKind)` - Branch current access
- `Temperature` - Simulation temperature
- `ParamGiven { param }` - Parameter given flags
- `PortConnected { port }` - Port connection status
- `HiddenState(VarId)` - Hidden state variables
- `Abstime` - Absolute simulation time

### CurrentKind

Specifies how branch currents are accessed:

```rust
pub enum CurrentKind {
    Branch(BranchId),           // Named branch current
    Unnamed { hi: NodeId, lo: Option<NodeId> },  // Unnamed branch
    Port(NodeId),               // Port current
}
```

### PlaceKind

Identifies output destinations:

- `Var(VarId)` - Variable outputs
- `Contribute { ... }` - Branch contributions
- `ImplicitEquationResidual(...)` - Implicit equation residuals

### CallBackKind

External function callbacks needed during simulation:

- `BuiltinLimit` - Limiting functions
- `StoreLimit` - State storage with limiting
- `Derivative` - Derivative computation
- `NodeDerivative` - Node voltage derivatives
- `Print/Display/Debug` - Output functions

### ImplicitEquation

Represents implicit equations for iterative solving:

```rust
pub struct ImplicitEquation(u32);

pub enum ImplicitEquationKind {
    Ddt(VarId),
    Idt { ... },
    LinearDdt(VarId),
    LinearIdt { ... },
}
```

## Lowering Process

1. **Expression Lowering** - Convert HIR expressions to MIR values
2. **Statement Lowering** - Convert HIR statements to MIR instructions
3. **Control Flow** - Lower conditionals and loops to branches
4. **Contribution Handling** - Process branch contribution statements
5. **Callback Resolution** - Identify required simulation callbacks

## Dependencies

- `hir_def` - HIR definitions being lowered
- `hir_ty` - Type information for operations
- `mir` - Target MIR representation
- `mir_build` - MIR construction utilities
