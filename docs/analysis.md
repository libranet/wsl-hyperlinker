# Analysis: why terminal file links don't open from WSL

## The problem

Tools like `ls`, `lsd` and `eza` can turn file names into clickable links, using the
[OSC 8 hyperlink escape sequence](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda).
In Windows Terminal you open such a link with **Ctrl + click**.

On WSL, clicking a file link from these tools does nothing, or shows an error. The prompt from
oh-my-posh on the same machine *does* open a link in Windows Explorer.

## What the tools actually write

The link target sits inside the escape sequence, hidden from view:

```
ESC ] 8 ; <params> ; <uri> ESC \   <visible text>   ESC ] 8 ; ; ESC \
```

Measured on WSL (`MYPC` stands for the machine's hostname, `me` for the user):

| Tool            | URI it writes                                   | Opens from Windows Terminal? |
|-----------------|-------------------------------------------------|------------------------------|
| lsd 1.2.0       | `file:///home/me/.config/lsd/config.yaml`       | no                           |
| eza 0.23.5      | `file:///home/me/.config/lsd/config.yaml`       | no                           |
| GNU ls 9.5      | `file://MYPC/home/me/...`                       | no                           |
| oh-my-posh 31.3 | `file://wsl.localhost/<distro>/home/me/...`     | **yes**                      |

## Why they fail

Windows Terminal hands the link to Windows (the shell's `ShellExecute`), which knows nothing
about the Linux file system:

- `file:///home/...` has no host, so Windows reads it as a local Windows path:
  `C:\home\...`, which doesn't exist.
- `file://MYPC/home/...` follows the OSC 8 spec: the hostname is the machine's
  name. But WSL shares its hostname with Windows, so Windows looks for a network share
  `\\MYPC\home`, which doesn't exist either.
- `file://wsl.localhost/<distro>/...` maps to the UNC path `\\wsl.localhost\<distro>\...`,
  which is how Windows reaches WSL files. This is the form oh-my-posh uses, and it works.

Files on Windows drives, mounted in WSL under `/mnt/c/...`, are best opened as a native Windows
path: `file:///C:/...`.

## Where it should be fixed

This is not really a bug in any one tool:

- The tools write valid links for a Linux terminal.
- The terminal is the component that knows it is bridging Linux and Windows, so the path
  translation belongs there.

Upstream issues:

- **Windows Terminal** - [microsoft/terminal#14116](https://github.com/microsoft/terminal/issues/14116)
  "Improve `file://` hyperlinks support - local/remote hostname". Open since October 2022,
  priority 3.
- **lsd** - [lsd-rs/lsd#930](https://github.com/lsd-rs/lsd/issues/930) "Include hostname in the
  hyperlink". Closed as completed in October 2023, but lsd 1.2.0 still writes no hostname.
  Even with a hostname, the link would not open on WSL (see GNU ls above).

Neither `lsd` nor `ls` has an option to change the link format.

## Options considered

| Option | Result |
|---|---|
| Use `wslview` to open links | Not possible: Windows Terminal handles the click on the Windows side; no Linux program is called. |
| Configure lsd / ls | No setting exists for the link format. |
| Per-command `sed` in an alias | Works, but needs `--color=always --hyperlink=always` because the tool now writes to a pipe, and must be repeated for every tool. |
| **A generic wrapper (this program)** | Works for any tool. Runs the command in a pseudo-terminal so `auto` colour and link detection keep working. Adds ~1 ms per command. |

## Design

`wsl-hyperlinker` has two modes.

**Wrapper mode** - `wsl-hyperlinker <command> [args...]`

1. If stdout is not a terminal, or not running on WSL, the command is run directly (`exec`), so
   pipes and redirects get clean output.
2. Otherwise a pseudo-terminal is opened with the same window size as the real terminal. The
   command's **stdout** is connected to it; stdin and stderr stay on the real terminal.
   Because stdout is a terminal, the command keeps its `auto` colours and links.
3. Output post-processing (`OPOST`) is turned off on the pseudo-terminal, so line endings pass
   through unchanged and the real terminal handles them once.
4. The output is read in chunks, rewritten, and written to the real stdout.
5. Ctrl+C reaches the command (same process group); `wsl-hyperlinker` ignores it so the remaining
   output is still written. The command's exit code is returned (127 = not found,
   128 + N = killed by signal N).

**Filter mode** - `<command> | wsl-hyperlinker`

Reads stdin, rewrites, writes stdout. The command must then force links itself
(for example `--hyperlink=always`), because it is writing to a pipe.

**Rewrite rules**

| Input URI | Output URI |
|---|---|
| `file:///path` | `file://wsl.localhost/<distro>/path` |
| `file://localhost/path` | `file://wsl.localhost/<distro>/path` |
| `file://<this hostname>/path` (case-insensitive) | `file://wsl.localhost/<distro>/path` |
| `file:///mnt/<drive>/path` | `file:///<DRIVE>:/path` |
| `file://<other host>/...` | unchanged |
| `http://`, `https://`, anything else | unchanged |

`<distro>` comes from the `WSL_DISTRO_NAME` environment variable, which WSL sets.
Both string terminators, `ESC \` and `BEL`, are supported, and link parameters (such as
`id=...`) are kept.

**Streaming**

A link sequence can be split across two reads. An incomplete sequence (or a trailing partial
`ESC ] 8 ;`) is held back until the rest arrives. A sequence without a terminator is released
unchanged once it passes 16 KiB, so broken input cannot stall the output.

## Performance

Average of 200 runs, output to a terminal:

| Command | Alone | With wsl-hyperlinker |
|---|---:|---:|
| `ls -l` | 1.9 ms | 2.7 ms |
| `lsd -l` | 10.8 ms | 12.5 ms |

## Limitations

- Full-screen programs (`less`, `vim`, `htop`) should not be run through `wsl-hyperlinker`: only
  their stdout is on the pseudo-terminal, and output is filtered. Use it for commands that
  print output and exit.
- If the terminal is resized while a command runs, the command is not told.
- Only OSC 8 links are rewritten. Paths that the terminal detects in plain text are not.
