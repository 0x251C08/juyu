use juyu_compiler::{check_source, codegen::generate_c};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "juyu-codegen-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn native(source: &str) -> Output {
    let program = check_source(source).unwrap();
    let c = generate_c(&program).unwrap();
    let dir = Scratch::new();
    let source = dir.0.join("program.c");
    let binary = dir.0.join("program");
    fs::write(&source, &c).unwrap();
    let cc = ["clang", "gcc"]
        .into_iter()
        .find(|cc| Command::new(cc).arg("--version").output().is_ok())
        .expect("native codegen tests require clang or gcc");
    let output = Command::new(cc)
        .args([
            "-std=c11",
            "-O2",
            "-Wall",
            "-Werror=implicit-function-declaration",
            "-fsanitize=undefined",
            "-fno-sanitize-recover=all",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{c}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(binary.is_file());
    Command::new(binary).current_dir(&dir.0).output().unwrap()
}
fn exits(source: &str, code: i32) {
    let output = native(source);
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn native_forward_recursion_and_math() {
    exits("fn main(): i32 { return fact(5) - 120; } fn fact(n:i32):i32 { if (n <= 1) { return 1; } return n * fact(n - 1); }", 0);
}
#[test]
fn native_control_shadowing_and_transfers() {
    exits("fn main(): i32 { var n = 0; var sum = 0; while (n < 10) { n += 1; if (n == 3) { continue; } if (n == 7) { break; } sum += n; } { let sum = 200; } loop { sum -= 1; break; } return sum - 17; }", 0);
}
#[test]
fn native_short_circuit_skips_bad_arithmetic() {
    exits("fn bad(): bool { let z = 0; let n = 1 / z; return n == 0; } fn main(): i32 { let a = false && bad(); let b = true || bad(); if (!a && b) { return 0; } return 1; }", 0);
}
#[test]
fn c_identifiers_cannot_collide_or_inject() {
    let source = "fn @\"switch\"(@\"x-y\":i32):i32 => @\"x-y\"; fn main():i32 { let @\"/* evil */\" = 9; return @\"switch\"(@\"/* evil */\") - 9; }";
    let c = generate_c(&check_source(source).unwrap()).unwrap();
    assert!(!c.contains("evil"));
    assert!(!c.contains("x-y"));
    exits(source, 0);
}
#[test]
fn strings_have_byte_lengths_and_value_equality() {
    exits("fn echo(s:str):str => s; fn main():i32 { let a = \"é\\0Z\"; let b = echo(\"é\\0Z\"); if (a == b && a != \"é\") { return 0; } return 1; }", 0);
}
#[test]
fn native_all_scalar_widths_and_casts() {
    for (ty, literal) in [
        ("i8", "-128"),
        ("i16", "-32768"),
        ("i32", "-2147483648"),
        ("i64", "-9223372036854775808"),
        ("u8", "255"),
        ("u16", "65535"),
        ("u32", "4294967295"),
        ("u64", "18446744073709551615"),
    ] {
        exits(&format!("fn main():i32 {{ let x:{ty} = {literal}; let y:{ty} = {literal}; if (x == y) {{ return 0; }} return 1; }}"), 0);
    }
    exits("fn main():i32 { let a:f32 = 1.25; let b:f64 = a as f64; let c = (b * 4.0) as i32; return c - 5; }", 0);
    exits("fn main():i32 { let x:u8 = 255; let y:i64 = x as i64; let z = ~y; return (-z - 256) as i32; }", 0);
}
#[test]
fn checked_arithmetic_and_casts_trap() {
    for code in [
        "let a:i8 = 127; let b = a + 1;",
        "let a:u8 = 255; let b = a + 1;",
        "let a:u64 = 0; let b = a - 1;",
        "let a:i64 = 9223372036854775807; let b = a * 2;",
        "let a = 0; let b = 1 / a;",
        "let a:i64 = -9223372036854775808; let b = a / -1;",
        "let a:i8 = -128; let b = -a;",
        "let a = 256; let b = a as u8;",
        "let a:u64 = 18446744073709551615; let b = a as i64;",
        "let a:f64 = 18446744073709551616.0; let b = a as u64;",
    ] {
        let output = native(&format!("fn main() {{ {code} }}"));
        assert!(!output.status.success(), "{code}");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("runtime error:"),
            "unexpected C UB: {code}"
        );
    }
}
#[test]
fn entrypoint_abi_and_missing_main() {
    exits("fn main() {}", 0);
    exits("fn main(): i32 => 37;", 37);
    exits("fn done() {} fn main() { return done(); }", 0);
    assert!(generate_c(&check_source("fn f() {}").unwrap())
        .unwrap_err()
        .contains("missing main"));
}

#[test]
fn runtime_io_uses_normal_typed_calls() {
    let output = native(
        r#"
        fn greeting():str => "Hello from Juyu!";
        fn print(s:str, n:i64) { std.io.println(msg: s); std.io.printlnInt(val: n); }
        fn main() { print(greeting(), 42); std.io.println("A\0B é"); std.io.printlnInt(-9223372036854775808); }
    "#,
    );
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"Hello from Juyu!\n42\nA\0B \xc3\xa9\n-9223372036854775808\n"
    );
}
#[test]
fn call_argument_order_is_left_to_right() {
    let output = native(
        r#"
        fn arg(n:i64):i64 { std.io.printlnInt(n); return n; }
        fn take(a:i64,b:i64) { std.io.printlnInt(a + b); }
        fn main() { take(arg(1),arg(2)); }
    "#,
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"1\n2\n3\n");
}

#[test]
fn ordinary_else_if_and_unparenthesized_conditions() {
    exits("fn f(flag:bool):i32 { if flag { return 1; } else if (!flag) { return 2; } else { return 3; } } fn main():i32 { var flag = true; while flag { flag = false; } return f(flag) - 2; }", 0);
}
#[test]
fn floating_special_values_cannot_trigger_undefined_integer_casts() {
    for source in [
        "let n = 0.0 / 0.0; let x = n as i32;",
        "let n = 1.0 / 0.0; let x = n as i64;",
        "let n = -1.0; let x = n as u32;",
    ] {
        let output = native(&format!("fn main() {{ {source} }}"));
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("runtime error:"));
    }
}

#[test]
fn nested_comparisons_lower_as_booleans() {
    exits("fn f(n:i64):bool => (n < 3) == (n > 0); fn main():i32 { if (f(2) && (!false == true)) { return 0; } return 1; }", 0);
}
