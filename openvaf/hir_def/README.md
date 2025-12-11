# hir_def

HIR definitions and name resolution for the OpenVAF Verilog-A compiler.

## Overview

The `hir_def` crate is responsible for lowering the concrete syntax tree (CST) into the High-Level Intermediate Representation (HIR). It handles:

- Desugaring syntax constructs into simpler forms
- Name resolution and scope management
- Building definition maps for items
- Creating typed indices for HIR nodes

## Key Components

### Item Definitions

The crate defines HIR nodes for all Verilog-A constructs:

- **ModuleId** - Module definitions
- **FunctionId** - Analog function definitions
- **VarId** - Variable declarations
- **ParamId** - Parameter declarations
- **NodeId** - Node (port/internal) declarations
- **BranchId** - Named branch declarations
- **NatureId** - Nature definitions
- **DisciplineId** - Discipline definitions
- **AliasParamId** - Parameter aliases

### ItemTree

The `ItemTree` is a condensed representation of all items in a file:

```rust
pub struct ItemTree {
    // Maps from item IDs to their definitions
    modules: Arena<Module>,
    functions: Arena<Function>,
    variables: Arena<Variable>,
    // ... etc
}
```

### Body

The `Body` structure represents the lowered form of expressions and statements:

```rust
pub struct Body {
    pub exprs: Arena<Expr>,
    pub stmts: Arena<Stmt>,
    pub entry_stmts: Box<[StmtId]>,
}
```

### Scopes and Name Resolution

The crate manages:

- **ScopeId** - Identifies a lexical scope
- **DefMap** - Maps names to their definitions within a scope
- **Resolver** - Performs name lookup across scopes

### Expression Types

HIR expressions after lowering:

- `Expr::Literal` - Numeric/string literals
- `Expr::Path` - Name references
- `Expr::BinaryOp` - Binary operations
- `Expr::UnaryOp` - Unary operations
- `Expr::Call` - Function/system function calls
- `Expr::Select` - Conditional expressions
- `Expr::Array` - Array expressions

### Statement Types

HIR statements:

- `Stmt::Expr` - Expression statements
- `Stmt::Assignment` - Variable assignments
- `Stmt::Contribute` - Branch contributions (`<+`)
- `Stmt::If` - Conditional statements
- `Stmt::Case` - Case statements
- `Stmt::For/While` - Loop statements
- `Stmt::Block` - Statement blocks

## Database Queries

The crate provides salsa queries via `HirDefDB`:

```rust
#[salsa::query_group(HirDefDBStorage)]
pub trait HirDefDB: BaseDB {
    fn item_tree(&self, file: FileId) -> Arc<ItemTree>;
    fn body(&self, owner: DefWithBodyId) -> Arc<Body>;
    fn scope_graph(&self, root: ScopeDefItem) -> Arc<ScopeGraph>;
    // ... more queries
}
```

## Dependencies

- `basedb` - Base database with VFS and parsing
- `syntax` - AST types for lowering
- `arena` - Arena allocator for HIR nodes
- `salsa` - Incremental computation framework
