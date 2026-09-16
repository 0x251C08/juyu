use juyu_compiler::syntax::lexer::Lexer;
use juyu_compiler::syntax::token::{TokenKind};

fn lex(source: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::Eof {
            break;
        }
        tokens.push(token.kind);
    }
    tokens
}

#[test]
fn test_new_tokens() {
    let tokens = lex("let x := 5; export fn bitand bitor bitxor");
    assert!(tokens.contains(&TokenKind::ColonEq));
    assert!(tokens.contains(&TokenKind::Export));
    assert!(tokens.contains(&TokenKind::BitAnd));
    assert!(tokens.contains(&TokenKind::BitOr));
    assert!(tokens.contains(&TokenKind::BitXor));
}
