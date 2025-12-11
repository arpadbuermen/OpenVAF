use std::mem;

use basedb::lints::LintRegistry;
use basedb::{AstIdMap, ErasedAstId, LintAttrs};
use syntax::ast::{self, ArgListOwner, AssignOp, AttrIter, AttrsOwner, BinaryOp, FunctionRef};
use syntax::name::{AsName, Name};
use syntax::{AstNode, AstPtr};

// use tracing::debug;
use super::{Body, BodySourceMap};
use crate::builtin::BuiltInPrimitive;
use crate::db::HirDefDB;
use crate::expr::{CaseCond, Event, GlobalEvent};
use crate::item_tree::{ItemTree, Module, ModuleInstItem, ModuleItem};
use crate::nameres::DefMapSource;
use crate::{BlockLoc, Case, Expr, ExprId, Intern, Literal, Path, ScopeId, Stmt, StmtId};

pub(super) struct LowerCtx<'a> {
    pub(super) db: &'a dyn HirDefDB,
    pub(super) body: &'a mut Body,
    pub(super) source_map: &'a mut BodySourceMap,
    pub(super) ast_id_map: &'a AstIdMap,
    pub(super) curr_scope: (ScopeId, ErasedAstId),
    pub(super) registry: &'a LintRegistry,
}

