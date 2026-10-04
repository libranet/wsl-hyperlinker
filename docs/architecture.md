# Architecture

This page explains how the code is organised and how one run of `wsl-hyperlinker` flows through it.
It is written for readers who are new to Rust. The Rust ideas are explained where the code uses
them, in boxes like this one:

!!! info "Rust: what is a crate?"
    A **crate** is Rust's unit of compilation: a library or a program. A **package** is a folder
    with a `Cargo.toml` that builds one or more crates. **Cargo** is the build tool and package
    manager: `cargo build`, `cargo test`, `cargo run`.

## The big picture

```text
                 +----------- wsl-hyperlinker ------------+
 ls, lsd, eza -->| pseudo-terminal --> Rewriter --> stdout |--> Windows Terminal
  (child)        |  (sys.rs)          (rewrite.rs)         |
                 +-----------------------------------------+
```

1. `main.rs` reads the arguments and decides which **mode** to run in.
2. In wrapper mode, `run.rs` starts the command with its output on a **pseudo-terminal**
   (created in `sys.rs`), so the command still thinks it is writing to a real terminal.
3. Everything the command writes is fed through the **`Rewriter`** (`rewrite.rs`), which
   rewrites `file://` links.
4. The result goes to our real stdout, which is Windows Terminal.

## Project layout

```text
wsl-hyperlinker/
+-- Cargo.toml            package manifest: name, version, dependencies, lints
+-- Cargo.lock            exact dependency versions (generated; commit it)
+-- .cargo/config.toml    Cargo settings for this project: build output goes to var/target/
+-- rustfmt.toml          code formatter settings
+-- src/
|   +-- main.rs           the program: argument handling, picks a mode
|   +-- lib.rs            the library: declares the modules below
|   +-- rewrite.rs        Rewriter: finds and rewrites links (pure logic, no I/O)
|   +-- run.rs            the modes: filter, wrapper, pass-through
|   +-- sys.rs            calls into the C library (libc): pty, host name, signals
+-- tests/
|   +-- rewrite.rs        tests for the Rewriter
|   +-- cli.rs            tests that run the finished program
+-- docs/                 this documentation (built with Zensical)
+-- justfile              project commands: `just test`, `just lint`, ...
+-- prek.toml             checks that run before each commit / push
+-- var/                  generated files (build output, docs site, caches); not in Git
```

## Library and program

The package builds **two crates** from `src/`:

| File | Crate | Role |
|---|---|---|
| `src/lib.rs` | library `wsl_hyperlinker` | All the real code. |
| `src/main.rs` | program `wsl-hyperlinker` | A thin entry point that calls the library. |

Cargo finds both by convention; `Cargo.toml` does not have to mention them. The program uses the
library like any other dependency:

```rust
use wsl_hyperlinker::{Rewriter, run};
```

!!! info "Rust: `wsl-hyperlinker` vs `wsl_hyperlinker`"
    A package name may contain `-`, but Rust identifiers may not. In code, the library is
    therefore called `wsl_hyperlinker`.

Why split it? The tests in `tests/` are separate crates. They can only use what a **library**
makes public, so moving the code into `lib.rs` is what makes it testable from outside.

## Modules

`lib.rs` declares the modules. Each `mod name;` line tells the compiler to load `src/name.rs`:

```rust
pub mod rewrite;   // public: usable as wsl_hyperlinker::rewrite
pub mod run;       // public: usable as wsl_hyperlinker::run
mod sys;           // private: only usable inside the library

pub use rewrite::Rewriter;   // shortcut: wsl_hyperlinker::Rewriter
```

!!! info "Rust: visibility"
    Everything in Rust is **private** by default. `pub` makes an item visible outside its
    module. `sys` is private on purpose: it contains the `unsafe` code, and nothing outside
    the library should call it directly.

| Module | Knows about | Does I/O? |
|---|---|---|
| `rewrite` | bytes and strings only | no |
| `run` | processes, stdin/stdout, `Rewriter`, `sys` | yes |
| `sys` | the operating system (via `libc`) | yes |

Keeping `rewrite` free of I/O makes it easy to test: give it bytes, check the bytes it returns.

## One run, step by step

### 1. `main.rs`: choose a mode

```rust
let args: Vec<OsString> = env::args_os().skip(1).collect();
```

`args_os()` returns the command-line arguments; `.skip(1)` drops the program's own name.

!!! info "Rust: `String` vs `OsString`"
    A Rust `String` is always valid UTF-8 text. File names on Linux are just bytes and need not
    be valid UTF-8. `OsString` holds such raw OS strings. Using `env::args()` instead would make
    the program crash on a file name like `caf\xe9.txt`.

Then a `match` picks the mode:

