use super::*;
use crate::grammar::stmts::{STMT_RECOVER, STMT_TS};

const MODULE_ITEM_RECOVERY: TokenSet = DIRECTION_TS.union(TokenSet::new(&[
    NET_TYPE,
    ANALOG_KW,
    INITIAL_KW,
    BRANCH_KW,
    STRING_KW,
    REAL_KW,
    INTEGER_KW,
    PARAMETER_KW,
    LOCALPARAM_KW,
    ENDMODULE_KW,
    EOF,
]));
pub(super) const MODULE_ITEM_OR_ATTR_RECOVERY: TokenSet =
    MODULE_ITEM_RECOVERY.union(TokenSet::unique(T!["(*"]));

pub(crate) fn module(p: &mut Parser, m: Marker) {
    p.bump(T![module]);
    name_r(p, TokenSet::new(&[T!['('], T![;]]));
    if p.at(T!['(']) {
        let m = p.start();
        p.bump(T!['(']);
        module_ports(p);
        m.complete(p, MODULE_PORTS);
    }
    p.expect(T![;]);
    module_items(p);

    p.expect(ENDMODULE_KW);

    m.complete(p, MODULE_DECL);
}

const MODULE_PORTS_RECOVERY: TokenSet = TokenSet::new(&[T![;], T![')'], ENDMODULE_KW, EOF]);

fn module_ports(p: &mut Parser) {
    while !p.at_ts(MODULE_PORTS_RECOVERY) {
        let m = p.start();
        if !eat_name(p) {
            let m = p.start();
            attrs(p, MODULE_PORTS_RECOVERY.union(DIRECTION_TS));
            port_decl::<true>(p, m)
        }
        m.complete(p, MODULE_PORT);
        if !p.at(T![')']) {
            p.expect_with(T![,], &[T![,], T![')']]);
        }
    }
    p.expect(T![')']);
}

pub(super) fn alias_parameter_decl(p: &mut Parser, m: Marker) {
    p.bump(ALIASPARAM_KW);
    name_r(p, TokenSet::new(&[T![;], T![=]]));
    p.expect(T![=]);
    if p.at(SYSFUN) {
        let m = p.start();
        p.bump_any();
        m.complete(p, SYS_FUN);
    } else {
        path(p);
    }
    p.eat(T![;]);
    m.complete(p, ALIAS_PARAM);
}

const DIRECTION_TS: TokenSet = TokenSet::new(&[T![inout], T![output], T![input]]);
const MODULE_PORT_RECOVERY: TokenSet =
    MODULE_PORTS_RECOVERY.union(DIRECTION_TS).union(TokenSet::unique(T!["(*"]));
const NET_RECOVERY: TokenSet = TokenSet::new(&[EOF, ENDMODULE_KW, T![;]]);

fn port_decl<const MODULE_HEAD: bool>(p: &mut Parser, m: Marker) {
    let direction = p.start();
    p.bump_ts(DIRECTION_TS);
    direction.complete(p, DIRECTION);

    //direction and type are both optional since only one is required
    if !p.nth_at_ts(1, MODULE_PORT_RECOVERY.union(TokenSet::unique(T![,]))) {
        eat_name_ref(p);
    }
    p.eat(NET_TYPE);

    if MODULE_HEAD {
        decl_list(p, T![')'], module_port, MODULE_PORT_RECOVERY);
    } else {
        net_dec_list(p);
    }

    let finished = m.complete(p, PORT_DECL);
    if !MODULE_HEAD {
        let m = finished.precede(p);
        p.eat(T![;]);
        m.complete(p, BODY_PORT_DECL);
    }
}

fn module_port(p: &mut Parser) -> bool {
    name_r(p, MODULE_PORT_RECOVERY.union(TokenSet::unique(T![,])));
    !(p.at(T![,]) && p.nth_at_ts(1, MODULE_PORT_RECOVERY))
}

