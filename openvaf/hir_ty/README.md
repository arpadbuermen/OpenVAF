# hir_ty

Type inference and checking for the OpenVAF Verilog-A compiler.

## Overview

The `hir_ty` crate implements the type system for Verilog-A, providing:

- Type inference for expressions
- Type checking and validation
- Type coercion rules
- Diagnostic generation for type errors

## Key Components

### Type System

The Verilog-A type system includes:

```rust
pub enum Type {
    Real,           // Floating-point numbers
    Integer,        // Integer numbers
    String,         // String values
    Bool,           // Boolean (internal)
    Void,           // No value
    Array { ty, len },  // Fixed-size arrays
    EmptyArray,     // Zero-length array
    Err,            // Error type for recovery
}
```

### Type Inference

The crate performs bidirectional type inference:

1. **Bottom-up inference** - Infer types from literals and known types
2. **Top-down checking** - Check expressions against expected types
3. **Coercion insertion** - Insert implicit type conversions

### InferenceResult

Stores the results of type inference for a body:

```rust
pub struct InferenceResult {
    pub expr_types: ArenaMap<ExprId, Type>,
    pub diagnostics: Vec<InferenceDiagnostic>,
}
```

### Type Coercion

Supported implicit coercions:

| From | To | Notes |
|------|-----|-------|
| Integer | Real | Widening |
| Bool | Integer | 0 or 1 |
| Bool | Real | 0.0 or 1.0 |
| Real | Integer | Truncation (with warning) |

### Database Queries

Type-related queries via `HirTyDB`:

```rust
#[salsa::query_group(HirTyDBStorage)]
pub trait HirTyDB: HirDefDB {
    fn inference_result(&self, owner: DefWithBodyId) -> Arc<InferenceResult>;
    fn param_ty(&self, param: ParamId) -> Type;
    fn var_ty(&self, var: VarId) -> Type;
}
```

### Diagnostics

Type-related diagnostics include:

- Type mismatch errors
- Invalid operation for type
- Array dimension mismatches
- Invalid coercion warnings

## Special Handling

### System Functions

Type signatures for builtin system functions like:
- `$temperature` - Returns real
- `$vt` - Returns real (thermal voltage)
- `$simparam` - Returns real
- `$param_given` - Returns integer (boolean)

### Analog Operators

Type rules for analog operators:
- `ddt(x)` - Same type as input
- `idt(x)` - Same type as input
- `limexp(x)` - Real
- `absdelay(x, delay)` - Same type as input

## Dependencies

- `hir_def` - HIR definitions with type annotations
- `basedb` - Base database for diagnostics
- `salsa` - Incremental computation