```rust
match args.split_first() {
    None if on_wsl => run::filter(...),                                      // `cmd | wsl-hyperlinker`
    None => run::passthrough(),                                              // not on WSL
    Some((cmd, args)) if on_wsl && io::stdout().is_terminal() => run::wrap(...),
    Some((cmd, args)) => run::exec(cmd, args),                               // pipe or not WSL
}
```

!!! info "Rust: `Option` and `match`"
    Rust has no `null`. A value that may be missing has type `Option<T>`: either `Some(value)` or
    `None`. `split_first()` returns `None` for an empty list, or `Some((first, rest))`.
    `match` must handle every case; the compiler refuses to build if one is missing.
    `if ...` after a pattern is a **guard**: an extra condition for that arm.

| Arguments | On WSL | stdout is a terminal | Mode |
|---|---|---|---|
| none | yes | - | `filter`: rewrite stdin to stdout |
| none | no | - | `passthrough`: copy stdin unchanged |
| command | yes | yes | `wrap`: run it on a pty and rewrite |
| command | otherwise | | `exec`: become the command, no rewriting |

`main` returns an `ExitCode`, which becomes the program's exit status.

### 2. `run::wrap`: start the command on a pseudo-terminal

```rust
let Ok((master, slave)) = sys::open_pty() else {
    return exec(cmd, args);
};
```

A **pseudo-terminal (pty)** is a pair of connected file handles. Whatever is written to the
*slave* end can be read from the *master* end, and to the program writing, the slave looks
like a real terminal. The command gets the slave as its stdout; we read from the master.

!!! info "Rust: `Result`, and `let ... else`"
    Operations that can fail return `Result<T, E>`: either `Ok(value)` or `Err(error)`.
    `let Ok(x) = f() else { ... };` means: if `f()` succeeded, bind the value to `x`;
    otherwise run the `else` block, which must leave the function (here: `return`).

```rust
let mut child = match Command::new(cmd).args(args).stdout(Stdio::from(slave)).spawn() { ... };
sys::ignore_interrupts();
pump(File::from(master), rw);
child.wait().map_or(ExitCode::FAILURE, exit_code)
```

