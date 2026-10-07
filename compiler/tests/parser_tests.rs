use juyu_compiler::syntax::ast::*;
use juyu_compiler::syntax::lexer::Lexer;
use juyu_compiler::syntax::parser::Parser;

fn parse(input: &str) -> Result<SourceFile, String> {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    parser.parse_file()
}

#[test]
fn test_parse_new_syntax() {
    let ast = parse("export fn main(): !void { let x := 5; }");
    assert!(ast.is_ok());
}

#[test]
fn test_parse_nested_generics() {
    let ast = parse("fn main() { let x: Vec<Vec<T>> = 0; }");
    assert!(ast.is_ok(), "{:?}", ast.err());
}

#[test]
fn test_parse_as_cast() {
    let ast = parse("fn main() { let x = 5 as u64; }");
    assert!(ast.is_ok());
}

#[test]
fn test_parse_postfix_try() {
    let ast = parse("fn main() { let x = foo()!; }");
    assert!(ast.is_ok());
}

#[test]
fn parse_control_transfers() {
    let ast = parse("fn f() { loop { break; } while (true) { continue; } return 4; }").unwrap();
    let Item::Function(f) = &ast.items[0] else {
        panic!()
    };
    let Some(FunctionBody::Block(b)) = &f.body else {
        panic!()
    };
    assert!(matches!(
        &b.stmts[2],
        Stmt::Expr(Expr::Return(Some(_), _), _)
    ));
    assert!(parse("fn f() { return; }").is_ok());
    assert!(parse("fn f() { break }").is_err());
}

#[test]
fn bare_conditions_and_else_if_are_statement_blocks() {
    let ast = parse("fn f(flag:bool) { if flag {} else if (false) {} while flag {} }").unwrap();
    let Item::Function(f) = &ast.items[0] else {
        panic!()
    };
    let Some(FunctionBody::Block(b)) = &f.body else {
        panic!()
    };
    let Stmt::Expr(Expr::If(_, _, Some(no), _), _) = &b.stmts[0] else {
        panic!()
    };
    assert!(no.yield_expr.is_none());
    assert!(matches!(no.stmts[0], Stmt::Expr(Expr::If(..), _)));
}
