use super::*;
use crate::diagnostic::Diagnostic;
use crate::syntax::{ast, token::Span};
use std::collections::HashMap;

type Result<T> = std::result::Result<T, Diagnostic>;

#[derive(Clone)]
struct Signature {
    target: CallTarget,
    params: Vec<(String, Type)>,
    result: Type,
}
#[derive(Clone, Copy)]
struct Local {
    id: usize,
    ty: Type,
    mutable: bool,
}

// Possible exits from a statement. Sequential composition only feeds reachable
// fallthrough into the next statement; dead code is still typechecked.
#[derive(Clone, Copy)]
struct Flow(u8);
impl Flow {
    const NEXT: Self = Self(1);
    const RETURN: Self = Self(2);
    const BREAK: Self = Self(4);
    const CONTINUE: Self = Self(8);
    fn has(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    fn then(self, next: Self) -> Self {
        Self((self.0 & !1) | if self.has(Self::NEXT) { next.0 } else { 0 })
    }
    fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

struct Checker {
    functions: HashMap<String, Signature>,
    scopes: Vec<HashMap<String, Local>>,
    next_local: usize,
    loops: usize,
    result: Type,
}

pub fn analyze(file: &ast::SourceFile) -> Result<Program> {
    let mut checker = Checker {
        functions: HashMap::new(),
        scopes: Vec::new(),
        next_local: 0,
        loops: 0,
        result: Type::Void,
    };
    for (id, function) in crate::runtime::FUNCTIONS.iter().enumerate() {
        checker.functions.insert(
            function.path.to_owned(),
            Signature {
                target: CallTarget::Runtime(id),
                params: function
                    .parameters
                    .iter()
                    .map(|(n, t)| ((*n).to_owned(), *t))
                    .collect(),
                result: function.result,
            },
        );
    }
    // Collect every signature before checking bodies: recursion and forward calls
    // use exactly the same lookup as calls to an earlier declaration.
    for (id, item) in file.items.iter().enumerate() {
        let ast::Item::Function(f) = item else {
            return Err(unsupported(item_span(item), "top-level declaration"));
        };
        if f.is_const || f.is_naked || f.is_static || f.is_inline || f.body.is_none() {
            return Err(unsupported(
                f.span,
                "function modifier or bodyless declaration",
            ));
        }
        let mut params = Vec::new();
        for p in &f.params {
            if p.default_val.is_some() || p.label.is_some() {
                return Err(unsupported(p.span, "default parameter or external label"));
            }
            let ty = resolve_type(&p.ty)?;
            if ty == Type::Void {
                return Err(error("E2003", p.span, "parameter cannot have type void"));
            }
            params.push((p.name.clone(), ty));
        }
        let result = f
            .return_type
            .as_ref()
            .map(resolve_type)
            .transpose()?
            .unwrap_or(Type::Void);
        if f.name == "main" && (!params.is_empty() || !matches!(result, Type::Void | Type::I32)) {
            return Err(error(
                "E2009",
                f.span,
                "main must have no parameters and return void or i32",
            ));
        }
        if checker
            .functions
            .insert(
                f.name.clone(),
                Signature {
                    target: CallTarget::User(id),
                    params,
                    result,
                },
            )
            .is_some()
        {
            return Err(error(
                "E2004",
                f.span,
                format!("duplicate declaration '{}'", f.name),
            ));
        }
    }
    let mut functions = Vec::new();
    for item in &file.items {
        let ast::Item::Function(f) = item else {
            unreachable!()
        };
        let sig = checker.functions[&f.name].clone();
        checker.result = sig.result;
        checker.scopes.push(HashMap::new());
        let mut params = Vec::new();
        for (p, (_, ty)) in f.params.iter().zip(&sig.params) {
            let local = checker.declare(&p.name, *ty, false, p.span)?;
            params.push((local.id, *ty));
        }
        let (body, flow) = match f.body.as_ref().unwrap() {
            ast::FunctionBody::Block(b) => checker.block(b, true, false)?,
            ast::FunctionBody::Expr(e) => {
                (vec![checker.return_value(Some(e), e.span())?], Flow::RETURN)
            }
        };
        if sig.result != Type::Void && flow.has(Flow::NEXT) {
            return Err(error(
                "E2009",
                f.span,
                format!(
                    "function '{}' may finish without returning {}",
                    f.name, sig.result
                ),
            ));
        }
        checker.scopes.pop();
        functions.push(Function {
            id: match sig.target {
                CallTarget::User(id) => id,
                CallTarget::Runtime(_) => unreachable!(),
            },
            name: f.name.clone(),
            params,
            result: sig.result,
            body,
        });
    }
    Ok(Program { functions })
}

impl Checker {
    fn lookup(&self, name: &str) -> Option<Local> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
    fn declare(&mut self, name: &str, ty: Type, mutable: bool, span: Span) -> Result<Local> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(name) {
            return Err(error(
                "E2004",
                span,
                format!("duplicate declaration '{name}'"),
            ));
        }
        let local = Local {
            id: self.next_local,
            ty,
            mutable,
        };
        self.next_local += 1;
        scope.insert(name.to_owned(), local);
        Ok(local)
    }
    fn block(
        &mut self,
        block: &ast::Block,
        function_body: bool,
        new_scope: bool,
    ) -> Result<(Vec<Statement>, Flow)> {
        if new_scope {
            self.scopes.push(HashMap::new());
        }
        let mut output = Vec::new();
        let mut flow = Flow::NEXT;
        for stmt in &block.stmts {
            let (stmt, exits) = self.statement(stmt)?;
            output.push(stmt);
            flow = flow.then(exits);
        }
        if let Some(expr) = &block.yield_expr {
            if !function_body {
                return Err(unsupported(
                    expr.span(),
                    "value-yielding nested block (use return in statement blocks)",
                ));
            }
            output.push(self.return_value(Some(expr), expr.span())?);
            flow = flow.then(Flow::RETURN);
        }
        if new_scope {
            self.scopes.pop();
        }
        Ok((output, flow))
    }
    fn statement(&mut self, stmt: &ast::Stmt) -> Result<(Statement, Flow)> {
        use ast::Stmt;
        let out = match stmt {
            Stmt::Let(v) | Stmt::Var(v) => {
                let expected = v.ty.as_ref().map(resolve_type).transpose()?;
                let value = self.expression(&v.value, expected)?;
                if value.ty == Type::Void {
                    return Err(error("E2003", v.span, "variable cannot have type void"));
                }
                let local = self.declare(&v.name, value.ty, v.is_mut, v.span)?;
                Statement::Declare(local.id, local.ty, value)
            }
            Stmt::Assign(target, op, value, span) => {
                let ast::Expr::Ident(name, _) = target else {
                    return Err(unsupported(*span, "assignment target"));
                };
                let local = self.lookup(name).ok_or_else(|| {
                    error(
                        "E2001",
                        target.span(),
                        format!("undefined identifier '{name}'"),
                    )
                })?;
                if !local.mutable {
                    return Err(error(
                        "E2007",
                        target.span(),
                        format!("cannot assign to immutable let/parameter '{name}'"),
                    ));
                }
                let rhs = self.expression(value, Some(local.ty))?;
                let value = match op {
                    ast::AssignOp::Assign => rhs,
                    _ => {
                        if !local.ty.is_numeric() {
                            return Err(error(
                                "E2007",
                                *span,
                                "compound assignment requires numbers",
                            ));
                        }
                        let op = match op {
                            ast::AssignOp::PlusAssign => ast::BinaryOp::Add,
                            ast::AssignOp::MinusAssign => ast::BinaryOp::Sub,
                            ast::AssignOp::StarAssign => ast::BinaryOp::Mul,
                            ast::AssignOp::SlashAssign => ast::BinaryOp::Div,
                            _ => unreachable!(),
                        };
                        Expression {
                            ty: local.ty,
                            kind: ExpressionKind::Binary(
                                Box::new(Expression {
                                    ty: local.ty,
                                    kind: ExpressionKind::Local(local.id),
                                }),
                                op,
                                Box::new(rhs),
                            ),
                        }
                    }
                };
                Statement::Assign(local.id, value)
            }
            Stmt::Expr(expr, _) => return self.control(expr),
            Stmt::Defer(_, span) | Stmt::Errdefer(_, span) => {
                return Err(unsupported(*span, "defer/errdefer"))
            }
        };
        Ok((out, Flow::NEXT))
    }
    fn control(&mut self, expr: &ast::Expr) -> Result<(Statement, Flow)> {
        use ast::Expr;
        match expr {
            Expr::Block(b) => {
                let (body, flow) = self.block(b, false, true)?;
                Ok((Statement::Block(body), flow))
            }
            Expr::If(cond, yes, no, _) => {
                let cond = self.expression(cond, Some(Type::Bool))?;
                let (yes, a) = self.block(yes, false, true)?;
                let (no, b) = match no {
                    Some(b) => self.block(b, false, true)?,
                    None => (Vec::new(), Flow::NEXT),
                };
                Ok((Statement::If(cond, yes, no), a.union(b)))
            }
            Expr::While(None, cond, body, _) => {
                let cond = self.expression(cond, Some(Type::Bool))?;
                self.loops += 1;
                let (body, exits) = self.block(body, false, true)?;
                self.loops -= 1;
                let endless = matches!(cond.kind, ExpressionKind::Bool(true));
                let flow =
                    Flow(exits.0 & Flow::RETURN.0).union(if !endless || exits.has(Flow::BREAK) {
                        Flow::NEXT
                    } else {
                        Flow(0)
                    });
                Ok((Statement::While(cond, body), flow))
            }
            Expr::Loop(None, body, _) => {
                self.loops += 1;
                let (body, exits) = self.block(body, false, true)?;
                self.loops -= 1;
                let flow = Flow(exits.0 & Flow::RETURN.0).union(if exits.has(Flow::BREAK) {
                    Flow::NEXT
                } else {
                    Flow(0)
                });
                Ok((Statement::Loop(body), flow))
            }
            Expr::Break(None, None, span) | Expr::Continue(None, span) => {
                if self.loops == 0 {
                    return Err(error("E2010", *span, "break/continue outside a loop"));
                }
                if matches!(expr, Expr::Break(..)) {
                    Ok((Statement::Break, Flow::BREAK))
                } else {
                    Ok((Statement::Continue, Flow::CONTINUE))
                }
            }
            Expr::Return(value, span) => {
                Ok((self.return_value(value.as_deref(), *span)?, Flow::RETURN))
            }
            _ => Ok((
                Statement::Evaluate(self.expression(expr, None)?),
                Flow::NEXT,
            )),
        }
    }
    fn return_value(&mut self, value: Option<&ast::Expr>, span: Span) -> Result<Statement> {
        let value = value
            .map(|e| self.expression(e, Some(self.result)))
            .transpose()
            .map_err(|mut e| {
                if e.code == "E2003" {
                    e.code = "E2009";
                    e.message = format!("return type mismatch: {}", e.message);
                }
                e
            })?;
        if value.is_none() && self.result != Type::Void {
            return Err(error(
                "E2009",
                span,
                format!("return requires a {} value", self.result),
            ));
        }
        Ok(Statement::Return(value))
    }
    // Non-literal hints make `1 + wide` and `wide + 1` symmetric without
    // implicitly converting typed values. Unsuffixed literals alone are flexible.
    fn hint(&self, expr: &ast::Expr) -> Option<Type> {
        use ast::Expr;
        match expr {
            Expr::Ident(n, _) => self.lookup(n).map(|l| l.ty),
            Expr::Int(_, Some(n), _) => Type::named(n),
            Expr::Cast(_, ty, _) => resolve_type(ty).ok(),
            Expr::BoolLit(..) | Expr::Unary(ast::UnaryOp::Not, _, _) => Some(Type::Bool),
            Expr::StringLit(..) => Some(Type::Str),
            Expr::Unary(_, v, _) => self.hint(v),
            Expr::Binary(a, op, b, _) => {
                use ast::BinaryOp::*;
                if matches!(
                    op,
                    Eq | NotEq | Lt | LtEq | Gt | GtEq | LogicalAnd | LogicalOr
                ) {
                    Some(Type::Bool)
                } else {
                    self.hint(a).or_else(|| self.hint(b))
                }
            }
            Expr::Call(callee, _, _) => {
                path(callee).and_then(|n| self.functions.get(&n).map(|f| f.result))
            }
            _ => None,
        }
    }
    fn expression(&mut self, expr: &ast::Expr, expected: Option<Type>) -> Result<Expression> {
        use ast::{BinaryOp as B, Expr as E, UnaryOp as U};
        let (ty, kind) = match expr {
            E::Int(n, suffix, span) => return self.integer(*n, suffix.as_deref(), *span, expected),
            E::Float(n, span) => {
                let ty = expected.filter(|t| t.is_float()).unwrap_or(Type::F64);
                if !n.is_finite() || (ty == Type::F32 && !(*n as f32).is_finite()) {
                    return Err(error("E2003", *span, "floating literal out of range"));
                }
                (ty, ExpressionKind::Float(*n))
            }
            E::BoolLit(b, _) => (Type::Bool, ExpressionKind::Bool(*b)),
            E::StringLit(s, _) => (Type::Str, ExpressionKind::String(s.clone())),
            E::Ident(name, span) => {
                let local = self.lookup(name).ok_or_else(|| {
                    error("E2001", *span, format!("undefined identifier '{name}'"))
                })?;
                (local.ty, ExpressionKind::Local(local.id))
            }
            E::Unary(U::Neg, v, span) if matches!(v.as_ref(), E::Int(..)) => {
                let E::Int(n, suffix, _) = v.as_ref() else {
                    unreachable!()
                };
                return self.integer(-n, suffix.as_deref(), *span, expected);
            }
            E::Unary(op, value, span) => {
                if matches!(op, U::AddressOf) {
                    return Err(unsupported(*span, "pointer expression"));
                }
                let value = self.expression(
                    value,
                    if *op == U::Not {
                        Some(Type::Bool)
                    } else {
                        expected
                    },
                )?;
                let valid = match op {
                    U::Not => value.ty == Type::Bool,
                    U::Neg => value.ty.is_signed() || value.ty.is_float(),
                    U::BitNot => value.ty.is_integer(),
                    _ => false,
                };
                if !valid {
                    return Err(error(
                        "E2007",
                        *span,
                        "invalid operand type for unary operator",
                    ));
                }
                (value.ty, ExpressionKind::Unary(*op, Box::new(value)))
            }
            E::Binary(a, op, b, span) => {
                let compare = matches!(op, B::Eq | B::NotEq | B::Lt | B::LtEq | B::Gt | B::GtEq);
                let logical = matches!(op, B::LogicalAnd | B::LogicalOr);
                if !compare && !logical && !matches!(op, B::Add | B::Sub | B::Mul | B::Div | B::Mod)
                {
                    return Err(unsupported(*span, "binary operator"));
                }
                let hint = if logical {
                    Some(Type::Bool)
                } else {
                    self.hint(a)
                        .or_else(|| self.hint(b))
                        .or(if compare { None } else { expected })
                };
                let a = self.expression(a, hint)?;
                let b = self.expression(b, Some(a.ty))?;
                let valid = if logical {
                    a.ty == Type::Bool
                } else if matches!(op, B::Eq | B::NotEq) {
                    a.ty != Type::Void
                } else if *op == B::Mod {
                    a.ty.is_integer()
                } else {
                    a.ty.is_numeric()
                };
                if !valid {
                    return Err(error(
                        "E2007",
                        *span,
                        "invalid operand types for binary operator",
                    ));
                }
                (
                    if compare || logical { Type::Bool } else { a.ty },
                    ExpressionKind::Binary(Box::new(a), *op, Box::new(b)),
                )
            }
            E::Call(callee, args, span) => {
                let name =
                    path(callee).ok_or_else(|| unsupported(*span, "indirect function call"))?;
                return self.call(&name, args, *span, expected);
            }
            E::MethodCall(base, member, args, span) => {
                let name = path(base)
                    .map(|p| format!("{p}.{member}"))
                    .ok_or_else(|| unsupported(*span, "method call"))?;
                return self.call(&name, args, *span, expected);
            }
            E::Cast(value, target, span) => {
                let target = resolve_type(target)?;
                let value = self.expression(value, None)?;
                if !(target.is_numeric() && value.ty.is_numeric()) {
                    return Err(error(
                        "E2007",
                        *span,
                        "casts require numeric source and target types",
                    ));
                }
                (target, ExpressionKind::Cast(Box::new(value)))
            }
            _ => return Err(unsupported(expr.span(), "expression")),
        };
        compatible(ty, expected, expr.span())?;
        Ok(Expression { ty, kind })
    }
    fn integer(
        &self,
        n: i128,
        suffix: Option<&str>,
        span: Span,
        expected: Option<Type>,
    ) -> Result<Expression> {
        let ty = match suffix {
            Some(s) => Type::named(s)
                .filter(|t| t.is_integer())
                .ok_or_else(|| error("E2002", span, format!("unsupported integer suffix '{s}'")))?,
            None => expected.filter(|t| t.is_integer()).unwrap_or(Type::I32),
        };
        let (min, max) = ty.bounds().unwrap();
        if n < min || n > max {
            return Err(error(
                "E2003",
                span,
                format!("integer literal {n} out of range for {ty}"),
            ));
        }
        compatible(ty, expected, span)?;
        Ok(Expression {
            ty,
            kind: ExpressionKind::Int(n),
        })
    }
    fn call(
        &mut self,
        name: &str,
        args: &[ast::CallArg],
        span: Span,
        expected: Option<Type>,
    ) -> Result<Expression> {
        if self.lookup(name.split('.').next().unwrap()).is_some() {
            return Err(error("E2007", span, format!("'{name}' is not callable")));
        }
        let f = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| error("E2001", span, format!("undefined function '{name}'")))?;
        if args.len() != f.params.len() {
            return Err(error(
                "E2005",
                span,
                format!(
                    "function '{name}' expects {} arguments, got {}",
                    f.params.len(),
                    args.len()
                ),
            ));
        }
        let mut values = Vec::new();
        for (arg, (label, ty)) in args.iter().zip(&f.params) {
            if arg.label.as_ref().is_some_and(|s| s != label) {
                return Err(error(
                    "E2005",
                    arg.span,
                    format!("expected argument label '{label}'"),
                ));
            }
            values.push(self.expression(&arg.value, Some(*ty))?);
        }
        compatible(f.result, expected, span)?;
        Ok(Expression {
            ty: f.result,
            kind: ExpressionKind::Call(f.target, values),
        })
    }
}

