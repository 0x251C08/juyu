//! Thin CLI dispatcher; semantic analysis and code generation live in the library.
mod pipeline;
mod toolchain;

use std::{
    env,
    io::{self, Write},
    path::Path,
    process::{self, Command},
};

fn main() {
    match dispatch(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}
fn dispatch(args: &[String]) -> Result<i32, String> {
    let Some(command) = args.first() else {
        return repl();
    };
    match command.as_str() {
        "check" | "build" | "run" => {
            if args.len() != 2 {
                return Err(format!("Usage: juyu {command} <file.ju>"));
            }
            let path = Path::new(&args[1]);
            match command.as_str() {
                "check" => {
                    pipeline::check(path)?;
                    println!("Checked {}", path.display());
                }
                "build" => {
                    let binary = pipeline::build(path)?;
                    println!("Built {}", binary.display());
                }
                "run" => {
                    let binary = pipeline::build(path)?;
                    let status = Command::new(&binary)
                        .status()
                        .map_err(|e| format!("could not execute {}: {e}", binary.display()))?;
                    return Ok(toolchain::exit_code(status));
                }
                _ => unreachable!(),
            }
            Ok(0)
        }
        "test" | "fmt" => Err(format!(
            "juyu {command} is not implemented in this bootstrap"
        )),
        "cc" => {
            let cc = toolchain::c_compiler()?;
            let status = Command::new(cc)
                .args(&args[1..])
                .status()
                .map_err(|e| format!("could not execute C compiler: {e}"))?;
            Ok(toolchain::exit_code(status))
        }
        "codebase" => {
            let status = Command::new("rg")
                .args(&args[1..])
                .status()
                .map_err(|e| format!("could not execute system rg: {e}"))?;
            Ok(toolchain::exit_code(status))
        }
        "repl" => repl(),
        "version" | "--version" | "-v" => {
            println!("Juyu bootstrap 0.1.0 (C backend)");
            Ok(0)
        }
        "help" | "--help" | "-h" => {
            println!("Usage: juyu <command> [arguments]\n\ncheck <file.ju>  Parse and typecheck\nbuild <file.ju>  Compile to C and a native executable\nrun <file.ju>    Build and execute; propagate exit status\ncc [args]       Invoke system C compiler (or CC executable)\ncodebase [args] Invoke system rg\nrepl            Inspect parsed ASTs (no evaluation)\nversion         Show compiler version\n\ntest and fmt are not implemented.");
            Ok(0)
        }
        _ => Err(format!("unknown command '{command}'; use juyu --help")),
    }
}
fn repl() -> Result<i32, String> {
    println!("Juyu bootstrap syntax REPL (AST inspection only). Type exit to quit.");
    let mut line = String::new();
    loop {
        print!("juyu> ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        line.clear();
        if io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            return Ok(0);
        }
        if matches!(line.trim(), "exit" | "quit") {
            return Ok(0);
        }
        if line.trim().is_empty() {
            continue;
        }
        match juyu_compiler::parse_source(&line) {
            Ok(ast) => println!("{ast:?}"),
            Err(error) => eprintln!("{error}"),
        }
    }
}
