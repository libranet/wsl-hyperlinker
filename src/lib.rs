//! Rewrite terminal hyperlinks (OSC 8) so Windows Terminal can open them from WSL.
//!
//! Linux tools write links as `file:///path` or `file://<hostname>/path`; Windows cannot open those.
//! They are rewritten to paths Windows understands:
//!
//! ```text
//! file:///home/me/x.txt  ->  file://wsl.localhost/<distro>/home/me/x.txt   (\\wsl.localhost\...)
//! file:///mnt/c/Users/x  ->  file:///C:/Users/x                            (native Windows path)
//! ```
//!
//! Links to other hosts, and non-file links (http, https, ...), are left unchanged.
//!
//! The crate is split into:
//! - [`rewrite`]: the streaming rewriter (pure, no I/O);
//! - [`run`]: the modes the binary runs in (filter, wrapper, pass-through);
//! - `sys`: the libc calls (pty, host name, signals); all `unsafe` code lives there.
//!
//! Related: <https://github.com/microsoft/terminal/issues/14116>

pub mod rewrite;
pub mod run;
mod sys;

pub use rewrite::Rewriter;
