use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "juyu cli {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn source(&self, source: &str) -> PathBuf {
        let path = self.0.join("program.ju");
        fs::write(&path, source).unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn cli(command: &str, path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_juyu"))
        .arg(command)
        .arg(path)
        .env_remove("CC")
        .output()
        .unwrap()
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn check_build_and_run_all_examples() {
    for (name, expected) in [
        ("hello", "Hello from Juyu!\n"),
        ("math", "square(7):\n49\nfactorial(6):\n720\n"),
        ("control", "sum(1, 2, 4, 5, 6) * 2:\n36\n"),
    ] {
        let dir = Scratch::new();
        let source = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../examples/bootstrap/{name}.ju")),
        )
        .unwrap();
        let path = dir.source(&source);
        success(&cli("check", &path));
        assert!(!path.with_extension("").exists());
        success(&cli("build", &path));
        assert!(path.with_extension("").is_file());
        assert!(path.with_extension("c").is_file());
        let output = Command::new(path.with_extension("")).output().unwrap();
        success(&output);
        assert_eq!(output.stdout, expected.as_bytes());
        let output = cli("run", &path);
        success(&output);
        assert_eq!(output.stdout, expected.as_bytes());
    }
}
#[test]
fn invalid_programs_fail_check_build_and_run() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../compiler/tests/fixtures/invalid");
    for entry in fs::read_dir(fixtures).unwrap() {
        let source = fs::read_to_string(entry.unwrap().path()).unwrap();
        let dir = Scratch::new();
        let path = dir.source(&source);
        for command in ["check", "build", "run"] {
            let output = cli(command, &path);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("error[E2"));
        }
        assert!(!path.with_extension("").exists());
    }
}
#[test]
fn run_propagates_exit_code_and_stale_binary_is_not_run() {
    let dir = Scratch::new();
    let path = dir.source("fn main():i32 { return 37; }");
    assert_eq!(cli("run", &path).status.code(), Some(37));
    fs::write(&path, "fn main() { missing; }").unwrap();
    assert_eq!(cli("run", &path).status.code(), Some(1));
    // The previous successful artifact is preserved, but never executed by run.
    assert_eq!(
        Command::new(path.with_extension(""))
            .status()
            .unwrap()
            .code(),
        Some(37)
    );
}
#[test]
fn missing_source_syntax_error_and_missing_entrypoint() {
    let dir = Scratch::new();
    assert!(!cli("build", &dir.0.join("absent.ju")).status.success());
    let path = dir.source("fn main( {");
    assert!(!cli("check", &path).status.success());
    let path = dir.source("fn f() {}");
    success(&cli("check", &path));
    let output = cli("build", &path);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing main"));
}
#[test]
fn compiler_invocation_failure_is_reported() {
    let dir = Scratch::new();
    let path = dir.source("fn main() {}");
    let output = Command::new(env!("CARGO_BIN_EXE_juyu"))
        .arg("build")
        .arg(path)
        .env("CC", dir.0.join("no-such-compiler"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("could not execute C compiler"));
}
#[cfg(unix)]
#[test]
fn compiler_failure_and_false_success_are_not_build_success() {
    use std::os::unix::fs::PermissionsExt;
    for (script, diagnostic) in [
        (
            "#!/bin/sh\necho compiler-failure >&2\nexit 9\n",
            "C compiler failed",
        ),
        ("#!/bin/sh\nexit 0\n", "produced no executable"),
    ] {
        let dir = Scratch::new();
        let path = dir.source("fn main() {}");
        let cc = dir.0.join("fake cc");
        fs::write(&cc, script).unwrap();
        fs::set_permissions(&cc, fs::Permissions::from_mode(0o755)).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_juyu"))
            .arg("run")
            .arg(&path)
            .env("CC", cc)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(diagnostic));
        assert!(!path.with_extension("").exists());
    }
}
#[test]
fn non_generated_c_is_preserved_and_stubs_fail_honestly() {
    let dir = Scratch::new();
    let path = dir.source("fn main() {}");
    fs::write(path.with_extension("c"), "user-owned C source").unwrap();
    assert!(!cli("build", &path).status.success());
    assert_eq!(
        fs::read_to_string(path.with_extension("c")).unwrap(),
        "user-owned C source"
    );
    for command in ["test", "fmt"] {
        let output = cli(command, &path);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("not implemented"));
    }
}

#[cfg(unix)]
#[test]
fn source_symlinks_cannot_overwrite_extensionless_input() {
    let dir = Scratch::new();
    let actual = dir.0.join("source");
    let source = "fn main() {}";
    fs::write(&actual, source).unwrap();
    let link = dir.0.join("program.ju");
    std::os::unix::fs::symlink(&actual, &link).unwrap();
    let output = cli("build", &link);
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(actual).unwrap(), source);
}
