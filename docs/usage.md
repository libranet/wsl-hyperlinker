# How to use

## Wrapper mode (recommended)

Put `wsl-hyperlinker` in front of any command that writes file links:

```bash
wsl-hyperlinker ls --hyperlink=auto -l
wsl-hyperlinker lsd -l                  # with `hyperlink: auto` in lsd's config.yaml
wsl-hyperlinker eza -l --hyperlink
```

Then **Ctrl + click** a file or folder name in Windows Terminal:

- folders open in Windows Explorer;
- files open in the Windows program set as default for that file type;
- files under `/mnt/c/...` open through their normal Windows path (`C:\...`).

The command still believes it writes to a terminal, so options such as `--color=auto` and
`--hyperlink=auto` work as usual. You don't need to force colours or links.

In pipes and redirects the command runs unchanged, with no link codes in the output:

```bash
wsl-hyperlinker ls -l | grep foo       # plain text
wsl-hyperlinker ls -l > listing.txt    # plain text
```

## Filter mode

Pipe output that already contains links through `wsl-hyperlinker`:

```bash
ls --hyperlink=always --color=always | wsl-hyperlinker
```

The command writes to a pipe here, so you must force links and colours yourself
(`always`). Prefer wrapper mode where possible.

## Shell functions

Wrap your everyday commands once, in `~/.bashrc`:

```bash
unalias ls 2>/dev/null
function ls { wsl-hyperlinker ls --color=auto --hyperlink=auto "$@"; }

unalias ll 2>/dev/null
function ll { wsl-hyperlinker lsd --almost-all --group-directories-first --long "$@"; }
```

Inside `wsl-hyperlinker`, `ls` means the real `/usr/bin/ls`, not the shell function, so the function
doesn't call itself. To bypass the function, use `command ls` or `\ls`.

## Help

```bash
wsl-hyperlinker --help
```

## What gets rewritten

| Link written by the tool | Link after wsl-hyperlinker |
|---|---|
| `file:///home/me/notes.md` | `file://wsl.localhost/<distro>/home/me/notes.md` |
| `file://<hostname>/home/me/notes.md` | `file://wsl.localhost/<distro>/home/me/notes.md` |
| `file:///mnt/c/Users/me` | `file:///C:/Users/me` |
| `file://otherhost/...`, `https://...` | unchanged |

## Checking a link

- Hover over a link in Windows Terminal: it is underlined, and a tooltip shows the target.
- Right-click a link and choose **Copy link** to see the full address.
- Show the raw escape codes:

  ```bash
  ls --hyperlink=always -d ~ | wsl-hyperlinker | cat -v
  ```

## Exit codes

| Code | Meaning |
|---|---|
| the command's own code | the command ran |
| 127 | command not found |
| 126 | command could not be started |
| 128 + N | command was killed by signal N (130 = Ctrl+C) |

## Behaviour outside WSL

When `WSL_DISTRO_NAME` is not set, `wsl-hyperlinker` runs the command unchanged (wrapper mode) or
copies input to output unchanged (filter mode). The same shell setup can be used on
non-WSL machines.

## Don'ts

- Don't run full-screen programs through it (`less`, `vim`, `htop`, `top`). Use it for
  commands that print output and exit.
- Double-click does not open links: Windows Terminal always uses Ctrl + click, and double-click
  selects a word.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| Ctrl + click does nothing | Check the link target (hover or `cat -v`). If it starts with `file:///home`, the output did not go through `wsl-hyperlinker`. |
| No links at all | The tool isn't writing them: add `--hyperlink=auto` (ls, eza) or set `hyperlink: auto` (lsd). |
| No colours in wrapper mode | The tool may not be using `auto`. Add `--color=auto`. |
| `defining function based on alias` in zsh | Add `unalias <name> 2>/dev/null` before the function, and use the `function name { ... }` form. |
| Windows shows a security prompt on the first click | This can happen for `\\wsl.localhost` paths; allow it. |
