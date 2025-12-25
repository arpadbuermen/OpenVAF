# Testing

OpenVAF uses a multi-layered testing approach combining snapshot tests, integration tests, and unit tests. This guide explains how the test infrastructure works and how to add new tests.

## Quick Reference

```bash
# Run fast tests only (default)
cargo test

# Run release version tests
cargo test --release

# Run all tests including integration tests
RUN_DEV_TESTS=1 cargo test

# Run slow tests
RUN_SLOW_TESTS=1 cargo test

# Update snapshot expectations
UPDATE_EXPECT=1 cargo test

# Run specific integration tests
RUN_DEV_TESTS=1 cargo test --release --test integration
```

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUN_SLOW_TESTS` | Run slow/intensive tests | Not set (skip) |
| `RUN_DEV_TESTS` | Run integration tests with real device models | Not set (skip) |
| `UPDATE_EXPECT` | Regenerate snapshot files instead of comparing | Not set (verify mode) |
| `RAYON_NUM_THREADS=1` | Disable parallelization for debugging | Not set |

## Test Types

### 1. Snapshot Tests

Snapshot tests compare generated output against known-good reference files. They are the primary testing mechanism for the compiler pipeline.

**Location:** `openvaf/test_data/`

| Directory | Tests |
|-----------|-------|
| `ast/` | Abstract Syntax Tree generation |
| `body/` | Body-level IR |
| `item_tree/` | Item tree and definition maps |
| `mir/` | Mid-Level Intermediate Representation |
| `dae/` | Differential-Algebraic Equation systems |
| `init/` | Initialization systems |
| `contributions/` | Contribution topology |
| `osdi/` | OSDI descriptor generation |
| `ui/` | User-facing error diagnostics |
| `syn_ui/` | Syntax-level error diagnostics |

**File naming pattern:**
- Input: `{name}.va` - Verilog-A source file
- Output: `{name}.snap` or `{name}_{type}.snap` - Expected output

#### How Snapshot Tests Work

Tests use the `expect-test` crate with the `expect_file!` macro:

```rust
use expect_test::expect_file;

fn run_test(name: &str) {
    let source = read_file(&format!("{name}.va"));
    let output = compile(&source);

    // Compare output against snapshot file
    expect_file![format!("{name}.snap")].assert_eq(&output);
}
```

When `UPDATE_EXPECT=1` is set, failing tests will update the snapshot file instead of failing.

#### Adding a New Snapshot Test

1. Create a new `.va` file in the appropriate `test_data/` subdirectory:

```verilog
// openvaf/test_data/mir/my_new_test.va
`include "disciplines.vams"

module my_test(a, b);
    inout a, b;
    electrical a, b;

    analog begin
        I(a, b) <+ V(a, b) * 1e-3;
    end
endmodule
```

2. Run the tests to generate the snapshot:

```bash
UPDATE_EXPECT=1 cargo test my_new_test
```

3. Review the generated `.snap` file and commit both files.

### 2. Integration Tests

Integration tests compile real-world Verilog-A device models and verify the generated OSDI libraries work correctly.

**Location:** `integration_tests/`

Each subdirectory contains a complete device model:
- `BSIM3/`, `BSIM4/` - Berkeley MOSFET models
- `MEXTRAM/` - Bipolar transistor model
- `HICUML2/` - High-current bipolar model
- `PSP102/`, `PSP103/` - PSP MOSFET models
- `EKV/` - EKV MOSFET model
- And many more...

#### Running Integration Tests

```bash
# Run all integration tests
RUN_DEV_TESTS=1 cargo test --release --test integration

# Run a specific model
RUN_DEV_TESTS=1 cargo test --release --test integration BSIM4
```

#### How Integration Tests Work

The integration test harness (`openvaf/tests/integration.rs`):

1. Loads each model directory from `integration_tests/`
2. Compiles the `.va` file to an OSDI library
3. Verifies the OSDI descriptor structure
4. Compares output against snapshot files

#### Adding a New Integration Test

1. Create a new directory in `integration_tests/`:

