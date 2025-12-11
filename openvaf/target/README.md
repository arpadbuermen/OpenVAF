# target

Target specifications for the OpenVAF compiler.

## Overview

The `target` crate defines target platform specifications that control code generation. It provides information about target architectures, ABIs, and platform-specific settings needed for cross-compilation.

## Key Components

### Target Specification

Each target has a specification including:

```rust
pub struct Target {
    pub llvm_target: String,        // LLVM target triple
    pub data_layout: String,        // LLVM data layout string
    pub options: TargetOptions,     // Platform-specific options
}

pub struct TargetOptions {
    pub cpu: String,                // Default CPU
    pub features: String,           // CPU features
    pub is_builtin: bool,           // Built-in vs custom target
    // ...
}
```

### Host Triple

Get the current host's target triple:

```rust
pub fn host_triple() -> &'static str;
```

This returns the triple of the machine running the compiler, useful for native compilation.

## Supported Targets

Built-in targets include:

### Linux
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `riscv64-unknown-linux`

### macOS
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

### Windows
- `x86_64-pc-windows-msvc`
- `x86_64-pc-windows-gnu`

## Target Discovery

```rust
pub fn get_target_names() -> Vec<&'static str>;
```

Returns list of all available target names.

## Data Layout

The data layout string specifies:

- Endianness
- Pointer size
- Alignment requirements
- Type sizes

Example for x86-64:
```
e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128
```

## Platform-Specific Notes

### Linux
- Uses System V ABI
- PIC (Position Independent Code) required for shared libraries

### macOS
- Uses Mach-O binary format
- Universal binary considerations for Apple Silicon

### Windows
- Supports both MSVC and MinGW toolchains
- DLL export handling differs from Unix

## Build-Time Configuration

The crate uses a build script to determine the host triple:

```rust
// build.rs
fn main() {
    let triple = std::env::var("TARGET").unwrap();
    println!("cargo:rustc-env=CFG_COMPILER_HOST_TRIPLE={}", triple);
}
```

## Dependencies

- `stdx` - Standard library extensions
- `xshell` - Build script shell commands
