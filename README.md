# jett

**jettison the junk — a terminal disk space navigator and reclaimer**

https://github.com/user-attachments/assets/ad53fecf-0b14-413a-b1c1-9f2edf9a090b

## How does it work?

Given a path on your hard-drive (which could also be the root path, eg. `/`), `jett` scans it and indexes its metadata to memory so that you could explore its contents (even while still scanning!).

Once completed, you can navigate through subfolders, getting a visual treemap representation of what's taking up your disk space. You can even delete files or folders and `jett` will track how much space you've freed up in this session.

If you run it in `dry-run` mode, you will be able to get a list of items that would have been otherwise deleted. Great for relocating them to your backup destination or compressing them if applicable.

## Installation

### Prebuilt binaries (recommended)

With [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```
cargo binstall jett
```

Or download the `.tar.gz` archive for your platform — Linux and macOS, x86_64 and aarch64 — from the [releases](https://github.com/sdkks/jett/releases) page and put the `jett` binary on your `PATH`.

#### Static binaries for systems with an old or missing libc (NAS and friends)

The default Linux builds are dynamically linked against glibc and need a recent one. On machines with an old or missing libc — NAS appliances are the classic case — an error like ``GLIBC_2.xx not found`` means you want the fully static `musl` build, which runs on any Linux regardless of libc:

```
cargo binstall --target x86_64-unknown-linux-musl jett    # x86_64 (typical NAS)
cargo binstall --target aarch64-unknown-linux-musl jett   # ARM64
```

Add `--force` if you are replacing an already-installed version. The `musl` archives on the [releases](https://github.com/sdkks/jett/releases) page carry the same static binaries for direct download.

### From source

```
cargo install --git https://github.com/sdkks/jett
```

or, from a local clone:

```
cargo install --path .
```

## Supported platforms

`jett` targets linux, macos and windows.

## Usage

Either start `jett` in the folder you want to scan, or provide it with the folder you'd like to scan as an argument:

```
$ jett /home/user
```

Choose a color scheme with `--theme <name>` (for example, `--theme catppuccin-mocha`
or `--theme solarized-light`). Run `jett --help` for the complete list of 19 built-in
schemes plus `default`. Custom scheme names from your config are also accepted.
Without a flag or configured scheme, jett preserves the original appearance. Dark
and light schemes are explicit choices; jett does not detect your terminal's theme.

### Keyboard controls

Use the arrow keys (or `h`/`j`/`k`/`l`) to move, Enter to enter a folder, Esc to
visit its parent, `+`/`-`/`0` to zoom in/out/reset, Backspace to request deletion,
and `q` to quit. Deletion requires confirmation unless explicitly disabled.
After scanning finishes, press `t` to open the full-screen theme selector.

Type to filter built-in and custom scheme names; Up/Down previews the highlighted
scheme on the actual navigator without saving. Backspace edits the filter.
Enter saves the choice and returns to the navigator; Esc cancels and restores the
exact theme active before opening, including a `--theme` override. `q` also cancels
when the filter is empty, but is ordinary filter text otherwise. A failed save
restores the previous theme and shows a non-fatal bottom-line notice. The selector
never navigates folders or deletes files.

### Dry-run cleanup plans

Run `jett --dry-run /path/to/scan` to plan a cleanup without removing files.
Confirm deletions as usual: dry-run skips only filesystem removal, while the
file tree, treemap, and freed-space accounting update exactly as for a real
cleanup. The counter reads `would free:` instead of the usual `freed:`:
these bytes are hypothetical, not reclaimed. Cancelled requests do not count.

The bottom-right chip is always visible:

- Green `DRY-RUN ON`: files stay on disk, even with confirmation disabled.
- Amber `DRY-RUN OFF`: real deletion, with confirmation enabled.
- Red `DRY-RUN OFF + CONFIRM OFF`: real deletion without prompts because
  `--disable-delete-confirmation` was explicitly passed.

Dry-run does **not** bypass confirmation. Its prompts and progress messages
explicitly say that nothing will be deleted. When you quit, after the terminal
UI closes, jett prints a summary and, if at least one deletion was confirmed,
creates a private list file using `mktemp`:

```text
dry run: would have cleaned 3.0K across 2 items
list: /tmp/jett-dry-run.abcdefghij
```

With zero confirmed deletions, only the summary appears; no list file is created.
The list contains sorted (byte-wise), deduplicated, absolute paths, with exactly
one record per counted item. A folder is one record for the folder itself, not
an expansion of its contents. The temporary file remains available after exit;
remove it when finished. `mktemp` must be available on your system.

`--file-list-delim newline|nul|tab|pipe` selects the separator and terminates each
record; the default is `newline`. This option requires `--dry-run`. **Use `nul`
for scripting:** NUL is the only byte that can never appear inside a Unix
filename. Newlines, tabs, and pipes are all legal filename characters and can
make the other formats ambiguous.

```sh
jett --dry-run --file-list-delim nul /path/to/scan
# Copy the printed list path; inspect targets without splitting their names:
xargs -0 -r -I '{}' printf '%s\n' '{}' < /tmp/jett-dry-run.abcdefghij
# Or archive the planned targets (GNU tar accepts NUL-separated names):
tar --null --verbatim-files-from -T /tmp/jett-dry-run.abcdefghij -cf cleanup-plan.tar
```

The list is a plan, not a promise that disk contents will still be unchanged
when another tool uses it. Review targets before any later destructive action.

### Theme configuration

jett reads `config.toml` from:

- Linux, macOS, and other Unix systems: `$XDG_CONFIG_HOME/jett/config.toml` when
  `XDG_CONFIG_HOME` is nonempty, otherwise `$HOME/.config/jett/config.toml`.
- Windows: `%APPDATA%\jett\config.toml`.

The file is optional. An existing file must declare `version = 1`. For example:

```toml
version = 1

[theme]
scheme = "personal-light" # or a built-in such as "gruvbox-light"
# mode = "light"         # optional override for the selected scheme's mode

[theme.custom_schemes.personal-light]
mode = "light"           # optional; inferred from the name when omitted
foreground = "#202124"
title = "#abc"           # short hex expands to #aabbcc
accent = { rgb = [11, 87, 208] }
selected_file_fg = "black"
selected_file_bg = "#dbeafeff" # alpha is accepted but ignored
modal_surface = "white"
tile_border = "reset"
```

Selection order is **`--theme` > `[theme].scheme` > `default`**. A missing file
or missing `[theme]` section keeps the original appearance. Invalid TOML, unknown
schemes/keys, unsupported versions, and invalid colors are non-fatal when starting
the navigator: jett ignores the config, uses `default` (or an explicit built-in
`--theme` override), and shows a notice on the bottom status line. Run
`jett config validate` for the full problem description. Navigation and deletion
confirmation remain available.

Custom schemes start with the `dark` or `light` base palette; omitted roles retain
that base's values. `[theme].mode` wins over the custom table's `mode`; absent both,
a name containing `light` or `latte` (case-insensitive) means light, otherwise dark.
Built-ins retain their own mode unless `[theme].mode` overrides it; changing mode
alone does not recolor a built-in. Custom names cannot replace built-in names.

Colors accept the named Ratatui colors (`black`, `red`, `green`, `yellow`, `blue`,
`magenta`, `cyan`, `gray`, `darkgray`, `lightred`, `lightgreen`, `lightyellow`,
`lightblue`, `lightmagenta`, `lightcyan`, `white`), `none`/`reset` for terminal reset,
`#RGB`, `#RRGGBB`, `#RRGGBBAA` (alpha ignored), or `{ rgb = [r, g, b] }` with integer
channels from 0 to 255. Names are case-insensitive and allow spaces, hyphens, or
underscores; `grey` is also accepted.

Custom color roles are:

- Text/status: `foreground`, `title`, `title_separator`, `accent`, `success`,
  `error`, `warning`.
- Feedback: `path_error_fg`, `path_error_bg`, `freed_flash_fg`, `freed_flash_bg`.
- Modal/tile text: `modal_surface`, `modal_text`, `tile_text`, `tile_accent`.
- Selection: `selected_file_fg`, `selected_file_bg`, `selected_folder_bg`,
  `selected_folder_line1_fg`, `selected_folder_line2_fg`.
- Legend/empty folders: `legend_chip_fg`, `legend_chip_bg`, `empty_surface_fg`,
  `empty_surface_bg`.
- Tile composition seeds: `tile_fill`, `tile_border`, `tile_composition`,
  `tile_composition_track` (not all seeds paint a visible surface today).

When overriding a filled surface, choose readable foregrounds for it too; jett
preserves your custom colors rather than automatically adjusting contrast.

```sh
jett config set theme.scheme gruvbox-light
jett config set theme.scheme personal-light
jett config validate
jett --theme dracula /path/to/scan # override for this session only
```

`config set` accepts only a built-in name or a custom scheme already defined in
that file, creates missing parent directories, and **rewrites the file without
preserving comments or formatting**. It preserves other supported settings and
custom palettes. Saving from the theme selector uses the same rewrite behavior,
including removal of comments and formatting. A saved choice becomes active for
this session even if it began with `--theme`; the flag still overrides config on
subsequent launches. Invalid input is rejected before writing; a bad selected scheme
can be repaired with a valid `config set`. `config validate` checks every custom
scheme, even unused ones, and exits nonzero with a specific error on invalid config;
a missing file passes without creating one. Neither command starts the navigator.
A folder literally named `config` can be scanned as `jett ./config`.

## Running in a container

Build the image with Podman, then explore the container's temporary folder:

```sh
podman build -t jett .
podman run --rm -it jett /tmp
```

`jett` can delete files, so container isolation is the recommended way to try it; avoid mounting important host folders.

## Contributing

Contributions of any kind are very much welcome. If you think `jett` is cool and you'd like to hack at it, feel free to look through the issues. Take a look especially at ones marked "help wanted" or "good first issue".
Also, if you found a bug or have an idea for a new feature, please feel free to open an issue to discuss it.

For more detailed information, please see the CONTRIBUTING.md file at the root of this repository.

## License

MIT — see [LICENSE](LICENSE)