```
integration_tests/
└── MY_MODEL/
    ├── my_model.va      # Main Verilog-A source
    ├── includes/        # Optional: include files
    └── LICENSE          # If model has specific license
```

2. Run the test to generate snapshots:

```bash
RUN_DEV_TESTS=1 UPDATE_EXPECT=1 cargo test --release --test integration MY_MODEL
```

3. Review and commit the generated files.

### 3. Inline Expectation Tests

Some tests use inline expectations hardcoded in the test source code. These are useful for small, focused tests.

**Example from** `openvaf/mir_autodiff/src/builder/tests.rs`:

```rust
#[test]
fn test_derivative() {
    let src = r#"
        module test(a, b);
            ...
        endmodule
    "#;

    let expect = expect![[r#"
        function %bar(v10, v11) {
            inst0 = const fn %ddx_v10(1) -> 1
            ...
        }
    "#]];

    check(src, expect);
}
```

To update inline expectations, run with `UPDATE_EXPECT=1`. Note that this may require manual source code edits for complex cases.

### 4. Unit Tests

Traditional Rust `#[test]` functions exist throughout the codebase:

- `openvaf/lexer/src/tests.rs` - Lexer tests
- `openvaf/preprocessor/src/tests.rs` - Preprocessor tests
- `openvaf/mir/src/layout/tests.rs` - MIR layout tests

## Test Infrastructure

### Mini Harness

OpenVAF uses a custom test harness (`lib/mini_harness/`) instead of the default Rust test runner. This enables:

- Data-driven tests from directories
- Filtered test execution
- Custom output formatting

**Example test declaration:**

```rust
harness! {
    Test::from_dir_filtered(
        "integration",
        &integration_test,
        &Path::is_dir,
        &ignore_dev_tests,
        &project_root().join("integration_tests")
    ),
    Test::from_dir_filtered(
        "ui",
        &ui_test,
        &is_va_file,
        &ignore_never,
        &openvaf_test_data("ui")
    )
}
```

### Helper Functions

The `lib/stdx/` crate provides test utilities:

```rust
use stdx::{project_root, openvaf_test_data, integration_test_dir};

// Get project root directory
let root = project_root();

// Get path to test data
let ui_tests = openvaf_test_data("ui");

// Get integration test directory
let bsim4 = integration_test_dir("BSIM4");
```

## Best Practices

### When Writing Tests

1. **Use snapshot tests** for compiler output (AST, MIR, diagnostics)
2. **Use integration tests** for end-to-end compilation of real models
3. **Use unit tests** for small, isolated functionality
4. **Keep test inputs minimal** - use the smallest Verilog-A that reproduces the behavior

### When Updating Snapshots

1. Always review snapshot changes before committing
2. Run `UPDATE_EXPECT=1` only when you intentionally changed behavior
3. Check that snapshot changes match your expectations
4. Consider whether the change affects other tests

### Debugging Test Failures

```bash
# Run single test with output
cargo test test_name -- --nocapture

# Disable parallel execution for easier debugging
RAYON_NUM_THREADS=1 cargo test test_name

# Get verbose output
cargo test -- --test-threads=1

# Compare expected vs actual
diff expected.snap actual_output.txt
```

## Test Data Structure

```
openvaf/test_data/
├── ast/                # AST snapshot tests
│   ├── test1.va
│   └── test1.snap
├── body/               # Body IR tests
│   ├── test1.va
│   └── test1.body
├── item_tree/          # Item tree tests (3 files per test)
│   ├── test1.va
│   ├── test1.item_tree
│   └── test1.def_map
├── mir/                # MIR tests
├── osdi/               # OSDI descriptor tests
├── ui/                 # Error message tests
└── syn_ui/             # Syntax error tests

integration_tests/
├── AMPLIFIER/
├── BSIM3/
├── BSIM4/
├── DIODE/
├── EKV/
├── HICUML2/
├── MEXTRAM/
├── PSP102/
├── PSP103/
├── RESISTOR/
└── ... (30+ models)
```