fn compatible(actual: Type, expected: Option<Type>, span: Span) -> Result<()> {
    if let Some(expected) = expected {
        if actual != expected {
            return Err(error(
                "E2003",
                span,
                format!("expected {expected}, got {actual}"),
            ));
        }
    }
    Ok(())
}
fn resolve_type(ty: &ast::TypeExpr) -> Result<Type> {
    match ty {
        ast::TypeExpr::Named(name, span) => Type::named(name).ok_or_else(|| {
            error(
                "E2002",
                *span,
                format!("undefined or unsupported type '{name}'"),
            )
        }),
        _ => Err(unsupported(ty.span(), "type")),
    }
}
fn path(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Ident(name, _) => Some(name.clone()),
        ast::Expr::MemberAccess(base, member, _) => path(base).map(|p| format!("{p}.{member}")),
        _ => None,
    }
}
fn error(code: &'static str, span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(code, span, message)
}
fn unsupported(span: Span, what: &str) -> Diagnostic {
    error("E2011", span, format!("unsupported bootstrap {what}"))
}
fn item_span(item: &ast::Item) -> Span {
    match item {
        ast::Item::Function(v) => v.span,
        ast::Item::Struct(v) => v.span,
        ast::Item::Interface(v) => v.span,
        ast::Item::Extend(v) => v.span,
        ast::Item::TypeAlias(v) => v.span,
        ast::Item::Test(v) => v.span,
        ast::Item::Global(v) => v.span,
        ast::Item::Import(v) => v.span,
        ast::Item::Enum(v) => v.span,
        ast::Item::Union(v) => v.span,
    }
}
