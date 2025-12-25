# vfs

Virtual File System for the OpenVAF compiler.

## Overview

The `vfs` crate provides a virtual file system abstraction that stores all source files. It serves as the foundation for incremental compilation by tracking file changes and providing stable file identifiers.

## Features

- **File identity** - Stable `FileId` references for files
- **Change tracking** - Records file modifications
- **Encoding handling** - Automatic character encoding detection
- **Line ending normalization** - Converts CRLF to LF

## Key Components

### FileId

Opaque identifier for files:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct FileId(pub u16);
```

### VfsPath

Abstract file path representation:

```rust
pub enum VfsPath {
    PathBuf(AbsPathBuf),    // Real filesystem path
    Virtual(String),         // Virtual/in-memory path
}

impl VfsPath {
    pub fn new_virtual_path(path: String) -> VfsPath;
    pub fn as_path(&self) -> Option<&AbsPath>;
}
```

### Vfs

The main virtual file system:

```rust
pub struct Vfs {
    interner: PathInterner,
    data: Vec<VfsEntry>,
    changes: Vec<ChangedFile>,
}

impl Vfs {
    pub fn file_id(&self, path: &VfsPath) -> Option<FileId>;
    pub fn file_path(&self, file_id: FileId) -> VfsPath;
    pub fn file_contents(&self, file_id: FileId) -> Result<&str, FileReadError>;
    pub fn set_file_contents(&mut self, file_id: FileId, contents: VfsEntry) -> bool;
    pub fn take_changes(&mut self) -> Vec<ChangedFile>;
}
```

### Change Tracking

```rust
pub struct ChangedFile {
    pub file_id: FileId,
    pub change_kind: ChangeKind,
}

pub enum ChangeKind {
    Create,
    Modify,
    Delete,
}
```

## File Loading

Files can be loaded from:

1. **Filesystem** - Via `loader` module
2. **Memory** - Direct `VfsEntry` creation
3. **Virtual paths** - For generated content

```rust
// From filesystem
vfs.set_file_contents(file_id, std::fs::read(path)?.into());

// From memory
vfs.add_virt_file("/virtual/test.va", "module test; endmodule".into());
```

## Encoding Detection

The VFS automatically detects text encoding:

```rust
impl From<Vec<u8>> for VfsEntry {
    fn from(contents: Vec<u8>) -> Self {
        // Uses chardetng for encoding detection
        // Converts to UTF-8
        // Reports malformed characters
    }
}
```

## Error Handling

```rust
pub enum FileReadError {
    Io(io::ErrorKind),
    InvalidTextFormat(InvalidTextFormatErr),
}

pub struct InvalidTextFormatErr {
    pub pos: Arc<[Range<usize>]>,  // Positions of invalid characters
}
```

## Standard Library

The `va_std` module provides standard Verilog-A include files:

```rust
pub mod va_std {
    // Standard discipline definitions, etc.
}
```

## Anchored Paths

For relative path resolution:

```rust
pub struct AnchoredPath<'a> {
    pub anchor: FileId,
    pub path: &'a str,
}

pub struct AnchoredPathBuf {
    pub anchor: FileId,
    pub path: String,
}
```

## Integration with Salsa

The VFS integrates with salsa for incremental computation:

1. Files are loaded into VFS
2. Changes are tracked via `ChangedFile`
3. Changes are pushed to salsa database
4. Salsa invalidates dependent queries

## Dependencies

- `paths` - Path handling utilities
- `encoding_rs` - Character encoding
- `chardetng` - Encoding detection
