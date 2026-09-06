//! Hand-written recursive descent + Pratt parser for Juyu.

use crate::syntax::ast::*;
use crate::syntax::lexer::Lexer;
use crate::syntax::token::{Span, Token, TokenKind};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    previous: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let first = lexer.next_token();
        Self {
            lexer,
            current: first,
            previous: Token::new(TokenKind::Eof, Span::dummy()),
        }
    }

    fn advance(&mut self) -> Token {
        let next = self.lexer.next_token();
        self.previous = std::mem::replace(&mut self.current, next);
        self.previous.clone()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        &self.current.kind == kind
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, msg: &str) -> Result<Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(format!(
                "Syntax Error at line {}, col {}: Expected {}, got {:?}",
                self.current.span.line, self.current.span.col, msg, self.current.kind
            ))
        }
    }

    pub fn parse_file(&mut self) -> Result<SourceFile, String> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(SourceFile { items })
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        let is_pub = self.match_token(&TokenKind::Pub);
        let visibility = if is_pub { Visibility::Public } else { Visibility::Private };

        if self.match_token(&TokenKind::Import) {
            return self.parse_import(self.previous.span);
        }

        if self.match_token(&TokenKind::Test) {
            return self.parse_test();
        }

        if self.match_token(&TokenKind::Extend) {
            return self.parse_extend();
        }

        if self.match_token(&TokenKind::Interface) {
            return self.parse_interface(visibility);
        }

        if self.match_token(&TokenKind::Type) {
            return self.parse_type_alias(visibility);
        }

        if self.match_token(&TokenKind::Enum) {
            return self.parse_enum(visibility);
        }

        if self.match_token(&TokenKind::Union) {
            return self.parse_union(visibility);
        }

        if self.check(&TokenKind::Struct) || self.check(&TokenKind::Packed) || self.check(&TokenKind::Extern) {
            return self.parse_struct(visibility);
        }

        if self.check(&TokenKind::Fn) || self.check(&TokenKind::Static) || self.check(&TokenKind::Inline) || self.check(&TokenKind::Const) || self.check(&TokenKind::Naked) {
            return self.parse_function(visibility);
        }

        if self.match_token(&TokenKind::Let) {
            let var = self.parse_var_decl(false)?;
            self.expect(&TokenKind::Semicolon, "';' after let declaration")?;
            return Ok(Item::Global(var));
        }

        if self.match_token(&TokenKind::Var) {
            let var = self.parse_var_decl(true)?;
            self.expect(&TokenKind::Semicolon, "';' after var declaration")?;
            return Ok(Item::Global(var));
        }

        Err(format!(
            "Syntax Error at line {}: Unexpected token at top-level: {:?}",
            self.current.span.line, self.current.kind
        ))
    }

    fn parse_import(&mut self, start_span: Span) -> Result<Item, String> {
        // import { a, b } from "path";
        self.expect(&TokenKind::OpenBrace, "'{' for selective import")?;
        let mut selective = Vec::new();
        while !self.check(&TokenKind::CloseBrace) {
            let name_tok = self.expect_ident("imported symbol name")?;
            selective.push(name_tok);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::CloseBrace, "'}' after import list")?;
        
        let from_tok = self.advance();
        if let TokenKind::Ident(ref s) = from_tok.kind {
            if s != "from" {
                return Err("Expected 'from' after selective import list".to_string());
            }
        } else {
            return Err("Expected 'from' keyword".to_string());
        }

        let path_tok = self.advance();
        let path = match path_tok.kind {
            TokenKind::StringLit(s) => s,
            _ => return Err("Expected string path for import".to_string()),
        };
        self.expect(&TokenKind::Semicolon, "';' after import")?;

        Ok(Item::Import(ImportDecl {
            path,
            alias: None,
            selective,
            span: start_span,
        }))
    }

    fn parse_test(&mut self) -> Result<Item, String> {
        let name_tok = self.advance();
        let name = match name_tok.kind {
            TokenKind::StringLit(s) => s,
            _ => return Err("Expected string description for test block".to_string()),
        };
        let body = self.parse_block()?;
        Ok(Item::Test(TestDecl {
            name,
            body,
            span: name_tok.span,
        }))
    }

    fn parse_extend(&mut self) -> Result<Item, String> {
        let target = self.expect_ident("type to extend")?;
        self.expect(&TokenKind::OpenBrace, "'{' after extend type")?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::CloseBrace) {
            let is_pub = self.match_token(&TokenKind::Pub);
            let vis = if is_pub { Visibility::Public } else { Visibility::Private };
            if let Item::Function(func) = self.parse_function(vis)? {
                methods.push(func);
            }
        }
        self.expect(&TokenKind::CloseBrace, "'}' after extend methods")?;
        Ok(Item::Extend(ExtendDecl {
            target,
            methods,
            span: self.previous.span,
        }))
    }

    fn parse_interface(&mut self, visibility: Visibility) -> Result<Item, String> {
        let name = self.expect_ident("interface name")?;
        self.expect(&TokenKind::OpenBrace, "'{' after interface name")?;
        let mut methods = Vec::new();

        while !self.check(&TokenKind::CloseBrace) {
            self.expect(&TokenKind::Fn, "'fn' for interface method")?;
            let m_name = self.expect_ident("method name")?;
            let params = self.parse_params()?;
            let return_type = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(&TokenKind::Semicolon, "';' after interface method signature")?;
            methods.push(InterfaceMethod {
                name: m_name,
                params,
                return_type,
                span: self.previous.span,
            });
        }
        self.expect(&TokenKind::CloseBrace, "'}' after interface body")?;
        Ok(Item::Interface(InterfaceDecl {
            name,
            visibility,
            methods,
            span: self.previous.span,
        }))
    }

    fn parse_type_alias(&mut self, visibility: Visibility) -> Result<Item, String> {
        let name = self.expect_ident("type alias name")?;
        self.expect(&TokenKind::Eq, "'=' in type alias")?;
        let is_distinct = self.match_token(&TokenKind::Distinct);
        let target = self.parse_type()?;
        self.expect(&TokenKind::Semicolon, "';' after type alias")?;

        Ok(Item::TypeAlias(TypeAliasDecl {
            name,
            visibility,
            is_distinct,
            target,
            span: self.previous.span,
        }))
    }

    fn parse_enum(&mut self, visibility: Visibility) -> Result<Item, String> {
        let span = self.previous.span;
        let backing_type = if self.match_token(&TokenKind::OpenParen) {
            let ty = self.parse_type()?;
            self.expect(&TokenKind::CloseParen, "')' after enum backing type")?;
            Some(ty)
        } else {
            None
        };

        let name = self.expect_ident("enum name")?;
        self.expect(&TokenKind::OpenBrace, "'{' for enum variants")?;
        let mut variants = Vec::new();

        while !self.check(&TokenKind::CloseBrace) {
            let v_name = self.expect_ident("enum variant name")?;
            let value = if self.match_token(&TokenKind::Eq) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            variants.push(EnumVariant {
                name: v_name,
                value,
                span: self.previous.span,
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.expect(&TokenKind::CloseBrace, "'}' after enum variants")?;
        Ok(Item::Enum(EnumDecl {
            name,
            visibility,
            backing_type,
            variants,
            span,
        }))
    }

    fn parse_union(&mut self, visibility: Visibility) -> Result<Item, String> {
        let span = self.previous.span;
        let is_tagged = if self.match_token(&TokenKind::OpenParen) {
            self.expect(&TokenKind::Enum, "'enum' in union(enum)")?;
            self.expect(&TokenKind::CloseParen, "')' after union(enum)")?;
            true
        } else {
            false
        };

        let name = self.expect_ident("union name")?;
        self.expect(&TokenKind::OpenBrace, "'{' for union fields")?;
        let mut fields = Vec::new();

        while !self.check(&TokenKind::CloseBrace) {
            let f_name = self.expect_ident("union variant name")?;
            let ty = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            fields.push(UnionField {
                name: f_name,
                ty,
                span: self.previous.span,
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.expect(&TokenKind::CloseBrace, "'}' after union fields")?;
        Ok(Item::Union(UnionDecl {
            name,
            visibility,
            is_tagged,
            fields,
            span,
        }))
    }

    fn parse_struct(&mut self, visibility: Visibility) -> Result<Item, String> {
        let kind = if self.match_token(&TokenKind::Packed) {
            self.expect(&TokenKind::Struct, "'struct' after packed")?;
            StructKind::Packed
        } else if self.match_token(&TokenKind::Extern) {
            self.expect(&TokenKind::Struct, "'struct' after extern")?;
            StructKind::Extern
        } else {
            self.expect(&TokenKind::Struct, "'struct' keyword")?;
            StructKind::Normal
        };

        let name = self.expect_ident("struct name")?;
        self.expect(&TokenKind::OpenBrace, "'{' after struct name")?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.check(&TokenKind::CloseBrace) {
            if self.check(&TokenKind::Pub) || self.check(&TokenKind::Fn) || self.check(&TokenKind::Static) {
                let is_pub = self.match_token(&TokenKind::Pub);
                let vis = if is_pub { Visibility::Public } else { Visibility::Private };
                if let Item::Function(func) = self.parse_function(vis)? {
                    methods.push(func);
                }
            } else {
                // Struct field or anonymous embedded composition
                let first_ident = self.expect_ident("field name or embedded type")?;
                if self.match_token(&TokenKind::Colon) {
                    // Standard named field: 'name: Type'
                    let ty = self.parse_type()?;
                    let default_val = if self.match_token(&TokenKind::Eq) {
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    self.match_token(&TokenKind::Comma); // optional comma
                    fields.push(StructField {
                        name: Some(first_ident),
                        ty,
                        default_val,
                        span: self.previous.span,
                    });
                } else {
                    // Anonymous embedded composition: 'Entity,'
                    self.match_token(&TokenKind::Comma);
                    fields.push(StructField {
                        name: None,
                        ty: TypeExpr::Named(first_ident, self.previous.span),
                        default_val: None,
                        span: self.previous.span,
                    });
                }
            }
        }
        self.expect(&TokenKind::CloseBrace, "'}' after struct body")?;

        Ok(Item::Struct(StructDecl {
            name,
            visibility,
            kind,
            fields,
            methods,
            span: self.previous.span,
        }))
    }

    fn parse_function(&mut self, visibility: Visibility) -> Result<Item, String> {
        let is_static = self.match_token(&TokenKind::Static);
        let is_inline = self.match_token(&TokenKind::Inline);
        let is_const = self.match_token(&TokenKind::Const);
        let is_naked = self.match_token(&TokenKind::Naked);

        self.expect(&TokenKind::Fn, "'fn' keyword")?;
        let name = self.expect_ident("function name")?;
        let params = self.parse_params()?;

        let return_type = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Function body: either '=> expr;' or '{ ... }'
        let body = if self.match_token(&TokenKind::FatArrow) {
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "';' after expression function body")?;
            Some(FunctionBody::Expr(Box::new(expr)))
        } else if self.check(&TokenKind::OpenBrace) {
            let block = self.parse_block()?;
            Some(FunctionBody::Block(block))
        } else {
            self.expect(&TokenKind::Semicolon, "';' after extern function declaration")?;
            None
        };

        Ok(Item::Function(FunctionDecl {
            name,
            visibility,
            is_static,
            is_inline,
            is_const,
            is_naked,
            params,
            return_type,
            body,
            span: self.previous.span,
        }))
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        self.expect(&TokenKind::OpenParen, "'(' before parameters")?;
        let mut params = Vec::new();

        while !self.check(&TokenKind::CloseParen) {
            let first_tok = self.expect_ident("parameter name")?;
            let (label, name) = if let TokenKind::Ident(second) = self.current.kind.clone() {
                self.advance();
                (Some(first_tok), second)
            } else {
                (None, first_tok)
            };

            self.expect(&TokenKind::Colon, "':' after parameter name")?;
            let ty = self.parse_type()?;
            let default_val = if self.match_token(&TokenKind::Eq) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            params.push(Param {
                name,
                label,
                ty,
                default_val,
                span: self.previous.span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.expect(&TokenKind::CloseParen, "')' after parameters")?;
        Ok(params)
    }

    pub fn parse_type(&mut self) -> Result<TypeExpr, String> {
        let span = self.current.span;

        if self.match_token(&TokenKind::Bang) {
            // Inferred error union !T
            let inner = self.parse_type()?;
            return Ok(TypeExpr::ErrorUnion(None, Box::new(inner), span));
        }

        if self.match_token(&TokenKind::Question) {
            // Optional ?T or ?*T
            let inner = self.parse_type()?;
            return Ok(TypeExpr::Optional(Box::new(inner), span));
        }

        if self.match_token(&TokenKind::Star) {
            // Pointer *T or *const T
            let is_const = self.match_token(&TokenKind::Const);
            let inner = self.parse_type()?;
            return Ok(TypeExpr::Pointer(Box::new(inner), is_const, span));
        }

        if self.match_token(&TokenKind::OpenBracket) {
            // Slice []T or Array [N]T
            if self.match_token(&TokenKind::CloseBracket) {
                let is_const = self.match_token(&TokenKind::Const);
                let inner = self.parse_type()?;
                return Ok(TypeExpr::Slice(Box::new(inner), is_const, span));
            } else {
                let size_tok = self.advance();
                let size = match size_tok.kind {
                    TokenKind::Int(v, _) => v as usize,
                    _ => 0,
                };
                self.expect(&TokenKind::CloseBracket, "']' after array length")?;
                let inner = self.parse_type()?;
                return Ok(TypeExpr::Array(Box::new(inner), size, span));
            }
        }

        let name = self.expect_ident("type name")?;
        if name == "anytype" {
            Ok(TypeExpr::Anytype(span))
        } else {
            Ok(TypeExpr::Named(name, span))
        }
    }

    pub fn parse_block(&mut self) -> Result<Block, String> {
        let start_span = self.current.span;
        self.expect(&TokenKind::OpenBrace, "'{' to begin block")?;

        let mut stmts = Vec::new();
        let mut yield_expr = None;

        while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
            if self.match_token(&TokenKind::FatArrow) {
                // '=> expr;' arrow block return
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon, "';' after arrow block return")?;
                yield_expr = Some(Box::new(expr));
                break;
            }

            stmts.push(self.parse_stmt()?);
        }

        self.expect(&TokenKind::CloseBrace, "'}' to close block")?;
        Ok(Block {
            stmts,
            yield_expr,
            span: start_span,
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let span = self.current.span;

        if self.match_token(&TokenKind::Let) {
            let var = self.parse_var_decl(false)?;
            self.expect(&TokenKind::Semicolon, "';' after let statement")?;
            return Ok(Stmt::Let(var));
        }

        if self.match_token(&TokenKind::Var) {
            let var = self.parse_var_decl(true)?;
            self.expect(&TokenKind::Semicolon, "';' after var statement")?;
            return Ok(Stmt::Var(var));
        }

        if self.match_token(&TokenKind::Defer) {
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "';' after defer statement")?;
            return Ok(Stmt::Defer(expr, span));
        }

        if self.match_token(&TokenKind::Errdefer) {
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "';' after errdefer statement")?;
            return Ok(Stmt::Errdefer(expr, span));
        }

        let expr = self.parse_expr()?;

        // Check for assignment operators: =, +=, -=, *=, /=
        let assign_op = if self.match_token(&TokenKind::Eq) {
            Some(AssignOp::Assign)
        } else if self.match_token(&TokenKind::PlusEq) {
            Some(AssignOp::PlusAssign)
        } else if self.match_token(&TokenKind::MinusEq) {
            Some(AssignOp::MinusAssign)
        } else if self.match_token(&TokenKind::StarEq) {
            Some(AssignOp::StarAssign)
        } else if self.match_token(&TokenKind::SlashEq) {
            Some(AssignOp::SlashAssign)
        } else {
            None
        };

        if let Some(op) = assign_op {
            let val = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "';' after assignment")?;
            return Ok(Stmt::Assign(expr, op, val, span));
        }

        let is_block_expr = matches!(
            expr,
            Expr::If(..)
                | Expr::IfLet(..)
                | Expr::Match(..)
                | Expr::Loop(..)
                | Expr::While(..)
                | Expr::ForC(..)
                | Expr::ForIn(..)
                | Expr::Block(..)
        );

        if !is_block_expr {
            self.expect(&TokenKind::Semicolon, "';' after expression statement")?;
        } else {
            self.match_token(&TokenKind::Semicolon); // optional semicolon after block statements
        }
        Ok(Stmt::Expr(expr, span))
    }

    fn parse_var_decl(&mut self, is_mut: bool) -> Result<VarDecl, String> {
        let name = self.expect_ident("variable name")?;
        let ty = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&TokenKind::Eq, "'=' in variable declaration")?;
        let value = self.parse_expr()?;

        Ok(VarDecl {
            name,
            is_mut,
            ty,
            value,
            span: self.previous.span,
        })
    }

    // =========================================================================
    // Expression Parsing (Pratt Parsing)
    // =========================================================================

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, String> {
        let lhs = self.parse_prefix()?;
        self.parse_postfix_and_infix(lhs, min_bp)
    }

    fn parse_postfix_and_infix(&mut self, mut lhs: Expr, min_bp: u8) -> Result<Expr, String> {
        loop {
            // Postfix operators: (), [], .field, ?.field, .*, catch
            if self.check(&TokenKind::OpenParen) {
                lhs = self.parse_call(lhs)?;
                continue;
            }

            if self.match_token(&TokenKind::DotStar) {
                lhs = Expr::Deref(Box::new(lhs), self.previous.span);
                continue;
            }

            if self.match_token(&TokenKind::QuestionDot) {
                let member = self.expect_ident("member name after '?.'")?;
                lhs = Expr::OptionalChaining(Box::new(lhs), member, self.previous.span);
                continue;
            }

            if self.match_token(&TokenKind::Dot) {
                let member = self.expect_ident("field or method name")?;
                if self.check(&TokenKind::OpenParen) {
                    let call = self.parse_call(Expr::Ident(member.clone(), self.previous.span))?;
                    if let Expr::Call(_, args, span) = call {
                        lhs = Expr::MethodCall(Box::new(lhs), member, args, span);
                    }
                } else {
                    lhs = Expr::MemberAccess(Box::new(lhs), member, self.previous.span);
                }
                continue;
            }

            if self.match_token(&TokenKind::Catch) {
                let fallback = self.parse_expr()?;
                lhs = Expr::Catch(Box::new(lhs), None, Box::new(fallback), self.previous.span);
                continue;
            }

            // Infix binary operators
            let (op, l_bp, r_bp) = match self.current.kind {
                TokenKind::QuestionQuestion => (None, 1, 2), // Null-coalesce
                TokenKind::PipePipe => (Some(BinaryOp::LogicalOr), 3, 4),
                TokenKind::AmpAmp => (Some(BinaryOp::LogicalAnd), 5, 6),
                TokenKind::EqEq => (Some(BinaryOp::Eq), 7, 8),
                TokenKind::NotEq => (Some(BinaryOp::NotEq), 7, 8),
                TokenKind::Lt => (Some(BinaryOp::Lt), 9, 10),
                TokenKind::LtEq => (Some(BinaryOp::LtEq), 9, 10),
                TokenKind::Gt => (Some(BinaryOp::Gt), 9, 10),
                TokenKind::GtEq => (Some(BinaryOp::GtEq), 9, 10),
                TokenKind::Pipe => (Some(BinaryOp::BitOr), 11, 12),
                TokenKind::Caret => (Some(BinaryOp::BitXor), 13, 14),
                TokenKind::Ampersand => (Some(BinaryOp::BitAnd), 15, 16),
                TokenKind::Shl => (Some(BinaryOp::Shl), 17, 18),
                TokenKind::Shr => (Some(BinaryOp::Shr), 17, 18),
                TokenKind::Plus => (Some(BinaryOp::Add), 19, 20),
                TokenKind::Minus => (Some(BinaryOp::Sub), 19, 20),
                TokenKind::PlusPercent => (Some(BinaryOp::AddWrap), 19, 20),
                TokenKind::MinusPercent => (Some(BinaryOp::SubWrap), 19, 20),
                TokenKind::Star => (Some(BinaryOp::Mul), 21, 22),
                TokenKind::Slash => (Some(BinaryOp::Div), 21, 22),
                TokenKind::Percent => (Some(BinaryOp::Mod), 21, 22),
                TokenKind::StarPercent => (Some(BinaryOp::MulWrap), 21, 22),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            let op_tok = self.advance();
            let rhs = self.parse_expr_bp(r_bp)?;

            if op_tok.kind == TokenKind::QuestionQuestion {
                lhs = Expr::NullCoalesce(Box::new(lhs), Box::new(rhs), op_tok.span);
            } else if let Some(bin_op) = op {
                lhs = Expr::Binary(Box::new(lhs), bin_op, Box::new(rhs), op_tok.span);
            }
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::Int(v, suffix) => Ok(Expr::Int(v, suffix, tok.span)),
            TokenKind::Float(v) => Ok(Expr::Float(v, tok.span)),
            TokenKind::StringLit(s) => Ok(Expr::StringLit(s, tok.span)),
            TokenKind::InterpolatedString(s) => Ok(Expr::InterpolatedString(s, tok.span)),
            TokenKind::BoolLit(b) => Ok(Expr::BoolLit(b, tok.span)),
            TokenKind::Null => Ok(Expr::Null(tok.span)),
            TokenKind::Undefined => Ok(Expr::Undefined(tok.span)),
            TokenKind::Unreachable => Ok(Expr::Unreachable(tok.span)),

            TokenKind::Ident(s) => {
                // Check if this is a struct initializer: 'Point { .x = 10 }'
                if self.check(&TokenKind::OpenBrace) {
                    return self.parse_struct_init(TypeExpr::Named(s, tok.span));
                }
                Ok(Expr::Ident(s, tok.span))
            }

            TokenKind::Builtin(s) => {
                // Builtins like @cast, @sizeOf, @cImport, @assert
                self.expect(&TokenKind::OpenParen, "'(' after builtin")?;
                let mut args = Vec::new();
                while !self.check(&TokenKind::CloseParen) {
                    let val = self.parse_expr()?;
                    args.push(CallArg {
                        label: None,
                        value: val,
                        span: self.previous.span,
                    });
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::CloseParen, "')' after builtin arguments")?;
                Ok(Expr::Builtin(s, args, tok.span))
            }

            TokenKind::Bang => {
                let operand = self.parse_expr_bp(25)?;
                Ok(Expr::Unary(UnaryOp::Not, Box::new(operand), tok.span))
            }
            TokenKind::Minus => {
                let operand = self.parse_expr_bp(25)?;
                Ok(Expr::Unary(UnaryOp::Neg, Box::new(operand), tok.span))
            }
            TokenKind::Tilde => {
                let operand = self.parse_expr_bp(25)?;
                Ok(Expr::Unary(UnaryOp::BitNot, Box::new(operand), tok.span))
            }
            TokenKind::Ampersand => {
                let operand = self.parse_expr_bp(25)?;
                Ok(Expr::Unary(UnaryOp::AddressOf, Box::new(operand), tok.span))
            }

            TokenKind::Try => {
                let operand = self.parse_expr_bp(25)?;
                Ok(Expr::Try(Box::new(operand), tok.span))
            }

            TokenKind::OpenParen => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::CloseParen, "')' to match '('")?;
                Ok(expr)
            }

            TokenKind::OpenBrace => {
                // Block expression
                self.parse_block_expr(tok.span)
            }

            TokenKind::If => self.parse_if(tok.span),
            TokenKind::Match => self.parse_match(tok.span),
            TokenKind::Loop => self.parse_loop(None, tok.span),
            TokenKind::While => self.parse_while(None, tok.span),
            TokenKind::For => self.parse_for(None, tok.span),

            other => Err(format!(
                "Syntax Error at line {}: Unexpected prefix expression token {:?}",
                tok.span.line, other
            )),
        }
    }

    fn parse_call(&mut self, callee: Expr) -> Result<Expr, String> {
        let span = self.current.span;
        self.expect(&TokenKind::OpenParen, "'(' for function call")?;
        let mut args = Vec::new();

        while !self.check(&TokenKind::CloseParen) {
            let mut label = None;
            let val = if let TokenKind::Ident(ref lbl) = self.current.kind {
                let lbl_clone = lbl.clone();
                let ident_tok = self.advance();
                if self.match_token(&TokenKind::Colon) {
                    label = Some(lbl_clone);
                    self.parse_expr()?
                } else {
                    let ident_expr = Expr::Ident(lbl_clone, ident_tok.span);
                    self.parse_postfix_and_infix(ident_expr, 0)?
                }
            } else {
                self.parse_expr()?
            };

            args.push(CallArg {
                label,
                value: val,
                span: self.previous.span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.expect(&TokenKind::CloseParen, "')' after call arguments")?;
        Ok(Expr::Call(Box::new(callee), args, span))
    }

    fn parse_struct_init(&mut self, ty: TypeExpr) -> Result<Expr, String> {
        let span = self.current.span;
        self.expect(&TokenKind::OpenBrace, "'{' for struct initializer")?;
        let mut fields = Vec::new();

        while !self.check(&TokenKind::CloseBrace) {
            self.expect(&TokenKind::Dot, "'.' for designated field initializer")?;
            let name = self.expect_ident("field name")?;
            self.expect(&TokenKind::Eq, "'=' after designated field")?;
            let val = self.parse_expr()?;

            fields.push(FieldInit {
                name,
                value: val,
                span: self.previous.span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.expect(&TokenKind::CloseBrace, "'}' after struct initializer")?;
        Ok(Expr::StructInit(ty, fields, span))
    }

    fn parse_block_expr(&mut self, start_span: Span) -> Result<Expr, String> {
        let mut stmts = Vec::new();
        let mut yield_expr = None;

        while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
            if self.match_token(&TokenKind::FatArrow) {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon, "';' after arrow block return")?;
                yield_expr = Some(Box::new(expr));
                break;
            }
            stmts.push(self.parse_stmt()?);
        }

        self.expect(&TokenKind::CloseBrace, "'}' to close block")?;
        Ok(Expr::Block(Block {
            stmts,
            yield_expr,
            span: start_span,
        }))
    }

    fn parse_branch_block(&mut self) -> Result<Block, String> {
        if self.check(&TokenKind::OpenBrace) {
            self.parse_block()
        } else {
            let expr = self.parse_expr()?;
            Ok(Block {
                stmts: Vec::new(),
                yield_expr: Some(Box::new(expr)),
                span: self.previous.span,
            })
        }
    }

    fn parse_if(&mut self, start_span: Span) -> Result<Expr, String> {
        let has_paren = self.match_token(&TokenKind::OpenParen);

        // Check for optional unwrapping: 'if (let x = maybe)'
        if self.match_token(&TokenKind::Let) {
            let var_name = self.expect_ident("unwrapped variable name")?;
            self.expect(&TokenKind::Eq, "'=' in if-let")?;
            let expr = self.parse_expr()?;
            if has_paren {
                self.expect(&TokenKind::CloseParen, "')' after if-let condition")?;
            }
            let then_block = self.parse_branch_block()?;
            let else_block = if self.match_token(&TokenKind::Else) {
                Some(self.parse_branch_block()?)
            } else {
                None
            };
            return Ok(Expr::IfLet(var_name, Box::new(expr), then_block, else_block, start_span));
        }

        let cond = self.parse_expr()?;
        if has_paren {
            self.expect(&TokenKind::CloseParen, "')' after if condition")?;
        }

        let then_block = self.parse_branch_block()?;
        let else_block = if self.match_token(&TokenKind::Else) {
            Some(self.parse_branch_block()?)
        } else {
            None
        };

        Ok(Expr::If(Box::new(cond), then_block, else_block, start_span))
    }

    fn parse_match(&mut self, start_span: Span) -> Result<Expr, String> {
        let has_paren = self.match_token(&TokenKind::OpenParen);
        let target = self.parse_expr()?;
        if has_paren {
            self.expect(&TokenKind::CloseParen, "')' after match target")?;
        }

        self.expect(&TokenKind::OpenBrace, "'{' for match arms")?;
        let mut arms = Vec::new();

        while !self.check(&TokenKind::CloseBrace) {
            let pattern = if self.match_token(&TokenKind::Else) {
                Pattern::Else(self.previous.span)
            } else if self.match_token(&TokenKind::Dot) {
                let tag = self.expect_ident("enum tag or variant")?;
                let binding = if self.match_token(&TokenKind::OpenParen) {
                    self.expect(&TokenKind::Let, "'let' in variant payload binding")?;
                    let name = self.expect_ident("payload binding name")?;
                    self.expect(&TokenKind::CloseParen, "')' after payload binding")?;
                    Some(name)
                } else {
                    None
                };
                Pattern::Variant(tag, binding, self.previous.span)
            } else {
                let lit = self.parse_expr()?;
                Pattern::Literal(Box::new(lit), self.previous.span)
            };

            self.expect(&TokenKind::FatArrow, "'=>' after match pattern")?;
            let body = if self.check(&TokenKind::OpenBrace) {
                MatchArmBody::Block(self.parse_block()?)
            } else {
                let expr = self.parse_expr()?;
                self.match_token(&TokenKind::Comma);
                MatchArmBody::Expr(Box::new(expr))
            };

            arms.push(MatchArm {
                pattern,
                body,
                span: self.previous.span,
            });

            self.match_token(&TokenKind::Comma);
        }

        self.expect(&TokenKind::CloseBrace, "'}' after match arms")?;
        Ok(Expr::Match(Box::new(target), arms, start_span))
    }

    fn parse_loop(&mut self, label: Option<String>, start_span: Span) -> Result<Expr, String> {
        let block = self.parse_block()?;
        Ok(Expr::Loop(label, block, start_span))
    }

    fn parse_while(&mut self, label: Option<String>, start_span: Span) -> Result<Expr, String> {
        let has_paren = self.match_token(&TokenKind::OpenParen);
        let cond = self.parse_expr()?;
        if has_paren {
            self.expect(&TokenKind::CloseParen, "')' after while condition")?;
        }
        let block = self.parse_block()?;
        Ok(Expr::While(label, Box::new(cond), block, start_span))
    }

    fn parse_for(&mut self, label: Option<String>, start_span: Span) -> Result<Expr, String> {
        let has_paren = self.match_token(&TokenKind::OpenParen);

        // Check if collection for: 'for (item in slice)'
        if let TokenKind::Ident(ref var_name) = self.current.kind.clone() {
            // Check if next token is 'in'
            let next_tok = self.lexer.next_token();
            if next_tok.kind == TokenKind::In {
                self.advance(); // consume ident
                let iter_expr = self.parse_expr()?;
                if has_paren {
                    self.expect(&TokenKind::CloseParen, "')' after for-in loop")?;
                }
                let block = self.parse_block()?;
                return Ok(Expr::ForIn(label, var_name.clone(), Box::new(iter_expr), block, start_span));
            }
        }

        // Three-part C-style for: 'for (var i = 0; i < n; i += 1)'
        self.expect(&TokenKind::Var, "'var' in 3-part for loop")?;
        let init_var = self.parse_var_decl(true)?;
        self.expect(&TokenKind::Semicolon, "';' after for-init")?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::Semicolon, "';' after for-condition")?;
        let step = {
            let lhs = self.parse_expr()?;
            if self.match_token(&TokenKind::PlusEq) {
                let rhs = self.parse_expr()?;
                Expr::Binary(Box::new(lhs), BinaryOp::Add, Box::new(rhs), self.previous.span)
            } else if self.match_token(&TokenKind::MinusEq) {
                let rhs = self.parse_expr()?;
                Expr::Binary(Box::new(lhs), BinaryOp::Sub, Box::new(rhs), self.previous.span)
            } else if self.match_token(&TokenKind::Eq) {
                self.parse_expr()?
            } else {
                lhs
            }
        };
        if has_paren {
            self.expect(&TokenKind::CloseParen, "')' after 3-part for header")?;
        }
        let block = self.parse_block()?;
        Ok(Expr::ForC(
            label,
            Box::new(Stmt::Var(init_var)),
            Box::new(cond),
            Box::new(step),
            block,
            start_span,
        ))
    }

    fn expect_ident(&mut self, msg: &str) -> Result<String, String> {
        if let TokenKind::Ident(s) = self.current.kind.clone() {
            self.advance();
            Ok(s)
        } else {
            Err(format!(
                "Syntax Error at line {}, col {}: Expected {}, got {:?}",
                self.current.span.line, self.current.span.col, msg, self.current.kind
            ))
        }
    }
}
