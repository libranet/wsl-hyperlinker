//! The modes the binary runs in: filter, wrapper, and pass-through.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, ExitCode, ExitStatus, Stdio};

use crate::{Rewriter, sys};

/// Filter mode: copies stdin to stdout with hyperlinks rewritten.
#[must_use]
pub fn filter(rw: Rewriter) -> ExitCode {
    pump(io::stdin().lock(), rw);
    ExitCode::SUCCESS
}

/// Copies stdin to stdout unchanged (filter mode outside WSL).
#[must_use]
pub fn passthrough() -> ExitCode {
    let _ = io::copy(&mut io::stdin().lock(), &mut io::stdout().lock());
    ExitCode::SUCCESS
}

/// Replaces this process with `cmd args...`. Returns only if the command cannot be started.
#[must_use]
pub fn exec(cmd: &OsStr, args: &[OsString]) -> ExitCode {
    let err = Command::new(cmd).args(args).exec();
    command_error(cmd, &err)
}

/// Wrapper mode: runs `cmd args...` with its stdout on a pty (stdin and stderr stay on the real
/// terminal), copies its output to our stdout with hyperlinks rewritten, and returns its exit code.
///
/// The pty makes the command believe it writes to a terminal, so it keeps its colours and its
/// `--hyperlink=auto` links. Without a pty the command runs unchanged via [`exec`].
#[must_use]
pub fn wrap(cmd: &OsStr, args: &[OsString], rw: Rewriter) -> ExitCode {
    let Ok((master, slave)) = sys::open_pty() else {
        return exec(cmd, args);
    };
    let mut child = match Command::new(cmd).args(args).stdout(Stdio::from(slave)).spawn() {
        Ok(child) => child,
        Err(err) => return command_error(cmd, &err),
    };
    // After spawn, so the child keeps the default Ctrl+C behaviour.
    sys::ignore_interrupts();

    pump(File::from(master), rw);
    child.wait().map_or(ExitCode::FAILURE, exit_code)
}

/// Reports a command that could not be started, with the exit code a shell would use.
fn command_error(cmd: &OsStr, err: &io::Error) -> ExitCode {
    eprintln!("wsl-hyperlinker: {}: {err}", cmd.display());
    ExitCode::from(match err.kind() {
        ErrorKind::NotFound => 127, // command not found
        _ => 126,                   // found but not executable
    })
}

/// Converts a child's exit status to ours; like a shell, death by signal N becomes 128 + N.
fn exit_code(status: ExitStatus) -> ExitCode {
    let code = status.code().or_else(|| status.signal().map(|sig| 128 + sig));
    code.and_then(|c| u8::try_from(c).ok()).map_or(ExitCode::FAILURE, ExitCode::from)
}

/// Copies `input` to stdout through `rw` until EOF, or until stdout is closed.
///
/// A read error also ends the copy: on a pty master, EIO means the child closed its side.
fn pump(mut input: impl Read, mut rw: Rewriter) {
    let mut stdout = io::stdout().lock();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        let out = rw.feed(&buf[..n]);
        if stdout.write_all(&out).and_then(|()| stdout.flush()).is_err() {
            return; // broken pipe
        }
    }
    let _ = stdout.write_all(&rw.finish());
    let _ = stdout.flush();
}
