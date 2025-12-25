# mir_interpret

MIR interpreter for testing and debugging in the OpenVAF compiler.

## Overview

The `mir_interpret` crate provides an interpreter for executing MIR (Mid-Level Intermediate Representation) functions directly. This is primarily used for testing MIR transformations and validating optimization passes.

## Key Components

### Interpreter

The main interpreter structure:

```rust
pub struct Interpreter<'a> {
    pub state: InterpreterState,
    calls: &'a TiSlice<FuncRef, (Func<'a>, *mut c_void)>,
    func: &'a Function,
}

impl<'a> Interpreter<'a> {
    pub fn new(
        func: &'a Function,
        calls: &'a TiSlice<FuncRef, (Func<'a>, *mut c_void)>,
        args: &TiSlice<Param, Data>,
    ) -> Interpreter<'a>;

    pub fn run(&mut self);
    pub fn eval(&mut self, inst: Inst);
}
```

### InterpreterState

Maintains execution state during interpretation:

```rust
pub struct InterpreterState {
    vals: TiVec<Value, Data>,
    prev_bb: Block,
    next_inst: Option<Inst>,
}

impl InterpreterState {
    pub fn write(&mut self, dst: Value, val: impl Into<Data>);
    pub fn read<T: From<Data>>(&self, val: Value) -> T;
}
```

### Data Type

Represents runtime values:

```rust
pub enum Data {
    Real(f64),
    Int(i32),
    Bool(bool),
    Str(Spur),  // Interned string
    Undef,
}
```

## Supported Operations

The interpreter supports all MIR opcodes:

### Arithmetic
- Float: `Fadd`, `Fsub`, `Fmul`, `Fdiv`, `Frem`
- Integer: `Iadd`, `Isub`, `Imul`, `Idiv`, `Irem`
- Bitwise: `Ishl`, `Ishr`, `Ixor`, `Iand`, `Ior`

### Math Functions
- `Sqrt`, `Exp`, `Ln`, `Log`
- `Sin`, `Cos`, `Tan`, `Asin`, `Acos`, `Atan`
- `Sinh`, `Cosh`, `Tanh`, `Asinh`, `Acosh`, `Atanh`
- `Hypot`, `Atan2`, `Pow`
- `Floor`, `Ceil`, `Clog2`

### Comparisons
- Float: `Flt`, `Fgt`, `Fle`, `Fge`, `Feq`, `Fne`
- Integer: `Ilt`, `Igt`, `Ile`, `Ige`, `Ieq`, `Ine`
- Boolean: `Beq`, `Bne`
- String: `Seq`, `Sne`

### Control Flow
- `Br` - Conditional branch
- `Jmp` - Unconditional jump
- `Phi` - Phi node evaluation
- `Call` - External function calls
- `Exit` - Function termination

### Type Casts
- `FIcast`, `IFcast` - Float/integer conversion
- `BIcast`, `IBcast` - Bool/integer conversion
- `FBcast`, `BFcast` - Float/bool conversion

## Usage

```rust
use mir_interpret::Interpreter;

// Create interpreter for a function
let mut interp = Interpreter::test(&function);

// Run to completion
interp.run();

// Read result value
let result: f64 = interp.state.read(result_value);
```

## External Calls

The interpreter supports external function callbacks:

```rust
pub type Func<'a> = fn(
    &mut InterpreterState,
    &[Value],      // Arguments
    &[Value],      // Return values
    *mut c_void    // User data
);
```

## Dependencies

- `mir` - MIR representation being interpreted
- `lasso` - String interner for string values
