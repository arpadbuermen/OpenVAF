# mir_opt

MIR optimization passes for the OpenVAF compiler.

## Overview

The `mir_opt` crate implements optimization passes that transform MIR (Mid-Level Intermediate Representation) to produce more efficient code. These passes run after HIR lowering and before LLVM code generation.

## Available Passes

### Sparse Conditional Constant Propagation (SCCP)

Combines constant propagation with unreachable code elimination:

```rust
pub fn sparse_conditional_constant_propagation(
    func: &mut Function,
    cfg: &ControlFlowGraph,
);
```

Features:
- Propagates constants through the CFG
- Eliminates branches with known conditions
- Removes unreachable code paths

### Dead Code Elimination (DCE)

Removes instructions whose results are unused:

```rust
pub fn dead_code_elimination(func: &mut Function);
```

Two variants:
- **Standard DCE** - Conservative, preserves side effects
- **Aggressive DCE** - More aggressive, requires explicit liveness marking

```rust
pub fn aggressive_dead_code_elimination(func: &mut Function);
```

### Global Value Numbering (GVN)

Eliminates redundant computations:

```rust
pub fn gvn(func: &mut Function) -> GVN;

pub struct GVN {
    classes: HashMap<Value, ClassId>,
}
```

Features:
- Identifies equivalent expressions
- Replaces redundant computations with existing values
- Handles commutative operations

### Instruction Combining

Simplifies instruction sequences:

```rust
pub fn inst_combine(func: &mut Function);
```

Optimizations include:
- Algebraic simplifications (`x + 0 → x`, `x * 1 → x`)
- Strength reduction (`x * 2 → x + x`)
- Constant folding
- Identity elimination

### CFG Simplification

Simplifies control flow graph structure:

```rust
pub fn simplify_cfg(func: &mut Function, cfg: &mut ControlFlowGraph);
pub fn simplify_cfg_init(func: &mut Function, cfg: &mut ControlFlowGraph);
pub fn simplify_cfg_no_phi_merge(func: &mut Function, cfg: &mut ControlFlowGraph);
```

Operations:
- Merges basic blocks
- Eliminates empty blocks
- Simplifies branch chains
- Removes unreachable blocks

### Taint Propagation

Tracks value dependencies for optimization:

```rust
pub fn propagate_taint(func: &Function, tainted: &BitSet<Value>) -> BitSet<Value>;
pub fn propagate_direct_taint(func: &Function, tainted: &BitSet<Value>) -> BitSet<Value>;
```

Used to:
- Identify values depending on specific inputs
- Split computations between setup and evaluation phases
- Enable partial evaluation

## Pass Ordering

Recommended optimization pipeline:

1. `simplify_cfg_init` - Initial CFG cleanup
2. `sparse_conditional_constant_propagation` - Constant propagation
3. `simplify_cfg` - Post-SCCP cleanup
4. `inst_combine` - Instruction simplification
5. `gvn` - Common subexpression elimination
6. `dead_code_elimination` - Remove unused code
7. `simplify_cfg` - Final CFG cleanup

## Usage Example

```rust
use mir_opt::*;

fn optimize(func: &mut Function, cfg: &mut ControlFlowGraph) {
    simplify_cfg_init(func, cfg);
    sparse_conditional_constant_propagation(func, cfg);
    simplify_cfg(func, cfg);
    inst_combine(func);
    let _gvn = gvn(func);
    dead_code_elimination(func);
    simplify_cfg(func, cfg);
}
```

## Dependencies

- `mir` - MIR representation being optimized
- `bitset` - Efficient bit sets for liveness analysis
- `workqueue` - Work list for iterative algorithms
