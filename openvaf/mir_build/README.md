# mir_build

MIR construction utilities for the OpenVAF compiler.

## Overview

The `mir_build` crate provides utilities for constructing MIR (Mid-Level Intermediate Representation) functions. It offers a builder pattern API for creating well-formed MIR, handling SSA construction and basic block management.

## Key Components

### FunctionBuilder

The main API for constructing MIR functions:

```rust
pub struct FunctionBuilder<'a> {
    func: &'a mut Function,
    current_block: Option<Block>,
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(func: &'a mut Function) -> Self;

    // Block management
    pub fn create_block(&mut self) -> Block;
    pub fn switch_to_block(&mut self, block: Block);
    pub fn seal_block(&mut self, block: Block);

    // Instruction insertion
    pub fn ins(&mut self) -> InstructionBuilder;
}
```

### InstructionBuilder

Builder for inserting instructions:

```rust
impl InstructionBuilder {
    // Arithmetic
    pub fn fadd(&mut self, a: Value, b: Value) -> Value;
    pub fn fsub(&mut self, a: Value, b: Value) -> Value;
    pub fn fmul(&mut self, a: Value, b: Value) -> Value;
    pub fn fdiv(&mut self, a: Value, b: Value) -> Value;

    // Control flow
    pub fn br(&mut self, cond: Value, then_bb: Block, else_bb: Block);
    pub fn jmp(&mut self, target: Block);
    pub fn exit(&mut self);

    // Constants
    pub fn fconst(&mut self, val: f64) -> Value;
    pub fn iconst(&mut self, val: i32) -> Value;
}
```

## SSA Construction

The builder handles SSA construction automatically:

1. **Variable tracking** - Tracks current definition of variables
2. **Phi node insertion** - Automatically inserts phi nodes at join points
3. **Block sealing** - Finalizes blocks when all predecessors are known

```rust
// Example: Building a simple function
let mut func = Function::default();
let mut builder = FunctionBuilder::new(&mut func);

let entry = builder.create_block();
builder.switch_to_block(entry);

let x = builder.ins().fconst(1.0);
let y = builder.ins().fconst(2.0);
let sum = builder.ins().fadd(x, y);

builder.ins().exit();
builder.seal_block(entry);
```

## Block Management

Proper block management is essential for well-formed MIR:

1. **Create blocks** before referencing them in branches
2. **Switch to block** before inserting instructions
3. **Seal blocks** once all predecessors are connected

## Dependencies

- `mir` - Target MIR representation
- `bforest` - B-tree forest for phi node management
