# Project conventions

## File names
- Use lowercase names (`readme.md`, not `README.md`).
- Exceptions: names a tool requires or strongly expects: `Cargo.toml`, `Cargo.lock`,
  `CLAUDE.md`, `AGENTS.md`, `LICENSE`, root `README.md`, `docs/index.md`.

## Directories
- Put tool caches under `var/cache/<tool>/`, not in dot-directories in the project root.
  If a tool can't be configured, symlink its cache directory there.

## TOML
- Sort top-level sections alphabetically, and keys alphabetically within each section.
- Don't reorder array elements or `[[array-of-tables]]` entries: their order has meaning.
- `Cargo.toml`: keep `[package]` first, with `name` and `version` at its top.

## justfile
- Separate recipes with 2 blank lines.
- Indent recipe bodies with 4 spaces.
- Give every recipe a group attribute, e.g. `[group("dev")]`.

## Config files
- Start with a comment naming the official reference and the syntax:
      # <tool> configuration for <project>.
      # Reference: <url>
      # Syntax: TOML (https://toml.io)
- Explain any code in an inline comment:
  `#ffffff  # white`, `0o755  # rwxr-xr-x`, `127  # command not found`.
- Separate an inline comment from the code by exactly 2 spaces; don't align comments in columns.
