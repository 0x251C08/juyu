use juyu_compiler::{
    check_source, parse_source,
    sema::{self, Type},
};

fn good(source: &str) {
    check_source(source).unwrap_or_else(|e| panic!("{source}\n{e}"));
}
fn bad(source: &str, code: &str, message: &str) {
    let ast = parse_source(source).unwrap();
    let e = sema::analyze(&ast).unwrap_err();
    assert_eq!(e.code, code, "{e}");
    assert!(e.message.contains(message), "{e}");
    assert!(e.span.line > 0 && e.span.col > 0);
}

#[test]
fn forward_calls_recursion_and_labels() {
    good("fn first(): i32 => later(n: 4); fn later(n: i32): i32 { if (n == 0) { return 1; } return n * later(n - 1); }");
}
#[test]
fn scalar_types_literals_casts_and_operators() {
    for (ty, value) in [
        ("i8", "-128"),
        ("i16", "-32768"),
        ("i32", "-2147483648"),
        ("i64", "-9223372036854775808"),
        ("u8", "255"),
        ("u16", "65535"),
        ("u32", "4294967295"),
        ("u64", "18446744073709551615"),
        ("f32", "1.25"),
        ("f64", "2.5"),
        ("bool", "true"),
        ("str", "\"hello\""),
    ] {
        good(&format!(
            "fn f(): {ty} {{ let x: {ty} = {value}; return x; }}"
        ));
    }
    good("fn f(x: i64): bool { let y = 1 + x; let z = x + 1; let small = y as i32; return (y == z) && !(small < 0); }");
    good("fn f(x: i32): i32 { var y = -x; y += 2; y -= 1; y *= 3; y /= 2; return ~y % 3; }");
}
#[test]
fn lexical_scopes_and_shadowing() {
    let p = check_source("fn f(x: i32): i32 { { let x = true; } return x; }").unwrap();
    assert_eq!(p.functions()[0].result, Type::I32);
    good("fn f() { let x = 1; { let x = x + 1; } }");
    bad(
        "fn f() { { let inner = 1; } inner; }",
        "E2001",
        "undefined identifier",
    );
    bad("fn f() { let x = x; }", "E2001", "undefined identifier");
}
#[test]
fn undefined_names_and_functions() {
    bad("fn f() { missing; }", "E2001", "undefined identifier");
    bad("fn f() { missing(); }", "E2001", "undefined function");
    bad("fn f() { let f = 1; f(); }", "E2007", "not callable");
}
#[test]
fn duplicates() {
    bad("fn f() {} fn f() {}", "E2004", "duplicate declaration");
    bad("fn f(x:i32, x:i32) {}", "E2004", "duplicate declaration");
    bad(
        "fn f(x:i32) { let x = 2; }",
        "E2004",
        "duplicate declaration",
    );
    bad(
        "fn f() { let x = 1; var x = 2; }",
        "E2004",
        "duplicate declaration",
    );
}
#[test]
fn call_validation() {
    bad(
        "fn f(x:i32) {} fn g() { f(); }",
        "E2005",
        "expects 1 arguments",
    );
    bad(
        "fn f(x:i32) {} fn g() { f(1, 2); }",
        "E2005",
        "expects 1 arguments",
    );
    bad(
        "fn f(x:i32) {} fn g() { f(true); }",
        "E2003",
        "expected i32, got bool",
    );
    bad(
        "fn f(x:i32) {} fn g() { f(y: 2); }",
        "E2005",
        "argument label",
    );
}
#[test]
fn assignments_and_conditions() {
    bad("fn f() { let x = 1; x = 2; }", "E2007", "immutable");
    bad("fn f(x: i32) { x += 1; }", "E2007", "immutable");
    bad("fn f() { var x = 1; x = true; }", "E2003", "expected i32");
    bad("fn f() { let x: bool = 1; }", "E2003", "expected bool");
    bad("fn f() { if (1) {} }", "E2003", "expected bool");
    bad("fn f() { while (1) {} }", "E2003", "expected bool");
}
#[test]
fn returns_and_reachable_fallthrough() {
    good("fn f(x: bool): i32 { if (x) { return 1; } else { return 2; } }");
    good("fn f(): i32 { loop { continue; } }");
    good("fn f(): i32 { while (true) { return 3; } }");
    good("fn f(): i32 { loop { loop { break; } return 3; } }");
    good("fn f() { return; }");
    bad(
        "fn f(): i32 { return true; }",
        "E2009",
        "return type mismatch",
    );
    bad("fn f() { return 1; }", "E2009", "return type mismatch");
    bad("fn f(): i32 { return; }", "E2009", "requires a i32 value");
    bad(
        "fn f(x:bool): i32 { if (x) { return 1; } }",
        "E2009",
        "may finish",
    );
    bad("fn f(): i32 { loop { break; } }", "E2009", "may finish");
    bad(
        "fn f(): i32 { while (false) { return 1; } }",
        "E2009",
        "may finish",
    );
    bad(
        "fn f(): i32 { return 1; missing; }",
        "E2001",
        "undefined identifier",
    );
}
#[test]
fn loop_transfers() {
    good("fn f() { var n = 0; while (n < 10) { n += 1; if (n == 3) { continue; } if (n == 8) { break; } } loop { break; } }");
    bad("fn f() { break; }", "E2010", "outside a loop");
    bad("fn f() { continue; }", "E2010", "outside a loop");
    bad(
        "fn f() { loop { break; } continue; }",
        "E2010",
        "outside a loop",
    );
}
#[test]
fn numeric_range_and_explicit_conversions() {
    bad("fn f() { let x: u8 = 256; }", "E2003", "out of range");
    bad("fn f() { let x: u8 = -1; }", "E2003", "out of range");
    bad("fn f() { let x: i8 = 128; }", "E2003", "out of range");
    good("fn f(): i8 => -128i8;");
    bad(
        "fn f() { let x: i32 = 1i64; }",
        "E2003",
        "expected i32, got i64",
    );
    bad("fn f(x:i32) { let y: i64 = x; }", "E2003", "expected i64");
    bad("fn f() { let x = true + false; }", "E2007", "operand types");
    bad("fn f() { let x = 1 && 2; }", "E2003", "expected bool");
    bad(
        "fn f() { let x = true as i32; }",
        "E2007",
        "casts require numeric",
    );
}
#[test]
fn unsupported_constructs_fail_closed() {
    bad("fn f(): !void {}", "E2011", "unsupported bootstrap type");
    bad("struct X { x: i32, }", "E2011", "top-level");
    bad("fn f() { defer f(); }", "E2011", "defer");
    bad("fn f() { let x = null; }", "E2011", "expression");
    bad("fn f() { let x: i24 = 1; }", "E2002", "unsupported type");
    bad("fn f(x: void) {}", "E2003", "parameter");
    bad("fn f() { let x = f(); }", "E2003", "variable");
    bad("fn f() { let x = 1 +% 2; }", "E2011", "binary operator");
}
#[test]
fn entrypoint_and_diagnostic_spans() {
    good("export fn main(): i32 { return 7; }");
    good("export fn main() {}");
    bad("fn main(x:i32) {}", "E2009", "main must");
    bad("fn main(): bool => true;", "E2009", "main must");
    let ast = parse_source("fn f() {\n    missing;\n}").unwrap();
    let e = sema::analyze(&ast).unwrap_err();
    assert_eq!((e.span.line, e.span.col), (2, 5));
    assert_eq!(
        &"fn f() {\n    missing;\n}"[e.span.start..e.span.end],
        "missing"
    );
}

#[test]
fn bootstrap_runtime_signatures_are_checked() {
    good(r#"fn main() { std.io.println("Hello"); std.io.printlnInt(42); }"#);
    bad("fn main() { std.io.println(42); }", "E2003", "expected str");
    bad(
        "fn main() { std.io.printlnInt(true); }",
        "E2003",
        "expected i64",
    );
    bad(
        "fn main() { std.io.println(); }",
        "E2005",
        "expects 1 arguments",
    );
    bad(
        "fn main() { std.io.missing(); }",
        "E2001",
        "undefined function",
    );
    bad(
        r#"fn main() { let std = 1; std.io.println("hi"); }"#,
        "E2007",
        "not callable",
    );
}

#[test]
fn nested_comparison_hints_use_result_type() {
    good("fn f(n:i64): bool { return ((n < 3) == (n > 0)) && (!false == true); }");
    bad(
        "fn f() { let x = 1; } fn g() { x; }",
        "E2001",
        "undefined identifier",
    );
}
