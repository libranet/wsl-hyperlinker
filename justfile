# Project tasks for wsl-hyperlinker. Run `just` to list them.
# Reference: https://just.systems/man/en/
# Syntax: justfile (https://just.systems/man/en/syntax.html)

# List available recipes, optionally filtered on group name with: "just list <group>"
[group("default")]
list group="all":
    @{{ just_executable() }} {{ if group == "" { "" } else if group == "all" { "" } else { "--group " + group } }} --justfile {{ justfile() }} --list --unsorted

alias ls := list

# Install the tools these recipes need and the Git hooks
[group("project")]
setup:
    uv tool install zensical
    uv tool install prek
    prek install

# Remove build output, bin/, the generated site and the docs cache
[group("project")]
clean:
    cargo clean
    rm -rf bin var/html var/cache/zensical

# Install the binary to $CARGO_HOME/bin
[group("install")]
install:
    cargo install --locked --path .

# Remove the installed binary
[group("install")]
uninstall:
    cargo uninstall wsl-hyperlinker

# Build a debug binary
[group("dev")]
build:
    cargo build

# Build an optimised binary, linked as bin/wsl-hyperlinker
[group("dev")]
release:
    cargo build --release
    mkdir -p bin
    ln -sfn ../var/target/release/wsl-hyperlinker bin/wsl-hyperlinker

# Run wsl-hyperlinker with arguments, e.g. `just run ls --hyperlink=auto -l`
[group("dev")]
run *args:
    cargo run --quiet -- {{ args }}

# Run the tests
[group("dev")]
test:
    cargo test

# Format the code
[group("dev")]
fmt:
    cargo fmt

# Check formatting without changing files
[group("dev")]
fmt-check:
    cargo fmt --check

# Run clippy with the lints from Cargo.toml, failing on warnings
[group("dev")]
lint:
    cargo clippy --all-targets -- -D warnings

# Everything CI should run: formatting, lints, tests, docs build
[group("dev")]
check: fmt-check lint test docs

# Build the documentation site into var/html/
[group("docs")]
docs: docs-cache
    zensical build --strict

# Serve the documentation with live reload on http://localhost:<port>
[group("docs")]
docs-serve port="8000": docs-cache
    zensical serve --dev-addr localhost:{{ port }}

# Zensical always writes its cache to .cache/ next to zensical.toml; point that at var/cache/zensical.
# (Don't use `zensical build --clean`: it deletes the link target and then fails on the link.)
[group("docs")]
[private]
docs-cache:
    mkdir -p var/cache/zensical
    [ -L .cache ] || { rm -rf .cache; ln -s var/cache/zensical .cache; }

# Run the pre-commit checks (prek.toml) on all files, not just staged ones
[group("dev")]
hooks:
    prek run --all-files
