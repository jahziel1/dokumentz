//! `dokumentz-script-check <esperado> <resultado>`: ver [`dokumentz_script::check::run_cli`].

use std::ffi::OsString;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let codigo =
        dokumentz_script::check::run_cli(&args, &mut std::io::stdout(), &mut std::io::stderr());
    ExitCode::from(u8::try_from(codigo).unwrap_or(2))
}
