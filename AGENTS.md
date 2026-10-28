---
created_on: 2026-10-14 10:03
last_modified: 2026-10-14 10:11
status: current
---

# tag-track

Native Rust CLI for audio identification, inspection and atomic tag/artwork editing. Inherit the parent Rust instructions.

## Commands

- `just run --help` / `just run-ai --help`: human and agent help.
- `just test`: behavioral tests with local HTTP, native subprocesses and synthetic audio.
- `just lint`: package-scoped formatting and strict Clippy.
- `just coverage`: all-target line coverage, minimum 90%.
- `just check`: formatting, Clippy, tests and coverage.
- `just build`: release executable at `bin/tag-track`.

Use Rust 1.88+, Just, FFmpeg/FFprobe 4.4+, cargo-llvm-cov and llvm-tools. The native descriptor/signal paths target Unix; Linux is the tested platform. External dependencies are revision-pinned GitHub repositories. Scope formatting to this package.

## Contracts

- Keep the root file workflow and subject-first `track update`, `track inspect`, `artwork embed`, dependency management and self-update behavior grounded in the source commands. Put executable parsing in Clap.
- Delegate recognition, cache, subprocess handling and native audio tags to `audio-metadata`; progress transport to `cli-event-stream`; dependency/release operations to `cli-deps`; formatting/help/skill metadata to `cli-command-kit`. Keep this CLI's event schema, overrides and provenance policy here.
- Tag by copying native audio packets. Preserve unset tags, normalize artwork to 1400×1400 JPEG, commit atomically and refuse distinct destination collisions. A target matching the source is atomically replaced. Remove an in-place source only after its renamed output is committed. A failed batch returns nonzero even when earlier files succeeded.
- The default output is each source directory. `--out-dir .` is the process working directory. In-place output ignores `--out-dir`. Dry runs skip artwork reads and all audio writes.
- `--comment` overrides provenance. Preserve a complete release date. `--refresh` and `--no-cache` bypass metadata, recognition and artwork caches.
- Use native paths for filesystem/process operations. Metadata searches require UTF-8 filenames; artwork embedding preserves native filenames. Machine JSON paths require UTF-8 when an event stream is configured.
- Input descriptors are designated for exclusive reads while an operation is active. Duplicate ownership, O_NONBLOCK restoration, cancellation and final drop must remain coupled. Never use Tokio blocking stdin workers for pending pipe input.
- Print returned errors once from `main`, exit 130 for cancellation and 1 for execution/batch failures. Preserve Clap's argument-validation exits. Fold displayed home paths through the shared formatter, retaining native paths internally and in NDJSON.
- Keep event streams free of human output. Progress uses the common transport, and events carry UTC timestamps, step counts and the CLI-owned metadata/result schema.
- Keep installation consent explicit: no prompts in agent mode or with nonterminal stdin. Verify dependencies after installation. Test installers in isolated directories; never invoke a host package manager from tests.
- Rust artifacts and the local skill are unpublished. The skill command's canonical source URL does not establish that the Rust implementation is available there. Self-update needs all three explicit `TAG_TRACK_RUST_RELEASE_*` channel values; never use Go archives as Rust artifacts.

## Verification and boundaries

- Update `skills/tag-track/SKILL.md` and references in the same change as any command, argument, flag, type, default, environment, output or side effect. Maintain the live-interface drift test and metadata timestamps.
- Every change affecting execution results requires a corresponding test-file change and at least 90% line coverage; `scripts/` is excluded. Record genuine failing tests, implement, temporarily disable the implementation, verify failure, restore exactly and rerun.
- Use synthetic audio, local HTTP and isolated caches. Never run the upstream Go self-update test, which can replace a live executable. Never make live recognition, cloud or package-install writes during tests.
- Store temporary fixtures/evidence in `.tmp/`. Keep runtime errors and generated files out of Git. Preserve source MIT attribution and direct dependency licenses.
- Persist only standing project rules here; update rules in place. Keep task history, progress, assignments and handoffs outside this file.
- Never publish releases, tags, packages or deployments without explicit user authorization.
