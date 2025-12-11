# mir_autodiff

Automatic differentiation for MIR in the OpenVAF compiler.

## Overview

The `mir_autodiff` crate implements automatic differentiation (autodiff) on MIR functions. This is essential for computing Jacobian matrices required by circuit simulators, enabling efficient derivative computation without manual differentiation of Verilog-A models.

## Features

- **Forward-mode AD** - Computes derivatives alongside function evaluation
- **Sparse Jacobian support** - Only computes needed derivatives
- **Symbolic differentiation** - Transforms MIR operations to their derivatives
- **Multi-variable support** - Handles derivatives with respect to multiple inputs

## Algorithm

The autodiff implementation uses forward-mode automatic differentiation:

1. **Seed propagation** - Mark input variables with derivative seeds
2. **Instruction transformation** - Apply differentiation rules to each instruction
3. **Chain rule application** - Combine derivatives through operation chains
4. **Dead code elimination** - Remove unused derivative computations

## Derivative Rules

Each MIR opcode has corresponding derivative rules:

| Operation | Derivative |
|-----------|------------|
| `Fadd(a, b)` | `da + db` |
| `Fsub(a, b)` | `da - db` |
| `Fmul(a, b)` | `a*db + b*da` |
| `Fdiv(a, b)` | `(da*b - a*db) / (b*b)` |
| `Sqrt(x)` | `dx / (2*sqrt(x))` |
| `Exp(x)` | `exp(x) * dx` |
| `Ln(x)` | `dx / x` |
| `Sin(x)` | `cos(x) * dx` |
| `Cos(x)` | `-sin(x) * dx` |
| `Pow(x, y)` | `pow(x,y) * (y*dx/x + ln(x)*dy)` |

## Usage

```rust
use mir_autodiff::autodiff;

// Differentiate function with respect to specified parameters
let (diff_func, jacobian_map) = autodiff(
    &function,
    &derivative_targets,  // Which outputs need derivatives
    &derivative_inputs,   // With respect to which inputs
)?;
```

## Jacobian Computation

For circuit simulation, the Jacobian matrix ∂I/∂V is computed:

```
∂I_i/∂V_j = derivative of current at node i
            with respect to voltage at node j
```

The autodiff pass generates code that computes these derivatives efficiently alongside the current evaluation.

## Integration with sim_back

The `sim_back` crate uses `mir_autodiff` to:

1. Build the DAE (Differential-Algebraic Equation) system
2. Compute derivatives for Newton-Raphson iteration
3. Handle implicit equations from `idt()` and `ddt()` operators

## Performance Considerations

- Derivatives are computed lazily - only needed derivatives are generated
- Common subexpressions in derivatives are shared
- Constant derivatives (0 or 1) are propagated at compile time

## Dependencies

- `mir` - MIR representation being differentiated
- `bitset` - Efficient sparse sets for tracking live derivatives
