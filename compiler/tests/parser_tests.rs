use juyu_compiler::syntax::lexer::Lexer;
use juyu_compiler::syntax::parser::Parser;
use juyu_compiler::syntax::ast::*;

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
