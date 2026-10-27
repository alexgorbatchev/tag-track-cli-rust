---
created_on: 2026-10-14 10:03
last_modified: 2026-10-14 10:11
status: current
---

## Commands

| Usage | Behavior |
| :--- | :--- |
| `tag-track [FILE...]` | Identify and tag audio; default output is each source directory. |
| `tag-track track update [FILE...]` | Same full identification/tag workflow. |
| `tag-track track inspect [FILE...]` | Read tags, technical audio properties and cover dimensions. |
| `tag-track artwork embed [FILE...] --artwork IMAGE` | Embed a local image or HTTP(S) image while preserving existing tags. |
| `tag-track artwork embed [FILE...] --url URL` | Download and embed HTTP(S) artwork. Exactly one of `--url` or `--artwork` is required. |
| `tag-track dependencies` | Check both audio tools. |
| `tag-track dependencies check` | Check installed ffmpeg and ffprobe against minimum 4.4. |
| `tag-track dependencies install [DEPENDENCY...]` | Install named `ffmpeg`/`ffprobe`, or missing/outdated tools when names are omitted. |
| `tag-track dependencies update [DEPENDENCY...]` | Update named `ffmpeg`/`ffprobe`, or both when names are omitted. |
| `tag-track self update` | Update from an explicitly configured published Rust release channel. |
| `tag-track skill` | Print the Skills CLI installation command and canonical source skill URL verbatim, offline. |
| `tag-track help [COMMAND...]` | Print help for an optional command path. |
| `tag-track track help [COMMAND...]` | Print track command help. |
| `tag-track artwork help [COMMAND...]` | Print artwork command help. |
| `tag-track dependencies help [COMMAND...]` | Print dependency command help. |
| `tag-track self help [COMMAND...]` | Print self command help. |

`[FILE]...` is a native filesystem path, repeatable. `[DEPENDENCY]...` is repeatable string, accepted values `ffmpeg`, `ffprobe`. `COMMAND` is an optional variadic command path. No files plus nonterminal stdin reads lines until EOF; no paths after parsing prints root help. `~` and `~/` expand using the platform home directory. Input is bounded to 8 MiB total and 64 KiB per nonempty line, UTF-8. If any result-prefixed paths occur, other log/plain lines are ignored. Empty result lines are ignored.

## Global flags

| Flag | Type/default | Meaning |
| :--- | :--- | :--- |
| `--version` | bool, false | Print raw version and newline. |
| `--verbose`, `-v` | bool, false | Print processing paths and metadata providers on stderr; disable the spinner. |
| `--auto-install` | bool, false | Authorize installing missing/outdated audio tools. |
| `--help`, `-h` | bool, false | Print command help. |

## Root and `track update` flags

| Flag | Type/default | Meaning |
| :--- | :--- | :--- |
| `--out-dir DIRECTORY`, `-o` | path, unset | Output directory; effective default each source directory. Relative paths use the current working directory. |
| `--in-place`, `-i` | bool, false | Replace source after the tagged output is committed; takes precedence over out-dir. |
| `--refresh` | bool, false | Bypass all caches and identify again. |
| `--no-cache` | bool, false | Bypass all caches. |
| `--dry-run` | bool, false | Resolve/preview metadata, skip artwork and audio writes. |
| `--artist VALUE` | string, unset | Override artist and identification hint. |
| `--title VALUE` | string, unset | Override title and identification hint. |
| `--album VALUE` | string, unset | Override album. |
| `--date VALUE` | string, unset | Override release date, YYYY or valid YYYY-MM-DD. |
| `--genre VALUE` | string, unset | Override genre. |
| `--comment VALUE` | string, unset | Override generated provenance comment. |
| `--source-url VALUE` | string, unset | Record the source in provenance. |
| `--artwork VALUE` | string, unset | Local image path or HTTP(S) image URL. |
| `--progress-target URI` | string, unset | NDJSON target: stdout, stderr, fd://N, unix://PATH or tcp://HOST:PORT. |
| `--progress-socket PATH` | string, unset | Unix socket target if progress-target is absent. |

## `artwork embed` flags

`--artwork IMAGE` is string, unset: local image or HTTP(S) URL. `--url URL` is string, unset: HTTP(S) URL with a host. These flags are mutually exclusive and one is required. `--out-dir DIRECTORY`, `-o` is path, unset: each source directory effectively. `--in-place`, `-i` is bool, false: commit output then replace source, ignoring out-dir. Global flags and `--help`, `-h` apply.