- `Command` is the standard library's way to start a process.
- `Stdio::from(slave)` **moves** the slave into the command: after this line we no longer
  own it (see [Ownership](#ownership) below).
- `ignore_interrupts` makes Ctrl+C stop only the command, not us, so we can still copy
  its last output.
- `pump` copies everything until the command closes its output.
- `exit_code` turns the child's exit status into ours; a child killed by signal N gives
  128 + N, as in a shell.

### 3. `run::pump`: copy and rewrite

```rust
fn pump(mut input: impl Read, mut rw: Rewriter) {
    ...
    loop {
        let n = match input.read(&mut buf) { ... };
        let out = rw.feed(&buf[..n]);
        stdout.write_all(&out) ...
    }
    stdout.write_all(&rw.finish());
}
```

!!! info "Rust: traits and `impl Read`"
    A **trait** is a set of methods a type promises to have, like an interface in other
    languages. `Read` is the trait for "something you can read bytes from". `input: impl Read`
    accepts any such type: a file, stdin, the pty master. The same `pump` therefore serves
    both filter mode (stdin) and wrapper mode (pty).

### 4. `Rewriter`: find and rewrite links

A terminal hyperlink is an escape sequence:

```text
ESC ] 8 ; params ; URI  ST   visible text   ESC ] 8 ; ; ST
+- OSC 8 +                                   (closes the link)
```

`ST`, the terminator, is either `BEL` (byte 7) or `ESC \`.

The `Rewriter` searches for `ESC ] 8 ;`, finds the terminator, and passes the URI to
`rewrite_uri`:

```rust
fn rewrite_uri(&self, uri: &str) -> Option<String>
```

It returns `Some(new_uri)` to replace the URI, or `None` to leave it unchanged. The `?` operator
inside it is a shortcut: `uri.strip_prefix("file://")?` means "if this is `None`, return `None`
from the function right away". That is how non-`file://` links end up unchanged.

**Why the Rewriter keeps state.** Output arrives in chunks of whatever size the OS delivers, and
a link can be cut in half:

```text
chunk 1:  "abc \x1b]8;;file:///ho"
chunk 2:  "me/me\x07name\x1b]8;;\x07 def"
```

`feed` returns everything it can safely write, and keeps the unfinished part in
`self.pending` until the next chunk. `finish` releases whatever is left at the end.

```rust
pub struct Rewriter {
    distro: String,     // e.g. "Ubuntu-24.04", percent-encoded
    hostname: String,   // this machine's name, lowercase
    pending: Vec<u8>,   // bytes held back from the previous chunk
}
```

!!! info "Rust: `&self` and `&mut self`"
    Methods take `self` in one of three ways:
    `&self` (read-only access, like `rewrite_uri`),
    `&mut self` (may change the fields, like `feed`, which updates `pending`),
    or `self` (takes ownership, and the value is used up).

### 5. `sys`: talking to the operating system

Opening a pty, reading the host name and ignoring signals are not in Rust's standard library.
`sys.rs` calls the C library through the [`libc`](https://docs.rs/libc) crate, the project's
only dependency.

!!! warning "Rust: `unsafe`"
    The compiler cannot check C functions: they take raw pointers and may write anywhere.
    Calling them requires an `unsafe { ... }` block, in which **you** promise the call is
    correct. This project keeps every `unsafe` block in `sys.rs`, keeps each one small, and
    explains above it, in a `// SAFETY:` comment, why it is correct. A lint in `Cargo.toml`
    (`undocumented_unsafe_blocks`) fails the build if such a comment is missing.

The functions in `sys.rs` are **safe wrappers**: the rest of the code calls `sys::open_pty()`
like any other function and never sees a raw pointer.

## Ownership

Rust's central rule: every value has exactly **one owner**, and the value is cleaned up when
its owner goes away (goes "out of scope"). There is no garbage collector.

In this project, that shows best with file handles:

```rust
pub fn open_pty() -> io::Result<(OwnedFd, OwnedFd)>
```

An `OwnedFd` is an open file descriptor that **closes itself** when it is dropped. Nobody has
to remember to call `close()`:

- `Stdio::from(slave)` moves the slave into the `Command`. The `Command` is dropped right after
  `spawn()`, which closes our copy of the slave. That matters: the master only reports
  end-of-output once **every** copy of the slave is closed.
- `File::from(master)` moves the master into a `File` for reading; it is closed when `pump`
  finishes.

!!! info "Rust: borrowing"
    Instead of moving a value, you can **borrow** it with `&` (read-only) or `&mut` (writable).
    `rw.feed(&buf[..n])` lends `feed` a view of the first `n` bytes of `buf`, without copying
    them. `&[u8]` is a borrowed slice of bytes; `Vec<u8>` is an owned, growable list of bytes.
    Likewise `&str` is borrowed text and `String` owned text.

## Tests

| Where | What | Run with |
|---|---|---|
| `tests/rewrite.rs` | `Rewriter` cases: Linux paths, Windows drives, links split over chunks, ... | `cargo test` |
| `tests/cli.rs` | Runs the built program: filter mode, `--help`, exit codes, non-UTF-8 arguments | `cargo test` |
| `///` examples in `src/` | Code examples in doc comments are compiled and run as tests | `cargo test` |

A test is a function marked `#[test]`. It passes if it returns normally and fails if it
**panics**, for example when `assert_eq!(a, b)` finds `a != b`.

Wrapper mode needs a real terminal, which automated tests do not have. Check it by hand with
`just run ls --hyperlink=auto`.

!!! info "Rust: attributes and doc comments"
    `#[...]` above an item is an **attribute**: an instruction to the compiler. `#[test]` marks a
    test, `#[derive(Debug)]` generates code to print a value, `#[must_use]` warns when a
    caller ignores the return value.
    `///` is a **doc comment** for the item below it; `//!` documents the whole file or module.
    Run `cargo doc --open` to see them as a website.

## Tooling

| Task | Command | Configured in |
|---|---|---|
| List all project commands | `just` | `justfile` |
| Build / run | `just build`, `just run <command>` | `Cargo.toml` |
| Release build, linked as `bin/wsl-hyperlinker` | `just release` | `justfile` |
| Format the code | `just fmt` | `rustfmt.toml` |
| Lint (clippy) | `just lint` | `[lints]` in `Cargo.toml` |
| All checks: format, lint, test, docs | `just check` | `justfile` |
| Checks before commit / push | automatic after `just setup` | `prek.toml` |

**Clippy** is Rust's linter. It finds bugs and non-idiomatic code, and usually explains how to
fix it. The project enables the stricter `pedantic` group and treats every warning as an error
in `just lint`.

## Making a change: an example

Suppose links to `/tmp` should open the Windows temp folder instead:

1. **Write a test first** in `tests/rewrite.rs`, next to `windows_drives`:

    ```rust
    assert_eq!(run(&link("file:///tmp/x", BEL)), link("file:///C:/Temp/x", BEL));
    ```

2. `just test` shows it failing.
3. Add the rule to `rewrite_uri` in `src/rewrite.rs`, before the `wsl.localhost` fallback.
4. `just test` passes; `just check` runs everything else.
5. Update the rewrite table in [Analysis](analysis.md#design).

## Further reading

- [The Rust Programming Language](https://doc.rust-lang.org/book/): the official book, free online.
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/): the same topics as short examples.
- [Standard library docs](https://doc.rust-lang.org/std/): look up `Option`, `Result`, `Command`, `OwnedFd`.
- [The Cargo Book](https://doc.rust-lang.org/cargo/): packages, `Cargo.toml`, tests.
