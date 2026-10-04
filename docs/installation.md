# Installation

## Requirements

- WSL 2 with any Linux distribution.
- A Rust toolchain (`cargo`), edition 2024, to build. Rust 1.87 or newer.
- A terminal that supports OSC 8 links: Windows Terminal, or the VS Code terminal.

Runtime dependencies: none besides the C library. The only build dependency is the
[`libc`](https://crates.io/crates/libc) crate.

## Build and install

```bash
cargo install --locked --git https://github.com/libranet/wsl-hyperlinker
```

Or from a clone of the repository:

```bash
git clone https://github.com/libranet/wsl-hyperlinker
cd wsl-hyperlinker
cargo install --locked --path .
```

This builds an optimised binary and copies it to `$CARGO_HOME/bin/wsl-hyperlinker`
(`~/.cargo/bin` by default). Make sure that directory is on your `PATH`:

```bash
command -v wsl-hyperlinker
```

To place the binary somewhere else instead:

```bash
cargo build --locked --release
install -m 755 var/target/release/wsl-hyperlinker ~/.local/bin/  # rwxr-xr-x; var/target is set in .cargo/config.toml
```

## Run the tests

```bash
cargo test
```

## Check that it works

1. Rewriting:

   ```bash
   ls --hyperlink=always -d ~ | wsl-hyperlinker | cat -v
   ```

   The output should contain `file://wsl.localhost/<your distro>/home/...`.

2. Clicking: in Windows Terminal, run

   ```bash
   wsl-hyperlinker ls --hyperlink=auto
   ```

   then Ctrl + click a folder name. It should open in Windows Explorer.

## Shell setup

Add wrappers to `~/.bashrc` (these also work in zsh):

```bash
# ls with Ctrl+click links that Windows can open
unalias ls 2>/dev/null
function ls {
    wsl-hyperlinker ls --color=auto --hyperlink=auto "$@"
}

# lsd long listing with Ctrl+click links
unalias ll 2>/dev/null
function ll {
    wsl-hyperlinker lsd --almost-all --group-directories-first --long "$@"
}
```

Use `unalias` and the `function name { ... }` form: zsh fails with
`defining function based on alias` if an alias of the same name already exists.

For lsd, use `auto` in `~/.config/lsd/config.yaml`, so links are only written to a terminal:

```yaml
hyperlink: auto
```

Open a new terminal to load the changes.

## Uninstall

```bash
cargo uninstall wsl-hyperlinker
```

Then remove the `ls` / `ll` functions from your shell configuration.
