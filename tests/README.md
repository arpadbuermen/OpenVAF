# OpenVAF Test Suite

This directory contains documentation and utilities for the OpenVAF test infrastructure.

## Quick Reference

```bash
# Run fast tests only (default)
cargo test

# Run release version tests
cargo test --release

# Run all tests including integration tests
RUN_DEV_TESTS=1 cargo test --release

# Run slow tests
RUN_SLOW_TESTS=1 cargo test

# Update snapshot expectations
UPDATE_EXPECT=1 cargo test

# Run with code coverage (requires cargo-llvm-cov)
cargo llvm-cov --html
```

## Test Organization

### Test Data (`openvaf/test_data/`)

Snapshot tests comparing compiler output against known-good references:

| Directory | Purpose | Input | Output |
|-----------|---------|-------|--------|
| `ast/` | AST generation | `.va` | `.snap` |
| `body/` | Body-level IR | `.va` | `.body` |
| `item_tree/` | Item trees and def maps | `.va` | `.item_tree`, `.def_map` |
| `mir/` | Mid-level IR | `.va` | `.mir` |
| `dae/` | DAE system generation | `.va` | `_system.snap`, `_mir.snap` |
| `init/` | Initialization systems | `.va` | `.snap` |
| `contributions/` | Contribution topology | `.va` | `_topology.snap`, `_mir.snap` |
| `osdi/` | OSDI descriptors | `.va` | `.snap` |
| `ui/` | Error diagnostics | `.va` | `.log` |
| `syn_ui/` | Syntax error diagnostics | `.va` | `.log` |

### Integration Tests (`integration_tests/`)

Real-world Verilog-A compact models:

| Model | Type | Description |
|-------|------|-------------|
| `AMPLIFIER` | Behavioral | Simple amplifier model |
| `BSIM3`, `BSIM4`, `BSIM6` | MOSFET | Berkeley MOSFET models |
| `BSIMBULK`, `BSIMCMG`, `BSIMIMG`, `BSIMSOI` | MOSFET | Advanced BSIM variants |
| `DIODE`, `DIODE_CMC` | Diode | PN junction models |
| `EKV`, `EKV_LONGCHANNEL` | MOSFET | EKV MOSFET model |
| `HICUML2` | BJT | High-current bipolar model |
| `HiSIM2`, `HiSIMHV`, `HiSIMSOTB` | MOSFET | HiSIM models |
| `MEXTRAM` | BJT | Mextram bipolar model |
| `PSP102`, `PSP103` | MOSFET | PSP models |
| `RESISTOR` | Passive | Resistor model |
| `CCCS`, `VCCS` | Source | Controlled sources |
| `CURRENT_SOURCE` | Source | Current source |
| `ASMHEMT`, `MVSG_CMC` | HEMT | GaN HEMT models |
| `MODULE_INST` | Multi-module | Module instantiation test |
| `BUILTIN_PRIMITIVES` | Primitives | Built-in primitive test |
| `STRINGS` | Misc | String parameter test |

### Unit Tests (in source files)

Tests located within crate source directories:

| Location | Tests |
|----------|-------|
| `openvaf/lexer/src/tests.rs` | Lexer tokenization |
| `openvaf/preprocessor/src/tests.rs` | Macro expansion |
| `openvaf/mir/src/*/tests.rs` | MIR data structures |
| `openvaf/mir_autodiff/src/*/tests.rs` | Automatic differentiation |
| `openvaf/mir_opt/src/*/tests.rs` | MIR optimizations |
| `openvaf/sim_back/src/*/tests.rs` | Simulation backend |
| `openvaf/osdi/src/tests.rs` | OSDI generation |

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUN_SLOW_TESTS` | Enable slow/intensive tests | Not set (skip) |
| `RUN_DEV_TESTS` | Enable integration tests | Not set (skip) |
| `UPDATE_EXPECT` | Regenerate snapshots | Not set (verify) |
| `RAYON_NUM_THREADS` | Control parallelism | Auto |
| `CI` | CI environment detection | Not set |

## Adding New Tests

### Snapshot Test

1. Create a `.va` file in the appropriate `test_data/` subdirectory
2. Run `UPDATE_EXPECT=1 cargo test <test_name>` to generate snapshot
3. Review the generated `.snap` file
4. Commit both files

### Integration Test

1. Create a directory in `integration_tests/`
2. Add the main `.va` file (lowercase name matching directory)
3. Run `RUN_DEV_TESTS=1 UPDATE_EXPECT=1 cargo test --release --test integration <name>`
4. Review the generated OSDI snapshot
5. Commit all files

### Unit Test

Add `#[test]` functions in the appropriate `tests.rs` file:

```rust
#[test]
fn test_feature() {
    let input = "...";
    let expected = expect![[r#"..."#]];
    expected.assert_eq(&actual_output);
}
```

## Test Framework

OpenVAF uses a custom test harness (`lib/mini_harness/`) that provides:

- Data-driven tests from directories
- Filtered test execution
- Integration with `expect-test` for snapshots

## Code Coverage

To generate code coverage reports:

```bash
# Install cargo-llvm-cov (one-time)
cargo install cargo-llvm-cov

# Generate HTML coverage report
cargo llvm-cov --html

# Generate and open report
cargo llvm-cov --open

# Generate coverage for specific tests
RUN_DEV_TESTS=1 cargo llvm-cov --release --html
```

Coverage reports are generated in `target/llvm-cov/html/`.

## Known Limitations

- Some tests skip on Windows in CI
- Integration tests disabled by default (slow)
- Coverage requires LLVM-based instrumentation

## Related Issues

See GitHub issues with "test coverage" label for known gaps.
