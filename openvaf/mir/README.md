# mir

Mid-Level Intermediate Representation for the OpenVAF Verilog-A compiler.

## Overview

The `mir` crate defines the Mid-Level Intermediate Representation used as the primary optimization and code generation target. MIR is a low-level, SSA-form IR that represents Verilog-A code after type checking and before LLVM IR generation.

## Design

MIR uses a graph-based SSA representation with:

- **Basic blocks** - Sequential instruction containers
- **Instructions** - Operations producing values
- **Values** - SSA values referenced by instructions
- **Control flow** - Explicit branch/jump instructions

## Key Types

### Function

The top-level container for MIR code:

```rust
pub struct Function {
    pub dfg: DataFlowGraph,     // Values and instructions
    pub layout: Layout,          // Block and instruction ordering
    pub signature: Signature,    // Function type signature
}
```

### DataFlowGraph

Manages values and their definitions:

```rust
pub struct DataFlowGraph {
    pub insts: PrimaryMap<Inst, InstructionData>,
    pub results: SecondaryMap<Inst, Value>,
    pub values: PrimaryMap<Value, ValueDef>,
}
```

### Instructions

MIR supports various instruction types:

- **Unary operations** - `Fneg`, `Ineg`, `Bnot`, `Sqrt`, `Exp`, `Ln`, etc.
- **Binary operations** - `Fadd`, `Fsub`, `Fmul`, `Fdiv`, `Iadd`, etc.
- **Comparisons** - `Flt`, `Fgt`, `Feq`, `Ilt`, `Igt`, `Ieq`, etc.
- **Control flow** - `Br`, `Jmp`, `Exit`
- **Phi nodes** - `Phi` for SSA join points
- **Calls** - `Call` for external function invocation
- **Casts** - `FIcast`, `IFcast`, `BIcast`, etc.

### Opcodes

Complete opcode enumeration:

```rust
pub enum Opcode {
    // Unary float
    Fneg, Sqrt, Exp, Ln, Log, Floor, Ceil,
    Sin, Cos, Tan, Asin, Acos, Atan,
    Sinh, Cosh, Tanh, Asinh, Acosh, Atanh,

    // Binary float
    Fadd, Fsub, Fmul, Fdiv, Frem, Pow, Hypot, Atan2,

    // Integer operations
    Iadd, Isub, Imul, Idiv, Irem,
    Ishl, Ishr, Ixor, Iand, Ior,

    // Comparisons
    Flt, Fgt, Fle, Fge, Feq, Fne,
    Ilt, Igt, Ile, Ige, Ieq, Ine,

    // Control flow
    Br, Jmp, Phi, Call, Exit,

    // Type casts
    FIcast, IFcast, BIcast, IBcast, FBcast, BFcast,

    // Misc
    OptBarrier, Clog2,
}
```

### Values

Values can be:
- **Instruction results** - Produced by instructions
- **Parameters** - Function arguments
- **Constants** - Immediate values

```rust
pub enum ValueDef {
    Result(Inst, u16),
    Param(Param),
    Const(Const),
    Invalid,
}
```

## Block Structure

```rust
pub struct Layout {
    blocks: SecondaryMap<Block, BlockNode>,
    insts: SecondaryMap<Inst, InstNode>,
    first_block: Option<Block>,
    last_block: Option<Block>,
}
```

## Validation

The function includes validation for:
- SSA dominance properties
- Type consistency
- Control flow integrity

```rust
impl Function {
    pub fn validate(&self) -> bool {
        // Validates SSA properties and type consistency
    }
}
```

## Printing

MIR can be pretty-printed for debugging:

```rust
let output = function.print(&literals);
println!("{}", output);
```

## Dependencies

- `bforest` - B-tree forest for phi node edges
- `lasso` - String interner for literals
- `typed-index-collections` - Type-safe indices
