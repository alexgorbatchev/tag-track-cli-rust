---
created_on: 2026-10-14 10:03
last_modified: 2026-10-14 10:03
status: current
---

## Generated help queries

Use `--help`, `-h` on any command or supply an optional command path to its generated `help` command. Help produces no audio, dependency or network side effects. Agent help begins with the skill-reading alert. Supported generated query paths include:

- `tag-track help track`
- `tag-track help track update`
- `tag-track help track inspect`
- `tag-track help artwork`
- `tag-track help artwork embed`
- `tag-track help dependencies`
- `tag-track help dependencies check`
- `tag-track help dependencies install`
- `tag-track help dependencies update`
- `tag-track help self`
- `tag-track help self update`
- `tag-track help skill`
- `tag-track help help`
- `tag-track track help update`
- `tag-track track help inspect`
- `tag-track track help help`
- `tag-track artwork help embed`
- `tag-track artwork help help`
- `tag-track dependencies help check`
- `tag-track dependencies help install`
- `tag-track dependencies help update`
- `tag-track dependencies help help`
- `tag-track self help update`
- `tag-track self help help`

Additional words form the native help command's variadic `COMMAND` path. An unknown path returns an argument error.
