# wsl-hyperlinker documentation

`wsl-hyperlinker` rewrites terminal hyperlinks (OSC 8) so that **Ctrl + click** on a file name in
Windows Terminal opens the file from WSL.

```bash
wsl-hyperlinker lsd -l        # Ctrl + click a name: folders open in Explorer, files in their default app
```

- [Analysis](analysis.md) - why file links from `ls`, `lsd` and `eza` don't open on WSL, the
  options considered, and how `wsl-hyperlinker` works.
- [Installation](installation.md) - build, install, shell setup.
- [How to use](usage.md) - wrapper and filter mode, shell functions, troubleshooting.
- [Architecture](architecture.md) - how the code is organised, explained for newcomers to Rust.
