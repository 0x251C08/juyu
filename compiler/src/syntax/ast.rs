//! Abstract Syntax Tree (AST) node definitions for Juyu.

use crate::syntax::token::Span;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Export,
    Private,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDecl),
    Struct(StructDecl),
    Interface(InterfaceDecl),
    Extend(ExtendDecl),
    TypeAlias(TypeAliasDecl),
    Test(TestDecl),
    Global(VarDecl),
    Import(ImportDecl),
    Enum(EnumDecl),
    Union(UnionDecl),
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub visibility: Visibility,
    pub backing_type: Option<TypeExpr>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnionDecl {
    pub name: String,
    pub visibility: Visibility,
    pub is_tagged: bool,
    pub fields: Vec<UnionField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnionField {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
    pub alias: Option<String>,
    pub selective: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_inline: bool,
    pub is_const: bool,
    pub is_naked: bool,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Option<FunctionBody>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum FunctionBody {
    Block(Block),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub label: Option<String>,
    pub ty: TypeExpr,
    pub default_val: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructKind {
    Normal,
    Extern,
    Packed,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub visibility: Visibility,
    pub kind: StructKind,
    pub fields: Vec<StructField>,
    pub methods: Vec<FunctionDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: Option<String>, // None for embedded anonymous composition
    pub ty: TypeExpr,
    pub default_val: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: String,
    pub visibility: Visibility,
    pub methods: Vec<InterfaceMethod>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExtendDecl {
    pub target: String,
    pub methods: Vec<FunctionDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAliasDecl {
    pub name: String,
    pub visibility: Visibility,
    pub is_distinct: bool,
    pub target: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TestDecl {
    pub name: String,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub is_mut: bool,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String, Span),
    Pointer(Box<TypeExpr>, bool, Span), // inner, is_const
    Optional(Box<TypeExpr>, Span),
    Slice(Box<TypeExpr>, Span),
    Generic(Box<TypeExpr>, Vec<TypeExpr>, Span),
    Array(Box<TypeExpr>, usize, Span),
    SentinelSlice(Box<TypeExpr>, u8, Span),
    ErrorUnion(Option<Box<TypeExpr>>, Box<TypeExpr>, Span),
    Tuple(Vec<TupleFieldType>, Span),
    DynInterface(Box<TypeExpr>, Span),
    Anytype(Span),
}

#[derive(Debug, Clone)]
pub struct TupleFieldType {
    pub label: Option<String>,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub yield_expr: Option<Box<Expr>>, // From '=> expr;'
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(VarDecl),
    Var(VarDecl),
    Defer(Expr, Span),
    Errdefer(Expr, Span),
    Assign(Expr, AssignOp, Expr, Span),
    Expr(Expr, Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i128, Option<String>, Span),
    Float(f64, Span),
    StringLit(String, Span),
    InterpolatedString(String, Span),
    BoolLit(bool, Span),
    Ident(String, Span),
    Null(Span),
    Undefined(Span),
    Unreachable(Span),

    Builtin(String, Vec<CallArg>, Span),
    Binary(Box<Expr>, BinaryOp, Box<Expr>, Span),
    Unary(UnaryOp, Box<Expr>, Span),

    Call(Box<Expr>, Vec<CallArg>, Span),
    MethodCall(Box<Expr>, String, Vec<CallArg>, Span),
    MemberAccess(Box<Expr>, String, Span),
    OptionalChaining(Box<Expr>, String, Span),
    Deref(Box<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Slice(Box<Expr>, Box<Expr>, Box<Expr>, Span),

    StructInit(TypeExpr, Vec<FieldInit>, Span),
    Tuple(Vec<Expr>, Span),
    Block(Block),

    If(Box<Expr>, Block, Option<Block>, Span),
    IfLet(String, Box<Expr>, Block, Option<Block>, Span),
    Match(Box<Expr>, Vec<MatchArm>, Span),

    Loop(Option<String>, Block, Span),
    While(Option<String>, Box<Expr>, Block, Span),
    ForC(Option<String>, Box<Stmt>, Box<Expr>, Box<Expr>, Block, Span),
    ForIn(Option<String>, String, Box<Expr>, Block, Span),

    Break(Option<String>, Option<Box<Expr>>, Span),
    Continue(Option<String>, Span),
    Return(Option<Box<Expr>>, Span),

    Try(Box<Expr>, Span),
    Catch(Box<Expr>, Option<String>, Box<Expr>, Span),
    NullCoalesce(Box<Expr>, Box<Expr>, Span),
    ForceUnwrap(Box<Expr>, Span),
    Cast(Box<Expr>, TypeExpr, Span),
}

#[derive(Debug, Clone)]
pub struct CallArg {
    pub label: Option<String>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchArmBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchArmBody {
    Expr(Box<Expr>),
    Block(Block),
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard(Span),
    Literal(Box<Expr>, Span),
    Range(Box<Expr>, Box<Expr>, Span),
    Variant(String, Option<String>, Span), // tag, payload binding
    Else(Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    AddWrap,
    SubWrap,
    MulWrap,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    LogicalAnd,
    LogicalOr,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    AddressOf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::token::Span;

    #[test]
    fn test_ast_nodes_can_be_constructed() {
        let span = Span::dummy();
        
        let vis = Visibility::Export;
        
        let slice_type = TypeExpr::Slice(Box::new(TypeExpr::Named("u8".to_string(), span)), span);
        
        let generic_type = TypeExpr::Generic(
            Box::new(TypeExpr::Named("List".to_string(), span)),
            vec![TypeExpr::Named("T".to_string(), span)],
            span
        );
        
        let expr = Expr::Cast(
            Box::new(Expr::Int(42, None, span)),
            TypeExpr::Named("u64".to_string(), span),
            span
        );
        
        assert_eq!(vis, Visibility::Export);
        assert!(matches!(slice_type, TypeExpr::Slice(_, _)));
        assert!(matches!(generic_type, TypeExpr::Generic(_, _, _)));
        assert!(matches!(expr, Expr::Cast(_, _, _)));
    }
}
