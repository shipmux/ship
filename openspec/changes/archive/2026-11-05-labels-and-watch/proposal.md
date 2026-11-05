# Proposal

Status: Draft. Assembled from the locked `design/product.md`, `design/architecture.md` and `design/program.md`. The papers are the contract, and this proposal and the deltas under `specs/` are views of them. Parts that wait on the product's open unknowns (OU-1 to OU-3) or the program's choices (C1 to C6) are left out of the deltas until Cyan decides them.

## Why

Scripts can drive Ship through the CLI, but they can't follow what happens in it without polling, and they can't show anything in it ([#87](https://github.com/shipmux/ship/issues/87)). Plugins should be just more clients: they read the same state every client reads, and they act through the same `ship` commands, with no event system or plugin runtime to maintain.

## What Changes

- **`ship watch`** prints a first JSON line holding the server's `protocol_version`, then one JSON line per published state, unchanged from what clients receive. `--feed latest` (the default) gives the newest state and may skip some; `--feed every` gives each one in order. It prints no screens or clipboard text, and when the stream ends it says why and exits with an error.
- **Tags** on tabs and panes: a map from a free-form name to a value string with an optional `fg` and `bg` from the 16 named terminal colors. `ship pane set` and `ship tab set` take a repeatable `--tag NAME=VALUE`, a bare `--tag NAME` to remove one, and `--clear-tags`. `PATCH` takes `tags` as a JSON Merge Patch. The last write wins. Tags disappear with their tab or pane, aren't saved, and never drive attention.
- **Sidebar rows.** A new `[client.tabs] rows` lays out each tab's entry as lines of `"name"` and `{ tag = "..." }`, defaulting to today's single name line. A tab without a tag shows its first pane's. `[client.agents] rows` also accepts `{ tag = "..." }`. Missing tags disappear with their separators, and a tag's colors apply to its value only.
- **A git-branch example** in `examples/plugins/`, off by default, plus docs for `watch`, tags and the rows settings.

Not in this change: an event system, plugin SDK or runtime, tag ownership, client names, limits on tags, config-side style rules, hex colors, tags from escape sequences, screens in `watch`, a server startup command, plugin config layers, and git support in Ship.

## Capabilities

### New Capabilities

- `tags`: tag names, values and colors on tabs and panes, setting them from the CLI and over HTTP, and their lifetime.
- `state-watching`: `ship watch`, its first line, its feeds, what it leaves out and how it ends.

### Modified Capabilities

- `session-structure`: tabs and panes gain the `tags` attribute in `set` and in `PATCH`.
- `session-restore`: tags are never saved, and loading ignores them.
- `terminal-client`: a tab's sidebar entry can span several lines, from `[client.tabs] rows`.
- `config-file`: the new `[client.tabs]` table, and tag items in `[client.agents] rows`.
- `agent-attention`: agent rows can show a tag of the agent's pane.

## Impact

- `ship-core`: a new `tag` module, `tags` on `Pane` and `Tab`, `tags` on `SetPane` and `SetTab`, the `--tag` and `--clear-tags` flags, `[client.tabs]` in `defaults.toml`, and `PROTOCOL_VERSION` 21 to 22, so a client and server from either side of this change refuse each other with the existing restart guidance.
- `ship-server`: the two `Set` handlers, the saver and restore.
- `ship-client`: `observe()` made public, tag folding in `execute`, the tab and agent sidebar sections, and settings.
- `ship` binary: the `watch` subcommand.
- Dependencies: serde_with's `indexmap_2` feature.
- Repo: `examples/plugins/`, `docs/site/scripting/`, `docs/site/configuration.md` and `GLOSSARY.md`.