impl LowerCtx<'_> {
    pub fn collect_opt_expr(&mut self, expr: Option<ast::Expr>) -> ExprId {
        if let Some(expr) = expr {
            self.collect_expr(expr)
        } else {
            self.missing_expr()
        }
    }

    pub fn collect_expr(&mut self, expr: ast::Expr) -> ExprId {
        let e = match &expr {
            ast::Expr::PrefixExpr(e) => {
                let expr = self.collect_opt_expr(e.expr());
                if let Some(op) = e.op_kind() {
                    Expr::UnaryOp { expr, op }
                } else {
                    Expr::Missing
                }
            }

            ast::Expr::BinExpr(e) => {
                let lhs = self.collect_opt_expr(e.lhs());
                let rhs = self.collect_opt_expr(e.rhs());
                Expr::BinaryOp { lhs, rhs, op: e.op_kind() }
            }

            ast::Expr::ParenExpr(e) => return self.collect_opt_expr(e.expr()),

            ast::Expr::ArrayExpr(e) => {
                let vals = e.exprs().map(|expr| self.collect_expr(expr)).collect();
                Expr::Array(vals)
            }

            ast::Expr::Call(call) => {
                let fun = call.function_ref().and_then(|fun| match fun {
                    FunctionRef::Path(path) => Path::resolve(path),
                    FunctionRef::SysFun(fun) => Some(Path::new_ident(fun.as_name())),
                });

                let args = if let Some(args) = call.arg_list().map(|list| list.args()) {
                    args.map(|arg| self.collect_expr(arg)).collect()
                } else {
                    vec![]
                };

                Expr::Call { fun, args }
            }

            ast::Expr::SelectExpr(e) => {
                let cond = self.collect_opt_expr(e.condition());
                let then_val = self.collect_opt_expr(e.then_val());
                let else_val = self.collect_opt_expr(e.else_val());
                Expr::Select { cond, then_val, else_val }
            }

            // TODO refactor with if let binding and default case is missing expression
            // BLOCK
            ast::Expr::PathExpr(path) => {
                if let Some(path) = path.path().and_then(Path::resolve) {
                    Expr::Path { path, port: false }
                } else {
                    return self.missing_expr();
                }
            }

            ast::Expr::PortFlow(port_flow) => {
                if let Some(path) = port_flow.port().and_then(Path::resolve) {
                    Expr::Path { path, port: true }
                } else {
                    return self.missing_expr();
                }
            }

            ast::Expr::Literal(lit) => Expr::Literal(Literal::new(lit.kind())),
        };
        self.alloc_expr(e, AstPtr::new(&expr))
    }

    pub fn collect_opt_stmt(&mut self, stmt: Option<ast::Stmt>) -> StmtId {
        match stmt {
            Some(stmt) => self.collect_stmt(stmt),
            None => self.missing_stmt(),
        }
    }

    pub fn collect_stmt(&mut self, stmt: ast::Stmt) -> StmtId {
        let s = match &stmt {
            ast::Stmt::EmptyStmt(_) => Stmt::Empty,
            ast::Stmt::AssignStmt(stmt) => match stmt.assign() {
                Some(a) => Stmt::Assignment {
                    dst: self.collect_opt_expr(a.lval()),
                    val: self.collect_opt_expr(a.rval()),
                    assignment_kind: a.op().unwrap(),
                },
                None => {
                    // debug!(
                    //     tree = debug(stmt),
                    //     src = display(stmt),
                    //     "Assign Statement without assign?"
                    // );
                    Stmt::Missing
                }
            },
            ast::Stmt::ExprStmt(stmt) => Stmt::Expr(self.collect_opt_expr(stmt.expr())),
            ast::Stmt::IfStmt(stmt) => {
                let cond = self.collect_opt_expr(stmt.condition());
                let then_branch = self.collect_opt_stmt(stmt.then_branch());
                let else_branch = self.collect_opt_stmt(stmt.else_branch());
                Stmt::If { cond, then_branch, else_branch }
            }
            ast::Stmt::WhileStmt(stmt) => {
                let cond = self.collect_opt_expr(stmt.condition());
                let body = self.collect_opt_stmt(stmt.body());
                Stmt::WhileLoop { cond, body }
            }
            ast::Stmt::ForStmt(stmt) => {
                let cond = self.collect_opt_expr(stmt.condition());
                let init = self.collect_opt_stmt(stmt.init());
                let incr = self.collect_opt_stmt(stmt.incr());
                let body = self.collect_opt_stmt(stmt.for_body());
                Stmt::ForLoop { init, cond, incr, body }
            }
            ast::Stmt::CaseStmt(stmt) => self.collect_case_stmt(stmt),
            ast::Stmt::EventStmt(stmt) => return self.collect_event_stmt(stmt),
            ast::Stmt::BlockStmt(stmt) => self.collect_block(stmt),
        };
        self.alloc_stmt(s, AstPtr::new(&stmt), stmt.attrs())
    }

    fn collect_event_stmt(&mut self, event_stmt: &ast::EventStmt) -> StmtId {
        let kind = if event_stmt.initial_step_token().is_some() {
            GlobalEvent::InitialStep
        } else if event_stmt.final_step_token().is_some() {
            GlobalEvent::FinalStep
        } else {
            return self.collect_opt_stmt(event_stmt.stmt());
        };

        let phases = event_stmt.sim_phases().map(|lit| lit.unescaped_value()).collect();
        let event = Event::Global { kind, phases };
        let stmt = Stmt::EventControl { event, body: self.collect_opt_stmt(event_stmt.stmt()) };

        self.alloc_stmt(stmt, AstPtr::new(event_stmt).cast().unwrap(), event_stmt.attrs())
    }

    fn collect_case_stmt(&mut self, case_stmt: &ast::CaseStmt) -> Stmt {
        let discr = self.collect_opt_expr(case_stmt.discriminant());
        let case_arms = case_stmt
            .cases()
            .map(|case| {
                let cond = if case.default_token().is_some() {
                    debug_assert_eq!(case.exprs().next(), None);
                    CaseCond::Default
                } else {
                    let vals = case.exprs().map(|e| self.collect_expr(e)).collect();
                    CaseCond::Vals(vals)
                };
                Case { cond, body: self.collect_opt_stmt(case.stmt()) }
            })
            .collect();

        Stmt::Case { discr, case_arms }
    }

    pub fn collect_block(&mut self, block: &ast::BlockStmt) -> Stmt {
        let ast = self.ast_id_map.ast_id(block);
        let id = BlockLoc { ast, parent: self.curr_scope.0 }.intern(self.db);
        let scope = self.db.block_def_map(id);

        let parent_scope = match scope {
            Some(def_map) => {
                let scope = ScopeId {
                    root_file: self.curr_scope.0.root_file,
                    local_scope: def_map.entry(),
                    src: DefMapSource::Block(id),
                };

                mem::replace(&mut self.curr_scope, (scope, ast.into()))
            }
            None => {
                let scope = self.curr_scope.0;
                mem::replace(&mut self.curr_scope, (scope, ast.into()))
            }
        };

        let body = block.body().map(|stmt| self.collect_stmt(stmt)).collect();

        self.curr_scope = parent_scope;
        Stmt::Block { body }
    }

    fn alloc_expr(&mut self, expr: Expr, ptr: AstPtr<ast::Expr>) -> ExprId {
        let id = self.make_expr(expr, Some(ptr.clone()));
        self.source_map.expr_map.insert(ptr, id);
        id
    }
    // desugared exprs don't have ptr, that's wrong and should be fixed
    // somehow.
    pub(super) fn alloc_expr_desugared(&mut self, expr: Expr) -> ExprId {
        self.make_expr(expr, None)
    }

    fn missing_expr(&mut self) -> ExprId {
        self.alloc_expr_desugared(Expr::Missing)
    }

    fn make_expr(&mut self, expr: Expr, src: Option<AstPtr<ast::Expr>>) -> ExprId {
        let id = self.body.exprs.push_and_get_key(expr);
        self.source_map.expr_map_back.insert(id, src);
        id
    }

    fn alloc_stmt(&mut self, stmt: Stmt, ptr: AstPtr<ast::Stmt>, attrs: AttrIter) -> StmtId {
        let attrs = LintAttrs::resolve(
            self.registry,
            attrs,
            &mut self.source_map.diagnostics,
            self.curr_scope.1,
        );
        let id = self.make_stmt(stmt, Some(ptr.clone()), attrs);
        self.source_map.stmt_map.insert(ptr, id);

        id
    }

    // desugared stmts don't have ptr, that's wrong and should be fixed
    // somehow.
    pub(super) fn alloc_stmt_desugared(&mut self, stmt: Stmt) -> StmtId {
        self.make_stmt(stmt, None, LintAttrs::empty(self.curr_scope.1))
    }

    pub(super) fn missing_stmt(&mut self) -> StmtId {
        self.alloc_stmt_desugared(Stmt::Missing)
    }

    fn make_stmt(
        &mut self,
        stmt: Stmt,
        src: Option<AstPtr<ast::Stmt>>,
        attrs: LintAttrs,
    ) -> StmtId {
        let id = self.body.stmts.push_and_get_key(stmt);
        let id2 = self.body.stmt_scopes.push_and_get_key(self.curr_scope.0);
        let id3 = self.source_map.lint_map.push_and_get_key(attrs);
        debug_assert_eq!(id, id2);
        debug_assert_eq!(id2, id3);
        self.source_map.stmt_map_back.insert(id, src);
        id
    }

    // ============================================================================
    // Primitive module instance lowering
    // ============================================================================

    /// Lower primitive module instances to synthetic contribution statements.
    /// Called during module body lowering to transform instances like:
    ///   resistor #(.r(R)) r1 (a, b)  ->  I(a,b) <+ V(a,b) / R
    pub fn lower_primitive_instances(&mut self, module: &Module, tree: &ItemTree) -> Vec<StmtId> {
        let mut stmts = Vec::new();

        for item in &module.items {
            if let ModuleItem::ModuleInst(inst_id) = item {
                let inst = &tree.data.module_insts[*inst_id];

                if let Some(primitive) = BuiltInPrimitive::from_name(&inst.module_name) {
                    if let Some(stmt) = self.lower_primitive_instance(primitive, inst) {
                        stmts.push(stmt);
                    }
                }
            }
        }
        stmts
    }

    /// Lower a single primitive instance to a contribution statement
    fn lower_primitive_instance(
        &mut self,
        primitive: BuiltInPrimitive,
        inst: &ModuleInstItem,
    ) -> Option<StmtId> {
        // Validate: must have exactly 2 ports
        if inst.port_connections.len() != 2 {
            return None;
        }

        // Get parameter value expression AstPtr
        let param_name = primitive.param_name();
        let param_expr_ptr = inst
            .param_assignments
            .iter()
            .find(|(name, _)| &**name == param_name)
            .map(|(_, expr_ptr)| expr_ptr.clone())?;

        // Get the syntax tree to resolve the AstPtr
        let root_file = self.curr_scope.0.root_file;
        let syntax_tree = self.db.parse(root_file).tree();

        // Collect the parameter expression from AST
        let param_ast = param_expr_ptr.to_node(syntax_tree.syntax());
        let param_expr = self.collect_expr(param_ast);

        let hi = &inst.port_connections[0];
        let lo = &inst.port_connections[1];

        match primitive {
            BuiltInPrimitive::Resistor => self.create_resistor_contribution(hi, lo, param_expr),
            BuiltInPrimitive::Capacitor => self.create_capacitor_contribution(hi, lo, param_expr),
            BuiltInPrimitive::Inductor => self.create_inductor_contribution(hi, lo, param_expr),
        }
    }

    /// Create: I(hi, lo) <+ V(hi, lo) / R
    fn create_resistor_contribution(
        &mut self,
        hi: &Name,
        lo: &Name,
        r_expr: ExprId,
    ) -> Option<StmtId> {
        // Create V(hi, lo) branch access
        let v_access = self.create_branch_access(Name::new_inline("V"), hi, lo);

        // Create V(hi,lo) / R
        let div_expr = self.alloc_expr_desugared(Expr::BinaryOp {
            lhs: v_access,
            rhs: r_expr,
            op: Some(BinaryOp::Division),
        });

        // Create I(hi, lo) branch access
        let i_access = self.create_branch_access(Name::new_inline("I"), hi, lo);

        // Create contribution statement: I(hi,lo) <+ V(hi,lo) / R
        Some(self.alloc_stmt_desugared(Stmt::Assignment {
            dst: i_access,
            val: div_expr,
            assignment_kind: AssignOp::Contribute,
        }))
    }

    /// Create: I(hi, lo) <+ ddt(C * V(hi, lo))
    fn create_capacitor_contribution(
        &mut self,
        hi: &Name,
        lo: &Name,
        c_expr: ExprId,
    ) -> Option<StmtId> {
        // Create V(hi, lo) branch access
        let v_access = self.create_branch_access(Name::new_inline("V"), hi, lo);

        // Create C * V(hi,lo)
        let mul_expr = self.alloc_expr_desugared(Expr::BinaryOp {
            lhs: c_expr,
            rhs: v_access,
            op: Some(BinaryOp::Multiplication),
        });

        // Create ddt(C * V(hi,lo))
        let ddt_expr = self.create_ddt_call(mul_expr);

        // Create I(hi, lo) branch access
        let i_access = self.create_branch_access(Name::new_inline("I"), hi, lo);

        // Create contribution statement: I(hi,lo) <+ ddt(C * V(hi,lo))
        Some(self.alloc_stmt_desugared(Stmt::Assignment {
            dst: i_access,
            val: ddt_expr,
            assignment_kind: AssignOp::Contribute,
        }))
    }

    /// Create: V(hi, lo) <+ ddt(L * I(hi, lo))
    fn create_inductor_contribution(
        &mut self,
        hi: &Name,
        lo: &Name,
        l_expr: ExprId,
    ) -> Option<StmtId> {
        // Create I(hi, lo) branch access
        let i_access = self.create_branch_access(Name::new_inline("I"), hi, lo);

        // Create L * I(hi,lo)
        let mul_expr = self.alloc_expr_desugared(Expr::BinaryOp {
            lhs: l_expr,
            rhs: i_access,
            op: Some(BinaryOp::Multiplication),
        });

        // Create ddt(L * I(hi,lo))
        let ddt_expr = self.create_ddt_call(mul_expr);

        // Create V(hi, lo) branch access
        let v_access = self.create_branch_access(Name::new_inline("V"), hi, lo);

        // Create contribution statement: V(hi,lo) <+ ddt(L * I(hi,lo))
        Some(self.alloc_stmt_desugared(Stmt::Assignment {
            dst: v_access,
            val: ddt_expr,
            assignment_kind: AssignOp::Contribute,
        }))
    }

    /// Create a branch access expression like V(hi, lo) or I(hi, lo)
    fn create_branch_access(&mut self, nature: Name, hi: &Name, lo: &Name) -> ExprId {
        // Create path expressions for the nodes
        let hi_path = self
            .alloc_expr_desugared(Expr::Path { path: Path::new_ident(hi.clone()), port: false });
        let lo_path = self
            .alloc_expr_desugared(Expr::Path { path: Path::new_ident(lo.clone()), port: false });

        // Create the function call: V(hi, lo) or I(hi, lo)
        self.alloc_expr_desugared(Expr::Call {
            fun: Some(Path::new_ident(nature)),
            args: vec![hi_path, lo_path],
        })
    }

    /// Create a ddt() function call
    fn create_ddt_call(&mut self, arg: ExprId) -> ExprId {
        self.alloc_expr_desugared(Expr::Call {
            fun: Some(Path::new_ident(Name::new_inline("ddt"))),
            args: vec![arg],
        })
    }
}

impl Literal {
    pub fn new(ast: ast::LiteralKind) -> Literal {
        match ast {
            ast::LiteralKind::String(lit) => {
                Literal::String(lit.unescaped_value().into_boxed_str())
            }
            ast::LiteralKind::IntNumber(lit) => Literal::Int(lit.value()),
            ast::LiteralKind::SiRealNumber(lit) => Literal::Float(lit.value().into()),
            ast::LiteralKind::StdRealNumber(lit) => Literal::Float(lit.value().into()),
            ast::LiteralKind::Inf => {
                // TODO check that this allowed somewhere?
                Literal::Inf
            }
        }
    }
}
