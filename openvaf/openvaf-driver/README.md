# openvaf-driver

Command-line interface for the OpenVAF Verilog-A compiler.

## Overview

The `openvaf-driver` crate provides the `openvaf-r` command-line tool, which is the main user interface for compiling Verilog-A models into OSDI-compatible dynamic libraries.

## Installation

Build from source:

```bash
cargo build --release --bin openvaf-r
```

The binary will be at `target/release/openvaf-r`.

## Usage

```bash
# Basic compilation
openvaf-r model.va

# Specify output path
openvaf-r model.va -o model.osdi

# Cross-compilation
openvaf-r model.va --target x86_64-unknown-linux-gnu

# With include directories
openvaf-r model.va -I /path/to/includes

# With preprocessor defines
openvaf-r model.va -D MACRO_NAME=value

# Optimization level
openvaf-r model.va -O3

# Print preprocessor output
openvaf-r model.va --print-expansion
```

## Command-Line Options

### Input/Output

| Option | Description |
|--------|-------------|
| `<INPUT>` | Input Verilog-A file |
| `-o, --output <FILE>` | Output file path |
| `--cache <DIR>` | Use caching with specified directory |

### Preprocessing

| Option | Description |
|--------|-------------|
| `-I, --include <DIR>` | Add include search path |
| `-D, --define <MACRO>` | Define preprocessor macro |
| `--print-expansion` | Print preprocessed output |

### Optimization

| Option | Description |
|--------|-------------|
| `-O<LEVEL>` | Optimization level (0-3) |
| `--target <TRIPLE>` | Target triple |
| `--target-cpu <CPU>` | Target CPU |

### Diagnostics

| Option | Description |
|--------|-------------|
| `-A, --allow <LINT>` | Set lint to allow |
| `-W, --warn <LINT>` | Set lint to warn |
| `-D, --deny <LINT>` | Set lint to deny |

### Debug

| Option | Description |
|--------|-------------|
| `--dump-mir` | Dump optimized MIR |
| `--dump-unopt-mir` | Dump unoptimized MIR |
| `--dump-ir` | Dump optimized LLVM IR |
| `--dump-unopt-ir` | Dump unoptimized LLVM IR |

### Other

| Option | Description |
|--------|-------------|
| `-n, --dry-run` | Check without generating output |
| `-h, --help` | Print help information |
| `-V, --version` | Print version |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENVAF_LOG` | Log level filter |
| `OPENVAF_LOG_STYLE` | Log output style |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 65 | Compilation error (DATA_ERROR) |
| Other | Internal error |

## Examples

### Compile a BSIM4 model

```bash
openvaf-r bsim4.va -o bsim4.osdi -O2
```

### Debug compilation issues

```bash
openvaf-r model.va --dump-mir --dump-ir
```

### Cross-compile for Linux from macOS

```bash
openvaf-r model.va --target x86_64-unknown-linux-gnu -o model_linux.osdi
```

## Dependencies

- `openvaf` - Core compilation library
- `clap` - Command-line argument parsing
- `mimalloc` - High-performance memory allocator
