//! Command-line entry point: picks a mode from the arguments and environment.
//! See the library crate for how links are rewritten.

use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal};
use std::process::ExitCode;

use wsl_hyperlinker::{Rewriter, run};

/// Text printed by `-h` / `--help`.
const HELP: &str = "\
wsl-hyperlinker: rewrite terminal hyperlinks so Windows Terminal can open them from WSL

Usage:
  wsl-hyperlinker <command> [args...]   run command, rewrite its hyperlinks
  <command> | wsl-hyperlinker           rewrite hyperlinks in a stream

Outside WSL, or when stdout is not a terminal, commands run unchanged.

Examples:
  wsl-hyperlinker lsd -l
  wsl-hyperlinker ls --hyperlink=auto -l
  wsl-hyperlinker eza -l --hyperlink
";

fn main() -> ExitCode {
    // args_os: file names need not be valid UTF-8.
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    if matches!(args.first().and_then(|a| a.to_str()), Some("-h" | "--help")) {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }

    // WSL sets this in every session; unset or empty means we are not on WSL.
    let distro = env::var("WSL_DISTRO_NAME").unwrap_or_default();
    let on_wsl = !distro.is_empty();

    match args.split_first() {
        None if on_wsl => run::filter(Rewriter::for_local_host(&distro)),
        None => run::passthrough(),
        Some((cmd, args)) if on_wsl && io::stdout().is_terminal() => {
            run::wrap(cmd, args, Rewriter::for_local_host(&distro))
        }
        Some((cmd, args)) => run::exec(cmd, args),
    }
}
