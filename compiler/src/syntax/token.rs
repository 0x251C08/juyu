//! Token definitions, keyword mappings, and source spans for Juyu.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self { start, end, line, col }
    }

    pub fn dummy() -> Self {
        Self { start: 0, end: 0, line: 1, col: 1 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Let,
    Var,
    Fn,
    Struct,
    Packed,
    Extern,
    Interface,
    Extend,
    Pub,
    Static,
    Inline,
    Noinline,
    Const,
    Naked,
    Match,
    Loop,
    While,
    For,
    In,
    If,
    Else,
    Break,
    Continue,
    Defer,
    Errdefer,
    Try,
    Catch,
    Null,
    Undefined,
    Unreachable,
    Type,
    Distinct,
    Test,
    Import,
    Enum,
    Union,

    // Literals
    Int(i128, Option<String>), // value, optional bit-width suffix (e.g. "u8", "i24")
    Float(f64),
    StringLit(String),
    InterpolatedString(String),
    CharLit(char),
    BoolLit(bool),

    // Identifiers & Builtins
    Ident(String),
    Builtin(String), // e.g. @cast, @sizeOf, @cImport, @assert

    // Arithmetic & Wrapping Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPercent,   // +%
    MinusPercent,  // -%
    StarPercent,   // *%

    // Bitwise Operators
    Ampersand,     // &
    Pipe,          // |
    Caret,         // ^
    Tilde,         // ~
    Shl,           // <<
    Shr,           // >>

    // Logical Operators
    AmpAmp,        // &&
    PipePipe,      // ||
    Bang,          // !

    // Comparison Operators
    EqEq,          // ==
    NotEq,         // !=
    Lt,            // <
    LtEq,          // <=
    Gt,            // >
    GtEq,          // >=

    // Assignment Operators
    Eq,            // =
    PlusEq,        // +=
    MinusEq,       // -=
    StarEq,        // *=
    SlashEq,       // /=

    // Control & Punctuation
    FatArrow,          // =>
    QuestionQuestion,  // ??
    QuestionDot,       // ?.
    DotStar,           // .*
    Dot,               // .
    DotDot,            // ..
    DotDotDot,         // ...
    Colon,             // :
    Semicolon,         // ;
    Comma,             // ,
    Question,          // ?

    // Delimiters
    OpenParen,     // (
    CloseParen,    // )
    OpenBrace,     // {
    CloseBrace,    // }
    OpenBracket,   // [
    CloseBracket,  // ]

    // Comments & Special
    DocComment(String),
    Eof,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
