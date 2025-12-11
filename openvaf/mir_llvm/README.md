# mir_llvm

LLVM code generation from MIR for the OpenVAF compiler.

## Overview

The `mir_llvm` crate translates MIR (Mid-Level Intermediate Representation) into LLVM IR for native code generation. It interfaces with LLVM 18 via the `llvm-sys` bindings to produce optimized machine code.

## Key Components

### LLVMBackend

The main backend structure for LLVM code generation:

```rust
pub struct LLVMBackend<'t> {
    target: &'t Target,
    target_cpu: String,
    features: String,
}

impl<'t> LLVMBackend<'t> {
    pub fn new(
        cg_opts: &[String],
        target: &'t Target,
        target_cpu: String,
        target_features: &[String],
    ) -> LLVMBackend<'t>;

    pub unsafe fn new_module(
        &self,
        name: &str,
        opt_lvl: LLVMCodeGenOptLevel,
    ) -> Result<ModuleLlvm, LLVMString>;

    pub unsafe fn new_ctx<'a, 'll>(
        &'a self,
        literals: &'a Rodeo,
        module: &'ll ModuleLlvm,
    ) -> CodegenCx<'a, 'll>;
}
```

### ModuleLlvm

Wrapper around LLVM module with associated context:

```rust
pub struct ModuleLlvm {
    llcx: LLVMContextRef,
    llmod_raw: LLVMModuleRef,
    tm: LLVMTargetMachineRef,
    opt_lvl: LLVMCodeGenOptLevel,
}

impl ModuleLlvm {
    pub fn optimize(&self);
    pub fn verify(&self) -> Option<LLVMString>;
    pub fn emit_object(&self, dst: &Path) -> Result<(), LLVMString>;
    pub fn to_str(&self) -> LLVMString;
}
```

### CodegenCx

Codegen context providing LLVM type and value helpers:

```rust
pub struct CodegenCx<'a, 'll> {
    // Type construction
    pub fn ty_int(&self) -> &'ll LLVMType;
    pub fn ty_double(&self) -> &'ll LLVMType;
    pub fn ty_bool(&self) -> &'ll LLVMType;
    pub fn ty_ptr(&self) -> &'ll LLVMType;
    pub fn ty_array(&self, elem: &'ll LLVMType, len: u32) -> &'ll LLVMType;

    // Constant construction
    pub fn const_int(&self, val: i32) -> &'ll LLVMValue;
    pub fn const_double(&self, val: f64) -> &'ll LLVMValue;
    pub fn const_bool(&self, val: bool) -> &'ll LLVMValue;
}
```

### Builder

LLVM IR builder for instruction emission:

```rust
pub struct Builder<'a, 'll> {
    pub fn fadd(&self, a: &'ll LLVMValue, b: &'ll LLVMValue) -> &'ll LLVMValue;
    pub fn fsub(&self, a: &'ll LLVMValue, b: &'ll LLVMValue) -> &'ll LLVMValue;
    pub fn fmul(&self, a: &'ll LLVMValue, b: &'ll LLVMValue) -> &'ll LLVMValue;
    pub fn fdiv(&self, a: &'ll LLVMValue, b: &'ll LLVMValue) -> &'ll LLVMValue;
    // ... more operations
}
```

## Code Generation Process

1. **Module creation** - Create LLVM module with target configuration
2. **Type generation** - Generate LLVM types for MIR types
3. **Function generation** - Translate MIR functions to LLVM functions
4. **Instruction translation** - Convert MIR instructions to LLVM IR
5. **Optimization** - Run LLVM optimization passes
6. **Object emission** - Generate native object code

## Optimization Levels

Supports standard LLVM optimization levels:

| Level | Description |
|-------|-------------|
| `O0` | No optimization |
| `O1` | Basic optimization |
| `O2` | Standard optimization |
| `O3` | Aggressive optimization |

## Intrinsics

The crate provides LLVM intrinsics for math functions:

- `llvm.sqrt.f64`
- `llvm.exp.f64`
- `llvm.log.f64`
- `llvm.sin.f64`
- `llvm.cos.f64`
- `llvm.pow.f64`
- etc.

## Target Support

Supports multiple targets through LLVM:

- x86-64 (Linux, macOS, Windows)
- AArch64 (Linux, macOS)
- Other LLVM-supported targets

## Dependencies

- `llvm-sys` (v181) - LLVM C API bindings
- `mir` - Source MIR representation
- `target` - Target specification
- `lasso` - String interner for literals
