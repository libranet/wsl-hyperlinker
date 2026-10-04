# Changelog

All notable changes to `wsl-hyperlinker` are listed here, newest first.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 - unreleased

First public release.

### Added

- Rewrite OSC 8 file hyperlinks so **Ctrl + click** in Windows Terminal opens them from WSL:
  `file:///home/...` and `file://<hostname>/home/...` become `file://wsl.localhost/<distro>/home/...`.
- Open files on Windows drives through their native path: `file:///mnt/c/...` becomes `file:///C:/...`.
- Leave links to other hosts and non-file links unchanged.
- Wrapper mode, `wsl-hyperlinker <command> [args...]`: runs the command in a pseudo-terminal,
  so `--color=auto` and `--hyperlink=auto` keep working. In pipes and redirects the command runs
  unchanged.
- Filter mode, `<command> | wsl-hyperlinker`: rewrites links in output that already contains them.
- Exit codes follow the shell: the command's own code, 127 when the command is not found, 126 when
  it cannot be started, and 128 + N when it is killed by signal N.
- Outside WSL (no `WSL_DISTRO_NAME`), commands and input pass through unchanged.
- Documentation on [Read the Docs](https://wsl-hyperlinker.readthedocs.io/): analysis,
  installation, usage and architecture.
- Continuous integration on GitHub Actions: prek hooks (formatting, clippy, docs build), and
  tests on stable Rust and on Rust 1.87, the minimum supported version.
- Licensed under MIT OR Apache-2.0.
