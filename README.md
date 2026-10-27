`tag-track` identifies music, inspects existing tags and embeds square artwork for local audio collections. It writes tagged audio without transcoding and supports both terminal use and file pipelines.

# What It Does

- **Identify tracks:** Match audio fingerprints and filename hints with music metadata providers.
- **Edit tags safely:** Apply artist, title, album, release date, genre and provenance overrides to atomic output copies.
- **Embed artwork:** Normalize local or downloaded cover art to 1400×1400 JPEG while preserving other tags.
- **Inspect files:** Read audio properties, tags and artwork dimensions without modifying the file.

# How It Works

- Give `tag-track` one or more audio files, or pipe paths from `fetch-track`.
- Preview metadata with `--dry-run`, then save tagged copies with `--out-dir` or replace sources with `--in-place`.
- Use `artwork embed` when only the cover needs editing, and `track inspect` to check the result.

# How it Really Works

- Recognition sends audio fingerprints to Shazam and AcoustID, then uses iTunes and MusicBrainz metadata fallbacks. Filename hints retain remix descriptions; output names remove bracket descriptions and unsafe filename characters.
- MP3, FLAC, M4A, Ogg Vorbis and WAV audio packets are copied without transcoding. Output defaults to the source directory: when the generated name matches the source, that file is atomically updated. Use a separate `--out-dir` to retain the original. Distinct existing destinations are preserved; in-place source removal follows a successful atomic output commit.
- Metadata and artwork cache under `$XDG_CACHE_HOME/tag-track` or the platform cache directory. `--refresh` and `--no-cache` bypass caches. Dry runs identify tracks but skip image reads and audio writes.
- `--comment` supplies the provenance comment verbatim. Otherwise it records the source, metadata provider and tagging date. Artwork-only edits preserve other tags.
- Human output prints `DONE:` paths; `AGENT=1` prints compact fields. Optional NDJSON progress supports sockets, inherited descriptors, stdout and stderr. A failed batch exits nonzero and retains earlier successful outputs; cancellation exits 130.

# Installation

Rust release artifacts are unpublished. The original project's Go release archives do not contain this Rust executable. This independent local repository has no configured release channel.

# Setup

- [FFmpeg and FFprobe](https://ffmpeg.org/download.html) 4.4 or later provide audio decoding and inspection. Check with `tag-track dependencies check`; `dependencies install` explicitly installs missing tools through the supported system package manager.
- Run on Unix; Linux is the verified platform. Audio identification and remote artwork need network access to their providers.

# Quick Start

```sh
tag-track track inspect 'Music/Artist - Song.flac'
tag-track 'Music/Artist - Song.flac' --dry-run
tag-track 'Music/Artist - Song.flac' --out-dir tagged --comment 'My source note'
tag-track artwork embed 'Music/Artist - Song.flac' --artwork cover.png --in-place
```

Sample Output from a matching track:

```text
Matched: Artist - Song
DONE: tagged/Artist - Song.flac
```

# Options & Flags

Global options:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--version` | | `false` | Print the raw version. |
| `--verbose` | `-v` | `false` | Print processing paths and metadata providers on stderr; disable the spinner. |
| `--auto-install` | | `false` | Authorize installing missing or outdated audio tools. |
| `--help` | `-h` | `false` | Show command help. |

`tag-track [FILE...]` and `tag-track track update [FILE...]`:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--out-dir DIRECTORY` | `-o` | Source directory | Save tagged copies here. |
| `--in-place` | `-i` | `false` | Replace each source after committing output; overrides out-dir. |
| `--refresh` | | `false` | Bypass all caches and identify again. |
| `--no-cache` | | `false` | Bypass all caches. |
| `--dry-run` | | `false` | Preview metadata without artwork or audio writes. |
| `--artist VALUE` | | Unset | Override artist and search hint. |
| `--title VALUE` | | Unset | Override title and search hint. |
| `--album VALUE` | | Unset | Override album. |
| `--date VALUE` | | Unset | Override YYYY or valid YYYY-MM-DD release date. |
| `--genre VALUE` | | Unset | Override genre. |
| `--comment VALUE` | | Generated provenance | Set the comment verbatim. |
| `--source-url VALUE` | | Local file | Record a source in provenance. |
| `--artwork VALUE` | | Provider artwork | Local image path or HTTP(S) URL. |
| `--progress-target URI` | | Unset | Send NDJSON to stdout, stderr, fd://N, unix://PATH or tcp://HOST:PORT. |
| `--progress-socket PATH` | | Unset | Unix socket target if progress-target is absent. |

`tag-track artwork embed [FILE...]`:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--artwork IMAGE` | | Required unless URL | Embed a local image or HTTP(S) image URL. |
| `--url URL` | | Required unless artwork | Download an HTTP(S) image; mutually exclusive with artwork. |
| `--out-dir DIRECTORY` | `-o` | Source directory | Save copies here. |
| `--in-place` | `-i` | `false` | Replace the source after successful output commit. |

# Pipelines and Automation

Supply one path per stdin line, or pipe `DONE: <path>` / `output: <path>` results. A sole `-` selects stdin. Result-prefixed paths take precedence over surrounding log lines. `AGENT=1 tag-track skill` prints the canonical source skill installation instruction offline; the local Rust skill is present but publication is pending.

`TAG_TRACK_PROGRESS_TARGET` supplies a default NDJSON destination. Designated inherited descriptors must have exclusive writers while in use. Event output to stdout or stderr suppresses human results. See the [usage skill](skills/tag-track/SKILL.md) for complete event, environment and error contracts.

`self update` requires all of `TAG_TRACK_RUST_RELEASE_BASE_URL`, `TAG_TRACK_RUST_RELEASE_OWNER` and `TAG_TRACK_RUST_RELEASE_REPO` to identify an explicit published Rust channel. HTTPS is required except for loopback HTTP testing. An absent channel returns an unpublished-release error.

# License

[MIT](LICENSE), copyright 2026 Alex Gorbatchev.
