use std::{
    env,
    ffi::OsString,
    process::{Command, ExitStatus},
};

/// CC is one executable path, never shell text. Otherwise prefer clang, then gcc.
pub fn c_compiler() -> Result<OsString, String> {
    if let Some(cc) = env::var_os("CC") {
        if cc.is_empty() {
            return Err("CC is empty; set it to a C compiler executable".into());
        }
        return Ok(cc);
    }
    for cc in ["clang", "gcc"] {
        if Command::new(cc)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            return Ok(cc.into());
        }
    }
    Err("no system clang or gcc found; install one or set CC to its executable path".into())
}

pub fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    1
}
