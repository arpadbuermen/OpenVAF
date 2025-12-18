# syntax

Verilog-A syntax tree and AST definitions for the OpenVAF compiler.

## Overview

The `syntax` crate provides the concrete syntax tree (CST) representation and typed AST wrappers for Verilog-A source code. It's built on the `rowan` library for lossless syntax trees.

## Design

The crate uses a two-layer design:

1. **Untyped CST** - `SyntaxNode` and `SyntaxToken` from rowan
2. **Typed AST** - Wrapper types that provide convenient access

## Key Components

### Parse Result

```rust
pub struct Parse<T> {
    green: GreenNode,           // Immutable green tree
    errors: Arc<Vec<SyntaxError>>,
    ctx_map: Arc<Vec<(TextRange, SourceContext, TextSize)>>,
}

impl<T: AstNode> Parse<T> {
    pub fn tree(&self) -> T;
    pub fn errors(&self) -> &[SyntaxError];
    pub fn syntax_node(&self) -> SyntaxNode;
}
```

### SourceFile

The root of a parsed Verilog-A file:

```rust
impl SourceFile {
    pub fn parse(
        db: &dyn SourceProvider,
        root_file: FileId,
        preprocess: &Preprocess,
    ) -> Parse<SourceFile>;
}
```

### AST Nodes

Typed wrappers for syntax nodes:

```rust
pub trait AstNode: Clone {
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

// Example AST nodes
pub struct Module { syntax: SyntaxNode }
pub struct PortDecl { syntax: SyntaxNode }
pub struct ParamDecl { syntax: SyntaxNode }
pub struct AnalogStmt { syntax: SyntaxNode }
```

### Source Mapping

Maps parsed positions back to original sources:

```rust
impl<T> Parse<T> {
    pub fn to_file_span(&self, range: TextRange, sm: &SourceMap) -> FileSpan;
    pub fn to_ctx_span(&self, range: TextRange, sm: &SourceMap) -> CtxSpan;
}
```

## AST Categories

### Declarations
- `Module` - Module definition
- `Nature` - Nature definition
- `Discipline` - Discipline definition
- `PortDecl` - Port declaration
- `ParamDecl` - Parameter declaration
- `VarDecl` - Variable declaration
- `BranchDecl` - Branch declaration

### Statements
- `AnalogStmt` - Analog block statement
- `BlockStmt` - Begin/end block
- `IfStmt` - Conditional statement
- `CaseStmt` - Case statement
- `ForStmt` - For loop
- `WhileStmt` - While loop
- `ContributeStmt` - Branch contribution

### Expressions
- `Literal` - Numeric/string literals
- `PathExpr` - Name reference
- `BinExpr` - Binary operation
- `UnaryExpr` - Unary operation
- `CallExpr` - Function call
- `IndexExpr` - Array subscript
- `SelectExpr` - Conditional expression

## Match Macro

Convenience macro for matching AST node types:

```rust
match_ast! {
    match node {
        ast::Module(m) => handle_module(m),
        ast::Function(f) => handle_function(f),
        _ => None,
    }
}
```

## Validation

Syntax validation beyond parser capabilities:

```rust
mod validation {
    pub fn validate(root: &SyntaxNode, errors: &mut Vec<SyntaxError>);
}
```

## Re-exports

The crate re-exports from underlying libraries:

```rust
pub use rowan::{
    Direction, GreenNode, NodeOrToken, SyntaxText,
    TextRange, TextSize, TokenAtOffset, WalkEvent,
};
pub use tokens::{SyntaxKind, T};
```

## Dependencies

- `rowan` - Lossless syntax tree library
- `parser` - Parse event generation
- `preprocessor` - Preprocessed token stream
- `tokens` - Token and syntax kind definitions
