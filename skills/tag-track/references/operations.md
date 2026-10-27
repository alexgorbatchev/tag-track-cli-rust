---
created_on: 2026-10-14 10:03
last_modified: 2026-10-14 10:11
status: current
---

## Files and identification

Inspect and tag MP3, FLAC, M4A, Ogg Vorbis and WAV with FFmpeg/FFprobe 4.4+. Audio packets are copied without transcoding. Identification sends decoded fingerprints to Shazam and AcoustID; filename/manual hints drive iTunes and MusicBrainz fallbacks. Search removes video IDs and normalizes Unicode hyphens while keeping remix descriptors. Output artist/title names remove bracket descriptors and unsafe filename characters. Metadata search needs a UTF-8 filename; artwork embedding accepts native filesystem paths.

Absent tag values preserve existing tags except title and the provenance comment in a full update. A nonempty album, artist, genre or date replaces the existing field. The comment defaults to `Source: <source-url or local file> | Metadata: <provider> | Tagged: <YYYY-MM-DD>`; `--comment` supplies its exact replacement. Artwork-only editing preserves all other tags. Images become centered square 1400×1400 JPEG; remote and local artwork are bounded to 20 MiB. A dry run resolves metadata and emits its planned path without opening artwork or writing output. A distinct existing target is never overwritten. Output defaults to the source directory; if the generated target matches the source, the source is atomically updated. Use a separate out-dir to retain it. In-place source deletion occurs after atomic output commit.

Metadata and artwork cache under `$XDG_CACHE_HOME/tag-track` or the platform cache directory; AcoustID caches use audio fingerprints. Metadata is cached for seven days, AcoustID for thirty days and artwork for thirty days. Shazam results are retained in the resolved metadata cache. `--refresh` and `--no-cache` bypass reads/writes. Managed binaries use `$XDG_DATA_HOME/tag-track/bin` or the platform data directory, prepended to PATH. Dependency installation uses the supported system package manager. Agent mode and nonterminal stdin skip installation prompts; `--auto-install` supplies explicit consent. Installation is followed by verification.

## Output and errors

Human full updates print `Matched: <artist> - <title>` and `DONE: <path>`; dry runs print `Preview: <artist> - <title>`, album/date/genre/cover_art/source fields and `[DRY RUN] <path>`. `AGENT=1`, `true` or `yes` prints title/artist/album/date/year/genre/cover_art/source/dry_run/output key-value lines. Inspection prints file/format/codec/duration/bitrate/sample_rate/channels, present tags, has_artwork and cover dimensions/square state when present. Human terminal progress uses stderr. Verbose mode prints processing paths and metadata providers on stderr, except with stdout/stderr event targets. Displayed home paths fold to `~`; NDJSON paths retain filesystem values and require UTF-8. Returned errors print once on stderr with `ERR:`. Batches continue after individual file failures and return failure while retaining successful outputs.

Exit 0 means success/help/version; exit 1 means execution, dependency, transport or batch failure; exit 130 means SIGINT/SIGTERM or operation cancellation. Clap argument validation uses exit 2. Cancellation stops pending stdin, subprocess and network work. A failure after an output commit can leave that successful output in place; inspect it before retrying.

## Progress

Target precedence: `--progress-target`, then `--progress-socket`, then `TAG_TRACK_PROGRESS_TARGET`. An absent target emits no events. Targets accept stdout/stdout://, stderr/stderr://, inherited fd://N, unix://PATH or tcp://HOST:PORT. Bare paths starting /, ./ or ../, or ending .sock select Unix sockets; other bare values containing a colon select TCP. Designated inherited descriptors must not be concurrently written/reconfigured; nonblocking flags are restored after the final writer drops. TCP/Unix connection timeout is ten seconds. Transport errors fail the command. stdout/stderr event targets suppress human result output to keep NDJSON clean.

Each line is a JSON event: `timestamp` UTC RFC3339, `type`, `phase`, `step`, `total_steps`, optional `metadata`, optional `result`, optional `error`. Types/phases are phase_start/identify, metadata_found/identify, artwork_downloaded/artwork, tagged/tag, complete/complete and error/error. Metadata fields: title,artist,album,genre,release_date,release_year,cover_art_url,source. Complete result fields: path,title,artist,album,release_year,source,status (`ok` or `dry_run`). Dry runs omit artwork_downloaded and tagged events. Error events include a folded diagnostic.

## Releases

Rust binaries are unpublished by default. Set `TAG_TRACK_RUST_RELEASE_BASE_URL`, `TAG_TRACK_RUST_RELEASE_OWNER` and `TAG_TRACK_RUST_RELEASE_REPO` together for an explicit Rust release channel. The base URL requires HTTPS, except HTTP loopback for local testing. `self update` checks the latest tag once, selects the current OS/architecture release archive and atomically replaces the running executable only for a newer version. It prints update/current-or-installed and version. An absent channel returns a typed unpublished-release error; partial configuration is invalid. Go release archives do not supply this Rust binary.

`AGENT` selects output mode; PATH supplies external tools; XDG_DATA_HOME/XDG_CACHE_HOME override managed data/cache locations. The process uses the platform home-directory API for `~` expansion and display folding.