fn module_items(p: &mut Parser) {
    let mut error_range: Option<CompletedMarker> = None;
    while !p.at_ts(ITEM_RECOVERY_SET.union(TokenSet::unique(ENDMODULE_KW))) {
        let m = p.start();
        attrs(p, MODULE_ITEM_RECOVERY);

        match p.current() {
            ANALOG_KW if p.nth(1) == FUNCTION_KW => func_decl(p, m),
            ANALOG_KW => {
                p.bump(ANALOG_KW);
                p.eat(INITIAL_KW);
                stmt_with_attrs(p);
                m.complete(p, ANALOG_BEHAVIOUR);
            }
            NET_TYPE => {
                net_decl::<true>(p, m);
            }
            IDENT => {
                // Distinguish between net declaration and module instantiation:
                // Net declaration: `discipline name, name2;` - IDENT followed by IDENT, comma, or semicolon
                // Module instantiation: `module_name #(params) inst_name (ports);` or `module_name inst_name (ports);`
                // Key distinction: module inst has IDENT followed by #( or IDENT followed by IDENT then (
                if p.nth(1) == POUND || (p.nth(1) == IDENT && p.nth(2) == L_PAREN) {
                    module_inst(p, m);
                } else {
                    net_decl::<false>(p, m);
                }
            }
            PARAMETER_KW | LOCALPARAM_KW => {
                parameter_decl(p, m);
            }
            ALIASPARAM_KW => {
                alias_parameter_decl(p, m);
            }
            BRANCH_KW => {
                branch_decl(p, m);
            }
            INTEGER_KW | REAL_KW | STRING_KW => var_decl(p, m),
            INPUT_KW | OUTPUT_KW | INOUT_KW => port_decl::<false>(p, m),
            _ => {
                error_range = if let Some(error_range) = error_range {
                    m.abandon(p);
                    p.bump_any();
                    while !p.at_ts(MODULE_ITEM_RECOVERY) {
                        p.bump_any();
                    }
                    Some(error_range.undo_completion(p).complete(p, ERROR))
                } else {
                    let err = p.unexpected_tokens_msg(vec![
                        FUNCTION,
                        PORT_DECL,
                        NET_DECL,
                        ANALOG_BEHAVIOUR,
                    ]);
                    p.error(err);
                    p.bump_any();
                    while !p.at_ts(MODULE_ITEM_RECOVERY) {
                        p.bump_any();
                    }
                    Some(m.complete(p, ERROR))
                }
            }
        }
    }
}

fn net_decl<const NET_TYPE_FIRST: bool>(p: &mut Parser, m: Marker) {
    //direction and type ar both optional since only one is required
    if NET_TYPE_FIRST {
        p.bump(NET_TYPE);
        if !p.nth_at_ts(1, TokenSet::new(&[T![,], T![;]])) {
            eat_name_ref(p);
        }
    } else {
        name_ref_r(p, MODULE_ITEM_OR_ATTR_RECOVERY.union(TokenSet::unique(T![;])))
    }

    net_dec_list(p);
    p.eat(T![;]);
    m.complete(p, NET_DECL);
}

fn net_dec_list(p: &mut Parser) {
    decl_list(p, T![;], decl_name, NET_RECOVERY);
}

const FUNCTION_RECOVER: TokenSet = TokenSet::new(&[EOF, ENDMODULE_KW, ENDFUNCTION_KW]);
const FUN_ITEM_TS: TokenSet = TokenSet::new(&[PARAMETER_KW, LOCALPARAM_KW])
    .union(TYPE_TS)
    .union(STMT_RECOVER)
    .union(DIRECTION_TS)
    .union(STMT_TS);

