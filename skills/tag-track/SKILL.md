---
name: tag-track
description: Use when identifying, inspecting or updating audio tags and cover art with tag-track.
author: alexgorbatchev
metadata:
  created_on: 2026-10-14 10:03
  last_modified: 2026-10-14 10:11
  status: current
---

Use `AGENT=1 tag-track` for compact output. Read [commands](references/commands.md) for all commands, options and input rules, [help queries](references/help.md) for generated help paths, and [operations](references/operations.md) for output, caches, progress, cancellation and update policy.

## Workflows

Inspect existing tags before editing:

```sh
AGENT=1 tag-track track inspect 'Music/Artist - Song.flac'
```

Preview identification, then save a tagged copy:

```sh
AGENT=1 tag-track track update 'Music/Artist - Song.flac' --dry-run
AGENT=1 tag-track 'Music/Artist - Song.flac' --out-dir tagged --comment 'My source note'
```

Embed square artwork while preserving the existing tags:

```sh
AGENT=1 tag-track artwork embed 'Music/Artist - Song.flac' --artwork cover.png --in-place
```

Pipe `DONE: <path>`, `output: <path>` or one plain path per line through stdin. Use a sole `-` file argument to select stdin explicitly. Check the process status after a batch: successful earlier outputs remain when a later file fails.

The local Rust artifact and skill are unpublished. `tag-track skill` prints the canonical source repository install instruction and URL offline; publication availability is separate from that output.
