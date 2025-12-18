# linker

Platform-specific linker invocation for the OpenVAF compiler.

## Overview

The `linker` crate handles linking compiled object files into dynamic libraries (`.osdi` files). It provides a cross-platform abstraction over system linkers, supporting Linux, macOS, and Windows.

## Features

- **Cross-platform support** - Works with platform-native linkers
- **LLD integration** - Uses LLVM's LLD linker when available
- **Symbol export** - Properly exports OSDI API symbols
- **Position-independent code** - Generates PIC for shared libraries

## Supported Platforms

| Platform | Linker | Output Format |
|----------|--------|---------------|
| Linux | `ld` / `lld` | ELF shared object |
| macOS | `ld64` / `lld` | Mach-O dylib |
| Windows | `link.exe` / `lld-link` | PE DLL |

## Usage

```rust
use linker::link;
use target::spec::Target;

// Link object files into a shared library
link(
    linker_path,      // Optional custom linker path
    &target,          // Target specification
    output_path,      // Output library path
    |linker| {
        linker.add_object("module1.o");
        linker.add_object("module2.o");
    }
)?;
```

## Linker Interface

The `Linker` trait provides methods for building link commands:

```rust
pub trait Linker {
    fn add_object(&mut self, path: &Path);
    fn link_dylib(&mut self, name: &str);
    fn output_path(&mut self, path: &Path);
    fn finalize(&mut self) -> Command;
}
```

## Platform-Specific Notes

### Linux
- Uses `-shared` flag for shared library output
- Links against `libm` for math functions
- Sets appropriate `SONAME`

### macOS
- Uses `-dylib` flag
- Sets `-install_name` for library identification
- Handles universal binary considerations

### Windows
- Creates import library alongside DLL
- Exports symbols via `.def` file or `__declspec(dllexport)`

## Dependencies

- `target` - Target specification and platform detection
