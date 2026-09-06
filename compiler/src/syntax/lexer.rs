//! Hand-written fast recursive descent lexer for Juyu.

use crate::syntax::token::{Span, Token, TokenKind};

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let chars: Vec<(usize, char)> = source.char_indices().collect();
        Self {
            source,
            chars,
            cursor: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.cursor).map(|&(_, c)| c)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).map(|&(_, c)| c)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(&(_, c)) = self.chars.get(self.cursor) {
            self.cursor += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn current_pos(&self) -> usize {
        self.chars.get(self.cursor).map(|&(idx, _)| idx).unwrap_or(self.source.len())
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start_pos = self.current_pos();
        let start_line = self.line;
        let start_col = self.col;

        let ch = match self.advance() {
            Some(c) => c,
            None => return Token::new(TokenKind::Eof, Span::new(start_pos, start_pos, start_line, start_col)),
        };

        let kind = match ch {
            // Punctuation & Single/Multi-char Operators
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '~' => TokenKind::Tilde,
            '^' => TokenKind::Caret,

            '?' => {
                if self.peek() == Some('?') {
                    self.advance();
                    TokenKind::QuestionQuestion
                } else if self.peek() == Some('.') {
                    self.advance();
                    TokenKind::QuestionDot
                } else {
                    TokenKind::Question
                }
            }

            '.' => {
                if self.peek() == Some('*') {
                    self.advance();
                    TokenKind::DotStar
                } else if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        TokenKind::DotDotDot
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }

            '=' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::FatArrow
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }

            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::NotEq
                } else {
                    TokenKind::Bang
                }
            }

            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LtEq
                } else if self.peek() == Some('<') {
                    self.advance();
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }

            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GtEq
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }

            '+' => {
                if self.peek() == Some('%') {
                    self.advance();
                    TokenKind::PlusPercent
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }

            '-' => {
                if self.peek() == Some('%') {
                    self.advance();
                    TokenKind::MinusPercent
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }

            '*' => {
                if self.peek() == Some('%') {
                    self.advance();
                    TokenKind::StarPercent
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }

            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }

            '%' => TokenKind::Percent,

            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Ampersand
                }
            }

            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }

            // String Literals: Triple-quote """ or Single-line "
            '"' => self.lex_string(),

            // Interpolated String $"..."
            '$' => {
                if self.peek() == Some('"') {
                    self.advance();
                    self.lex_interpolated_string()
                } else {
                    TokenKind::Error("Unexpected '$' without string literal".to_string())
                }
            }

            // Character literal
            '\'' => self.lex_char(),

            // Compiler builtins '@builtin' or Escaped identifier '@"ident"'
            '@' => {
                if self.peek() == Some('"') {
                    self.advance(); // consume '"'
                    let raw = self.lex_raw_string_contents();
                    TokenKind::Ident(raw)
                } else if let Some(c) = self.peek() {
                    if c.is_ascii_alphabetic() || c == '_' {
                        let mut name = String::new();
                        while let Some(ch) = self.peek() {
                            if ch.is_ascii_alphanumeric() || ch == '_' {
                                name.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        TokenKind::Builtin(name)
                    } else {
                        TokenKind::Error("Expected builtin name after '@'".to_string())
                    }
                } else {
                    TokenKind::Error("Unexpected trailing '@'".to_string())
                }
            }

            // Numeric literals
            '0'..='9' => self.lex_number(ch),

            // Identifiers & Keywords
            'a'..='z' | 'A'..='Z' | '_' => self.lex_identifier(ch),

            other => TokenKind::Error(format!("Unexpected character '{}'", other)),
        };

        let end_pos = self.current_pos();
        Token::new(kind, Span::new(start_pos, end_pos, start_line, start_col))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            if self.peek() == Some('/') {
                if self.peek_next() == Some('/') {
                    // Line comment //
                    self.advance();
                    self.advance();
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            self.advance();
                            break;
                        }
                        self.advance();
                    }
                    continue;
                } else if self.peek_next() == Some('*') {
                    // Nestable block comment /* ... /* ... */ ... */
                    self.advance();
                    self.advance();
                    let mut depth = 1;
                    while depth > 0 {
                        if let Some(c) = self.advance() {
                            if c == '/' && self.peek() == Some('*') {
                                self.advance();
                                depth += 1;
                            } else if c == '*' && self.peek() == Some('/') {
                                self.advance();
                                depth -= 1;
                            }
                        } else {
                            break; // Unclosed block comment
                        }
                    }
                    continue;
                }
            }

            break;
        }
    }

    fn lex_string(&mut self) -> TokenKind {
        // Check for triple-quote multiline string """
        if self.peek() == Some('"') && self.peek_next() == Some('"') {
            self.advance();
            self.advance();
            return self.lex_multiline_string();
        }

        let mut content = String::new();
        while let Some(c) = self.advance() {
            match c {
                '"' => return TokenKind::StringLit(content),
                '\\' => {
                    if let Some(esc) = self.advance() {
                        match esc {
                            'n' => content.push('\n'),
                            'r' => content.push('\r'),
                            't' => content.push('\t'),
                            '0' => content.push('\0'),
                            '\\' => content.push('\\'),
                            '"' => content.push('"'),
                            other => content.push(other),
                        }
                    }
                }
                '\n' => return TokenKind::Error("Unterminated single-line string literal".to_string()),
                other => content.push(other),
            }
        }
        TokenKind::Error("Unclosed string literal".to_string())
    }

    fn lex_raw_string_contents(&mut self) -> String {
        let mut content = String::new();
        while let Some(c) = self.advance() {
            if c == '"' {
                break;
            }
            content.push(c);
        }
        content
    }

    fn lex_multiline_string(&mut self) -> TokenKind {
        let mut raw = String::new();
        while let Some(c) = self.advance() {
            if c == '"' && self.peek() == Some('"') && self.peek_next() == Some('"') {
                self.advance();
                self.advance();
                // Strip common leading indentation
                let stripped = Self::strip_common_indentation(&raw);
                return TokenKind::StringLit(stripped);
            }
            raw.push(c);
        }
        TokenKind::Error("Unclosed multiline string literal".to_string())
    }

    fn strip_common_indentation(s: &str) -> String {
        let lines: Vec<&str> = s.lines().collect();
        if lines.is_empty() {
            return String::new();
        }

        // Find min indent for non-empty lines:
        let min_indent = lines
            .iter()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
            .min()
            .unwrap_or(0);

        let mut result = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            if idx == 0 && line.trim().is_empty() {
                continue;
            }
            if line.len() >= min_indent {
                result.push(&line[min_indent..]);
            } else {
                result.push(line.trim_start());
            }
        }

        result.join("\n")
    }

    fn lex_interpolated_string(&mut self) -> TokenKind {
        let mut content = String::new();
        while let Some(c) = self.advance() {
            if c == '"' {
                return TokenKind::InterpolatedString(content);
            }
            content.push(c);
        }
        TokenKind::Error("Unclosed interpolated string literal".to_string())
    }

    fn lex_char(&mut self) -> TokenKind {
        let c = match self.advance() {
            Some('\\') => match self.advance() {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('0') => '\0',
                Some('\'') => '\'',
                Some(other) => other,
                None => return TokenKind::Error("Unclosed escape in char literal".to_string()),
            },
            Some(ch) => ch,
            None => return TokenKind::Error("Unclosed char literal".to_string()),
        };

        if self.advance() != Some('\'') {
            return TokenKind::Error("Expected closing quote for char literal".to_string());
        }

        TokenKind::CharLit(c)
    }

    fn lex_number(&mut self, first_digit: char) -> TokenKind {
        let mut num_str = String::from(first_digit);
        let mut is_float = false;

        // Check for 0x (hex) or 0b (binary)
        if first_digit == '0' {
            if let Some('x') | Some('X') = self.peek() {
                num_str.push(self.advance().unwrap());
                while let Some(c) = self.peek() {
                    if c.is_ascii_hexdigit() || c == '_' {
                        if c != '_' { num_str.push(c); }
                        self.advance();
                    } else {
                        break;
                    }
                }
                let val = i128::from_str_radix(&num_str[2..], 16).unwrap_or(0);
                return TokenKind::Int(val, None);
            } else if let Some('b') | Some('B') = self.peek() {
                num_str.push(self.advance().unwrap());
                while let Some(c) = self.peek() {
                    if c == '0' || c == '1' || c == '_' {
                        if c != '_' { num_str.push(c); }
                        self.advance();
                    } else {
                        break;
                    }
                }
                let val = i128::from_str_radix(&num_str[2..], 2).unwrap_or(0);
                return TokenKind::Int(val, None);
            }
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' { num_str.push(c); }
                self.advance();
            } else if c == '.' && self.peek_next().map(|p| p.is_ascii_digit()).unwrap_or(false) {
                is_float = true;
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Check for bit-width or float suffix: e.g. u8, i32, u24, f32
        let mut suffix = None;
        if let Some(c) = self.peek() {
            if c == 'u' || c == 'i' || c == 'f' {
                let mut s = String::new();
                while let Some(sc) = self.peek() {
                    if sc.is_ascii_alphanumeric() {
                        s.push(sc);
                        self.advance();
                    } else {
                        break;
                    }
                }
                suffix = Some(s);
            }
        }

        if is_float {
            let val: f64 = num_str.parse().unwrap_or(0.0);
            TokenKind::Float(val)
        } else {
            let val: i128 = num_str.parse().unwrap_or(0);
            TokenKind::Int(val, suffix)
        }
    }

    fn lex_identifier(&mut self, first_char: char) -> TokenKind {
        let mut ident = String::from(first_char);
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        match ident.as_str() {
            "let" => TokenKind::Let,
            "var" => TokenKind::Var,
            "fn" => TokenKind::Fn,
            "struct" => TokenKind::Struct,
            "packed" => TokenKind::Packed,
            "extern" => TokenKind::Extern,
            "interface" => TokenKind::Interface,
            "extend" => TokenKind::Extend,
            "pub" => TokenKind::Pub,
            "static" => TokenKind::Static,
            "inline" => TokenKind::Inline,
            "noinline" => TokenKind::Noinline,
            "const" => TokenKind::Const,
            "naked" => TokenKind::Naked,
            "match" => TokenKind::Match,
            "loop" => TokenKind::Loop,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "defer" => TokenKind::Defer,
            "errdefer" => TokenKind::Errdefer,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "null" => TokenKind::Null,
            "undefined" => TokenKind::Undefined,
            "unreachable" => TokenKind::Unreachable,
            "type" => TokenKind::Type,
            "distinct" => TokenKind::Distinct,
            "test" => TokenKind::Test,
            "import" => TokenKind::Import,
            "enum" => TokenKind::Enum,
            "union" => TokenKind::Union,
            "true" => TokenKind::BoolLit(true),
            "false" => TokenKind::BoolLit(false),
            _ => TokenKind::Ident(ident),
        }
    }
}
