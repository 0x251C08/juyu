//! Validated IR. Symbol IDs encode binding decisions once, before code generation.
//! Program storage is crate-private: consumers cannot construct unchecked programs.
use super::Type;
use crate::syntax::ast::{BinaryOp, UnaryOp};

#[derive(Debug)]
pub struct Program {
    pub(crate) functions: Vec<Function>,
}
#[derive(Debug)]
pub struct Function {
    pub id: usize,
    pub name: String,
    pub params: Vec<(usize, Type)>,
    pub result: Type,
    pub body: Vec<Statement>,
}
#[derive(Debug)]
pub enum Statement {
    Declare(usize, Type, Expression),
    Assign(usize, Expression),
    Evaluate(Expression),
    Block(Vec<Statement>),
    If(Expression, Vec<Statement>, Vec<Statement>),
    While(Expression, Vec<Statement>),
    Loop(Vec<Statement>),
    Break,
    Continue,
    Return(Option<Expression>),
}
#[derive(Debug)]
pub struct Expression {
    pub ty: Type,
    pub kind: ExpressionKind,
}
#[derive(Debug)]
pub enum ExpressionKind {
    Int(i128),
    Float(f64),
    Bool(bool),
    String(String),
    Local(usize),
    Unary(UnaryOp, Box<Expression>),
    Binary(Box<Expression>, BinaryOp, Box<Expression>),
    Call(CallTarget, Vec<Expression>),
    Cast(Box<Expression>),
}

impl Program {
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CallTarget {
    User(usize),
    Runtime(usize),
}