fn func_decl(p: &mut Parser, m: Marker) {
    p.bump(T![analog]);
    p.bump(T![function]);
    eat_ty(p);
    name_r(p, TokenSet::unique(T![;]));
    p.expect(T![;]);

    while !p.at_ts(FUNCTION_RECOVER) {
        let m = p.start();
        attrs(p, FUN_ITEM_TS.union(FUNCTION_RECOVER));
        if p.at_ts(TYPE_TS) {
            var_decl(p, m)
        } else if p.at_ts(TokenSet::new(&[PARAMETER_KW, LOCALPARAM_KW])) {
            parameter_decl(p, m)
        } else if p.at_ts(DIRECTION_TS) {
            func_arg(p, m);
        } else {
            stmt(p, m, FUN_ITEM_TS, FUNCTION_RECOVER)
        }
    }
    p.expect(ENDFUNCTION_KW);
    m.complete(p, FUNCTION);
}

const FUNC_ARG_RECOVER: TokenSet = TokenSet::new(&[EOF, ENDMODULE_KW]);
fn func_arg(p: &mut Parser, m: Marker) {
    let direction = p.start();
    p.bump_ts(DIRECTION_TS);
    direction.complete(p, DIRECTION);

    decl_list(p, T![;], decl_name, FUNC_ARG_RECOVER);
    p.eat(T![;]);
    m.complete(p, FUNCTION_ARG);
}

fn branch_decl(p: &mut Parser, m: Marker) {
    p.bump(BRANCH_KW);
    if !p.at(T!['(']) {
        p.error(p.unexpected_token_msg(T!['(']));
    }
    arg_list(p);
    decl_list(p, T![;], decl_name, MODULE_ITEM_OR_ATTR_RECOVERY);
    p.eat(T![;]);
    m.complete(p, BRANCH_DECL);
}

/// Parse module instantiation: `module_name #(.param(value), ...) instance_name (port1, port2, ...);`
/// Example: `resistor #(.r(rwire)) r1 (d, de0);`
fn module_inst(p: &mut Parser, m: Marker) {
    // Module name (the type being instantiated)
    name_ref_r(p, TokenSet::new(&[POUND, IDENT]));

    // Optional parameter assignments: #(.param(value), ...)
    if p.at(POUND) {
        param_assignments(p);
    }

    // Instance name
    name_r(p, TokenSet::new(&[T!['('], T![;]]));

    // Port connections: (port1, port2, ...) or (.port(connection), ...)
    if p.at(T!['(']) {
        port_connections(p);
    } else {
        p.error(p.unexpected_token_msg(T!['(']));
    }

    p.eat(T![;]);
    m.complete(p, MODULE_INST);
}

/// Parse parameter assignments: #(.param(value), ...)
fn param_assignments(p: &mut Parser) {
    let m = p.start();
    p.bump(POUND);
    p.expect(T!['(']);

    if !p.at(T![')']) {
        param_assignment(p);
        while p.eat(T![,]) {
            param_assignment(p);
        }
    }

    p.expect(T![')']);
    m.complete(p, PARAM_ASSIGNMENTS);
}

/// Parse a single parameter assignment: .param(value)
fn param_assignment(p: &mut Parser) {
    let m = p.start();
    p.expect(T![.]);
    name_r(p, TokenSet::new(&[T!['(']]));
    p.expect(T!['(']);
    expr(p);
    p.expect(T![')']);
    m.complete(p, PARAM_ASSIGNMENT);
}

/// Parse port connections: (expr, expr, ...) or (.port(expr), ...)
fn port_connections(p: &mut Parser) {
    let m = p.start();
    p.bump(T!['(']);

    if !p.at(T![')']) {
        port_connection(p);
        while p.eat(T![,]) {
            port_connection(p);
        }
    }

    p.expect(T![')']);
    m.complete(p, PORT_CONNECTIONS);
}

/// Parse a single port connection: either positional (expr) or named (.port(expr))
fn port_connection(p: &mut Parser) {
    let m = p.start();
    if p.at(T![.]) {
        // Named port connection: .port(expr)
        p.bump(T![.]);
        name_r(p, TokenSet::new(&[T!['(']]));
        p.expect(T!['(']);
        expr(p);
        p.expect(T![')']);
    } else {
        // Positional port connection: just an expression (usually a name)
        expr(p);
    }
    m.complete(p, PORT_CONNECTION);
}
