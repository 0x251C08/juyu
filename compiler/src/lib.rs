//! The Juyu Language Compiler Core Library.

pub mod syntax;

pub use syntax::ast::SourceFile;
pub use syntax::lexer::Lexer;
pub use syntax::parser::Parser;

/// Parses a Juyu source code string into an Abstract Syntax Tree (AST).
pub fn parse_source(source: &str) -> Result<SourceFile, String> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    parser.parse_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_and_parse_simple_function() {
        let code = r#"
            export fn add(a: i32, b: i32): i32 {
                => a + b;
            }
        "#;
        let ast = parse_source(code).expect("Should parse add function successfully");
        assert_eq!(ast.items.len(), 1);
    }

    #[test]
    fn test_lex_and_parse_struct_and_methods() {
        let code = r#"
            export struct Point {
                x: f32,
                y: f32,

                export static fn init(x: f32, y: f32): Point {
                    => Point { .x = x, .y = y };
                }

                export fn length(self: *const Point): f32 {
                    => math.sqrt(val: self.x * self.x + self.y * self.y);
                }
            }
        "#;
        let ast = parse_source(code).expect("Should parse struct and methods successfully");
        assert_eq!(ast.items.len(), 1);
    }

    #[test]
    fn test_parse_if_let_and_defer() {
        let code = r#"
            export fn process(maybe_ptr: ?*i32) {
                defer io.println(msg: "Done");
                if (let active = maybe_ptr) {
                    active.* += 1;
                }
            }
        "#;
        let ast = parse_source(code).expect("Should parse if-let and defer successfully");
        assert_eq!(ast.items.len(), 1);
    }
}
