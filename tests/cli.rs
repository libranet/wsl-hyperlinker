//! End-to-end tests that run the `wsl-hyperlinker` binary.

use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::{Command, Output, Stdio};

/// Path to the binary under test, built by Cargo.
const BIN: &str = env!("CARGO_BIN_EXE_wsl-hyperlinker");

/// Runs the binary with `args`, feeding `stdin`; `distro` sets or clears `WSL_DISTRO_NAME`.
fn run(args: &[&OsStr], distro: Option<&str>, stdin: &str) -> Output {
    let mut cmd = Command::new(BIN);
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    match distro {
        Some(d) => cmd.env("WSL_DISTRO_NAME", d),
        None => cmd.env_remove("WSL_DISTRO_NAME"),
    };
    let mut child = cmd.spawn().unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

const LINK: &str = "\x1b]8;;file:///tmp\x07tmp\x1b]8;;\x07";

#[test]
fn filter_rewrites_on_wsl() {
    let out = run(&[], Some("Distro"), LINK);
    assert!(out.status.success());
    assert_eq!(out.stdout, b"\x1b]8;;file://wsl.localhost/Distro/tmp\x07tmp\x1b]8;;\x07");
}

#[test]
fn filter_passes_through_outside_wsl() {
    let out = run(&[], None, LINK);
    assert!(out.status.success());
    assert_eq!(out.stdout, LINK.as_bytes());
}

#[test]
fn help() {
    let out = run(&[OsStr::new("--help")], None, "");
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().starts_with("wsl-hyperlinker:"));
}

#[test]
fn missing_command_exits_127() {
    let out = run(&[OsStr::new("wsl-hyperlinker-no-such-command")], Some("Distro"), "");
    assert_eq!(out.status.code(), Some(127)); // command not found
}

#[test]
fn exit_code_passed_through() {
    let out = run(&[OsStr::new("sh"), OsStr::new("-c"), OsStr::new("exit 3")], Some("Distro"), "");
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn non_utf8_argument() {
    let out = run(&[OsStr::new("true"), OsStr::from_bytes(b"\xff")], Some("Distro"), "");
    assert!(out.status.success(), "{out:?}");
}
