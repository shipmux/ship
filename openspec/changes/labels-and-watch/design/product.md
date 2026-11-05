# `ship watch` and tags

Status: Locked on 2026-11-04, re-approved after the rename from labels to tags ("If you renamed everything properly, then they're good to go"). First locked on 2026-11-04. Written from a design conversation with Cyan on 2026-11-04, continuing the plugin design session of the same day, for [#87](https://github.com/shipmux/ship/issues/87), then approved in Plannotator review ("LGTM"). The open unknowns below stay open until Cyan decides them.

## Goal

Scripts can drive Ship through the CLI but can't follow what happens in it without polling, and they can't show anything in it. This change makes plugins just more clients: `ship watch` prints the same state stream every client receives, and tags let a script put short text, optionally colored, on tabs and panes, which the sidebar shows where the user's config asks. Ship gains no event system, plugin runtime or git support. A git-branch example plugin, kept in the repo, proves the pieces fit.

What a user with the git example running and a two-line tab layout sees:

```text
┌ tabs ──────────────── ≡ ┐
│▌web                     │
│   main · 2              │   ← git.branch, git.ahead: the tab's own, else its first pane's
│ api                     │
│   fix/auth · FAIL       │   ← FAIL drawn red by the plugin that set it
│ notes                   │   ← no tags: the second line disappears
└─────────────────────────┘
```

```toml
[client.tabs]
rows = [["name"], [{ tag = "git.branch" }, { tag = "git.ahead" }]]
```

The conventions behind this:

- Plugins as clients follows i3/sway IPC, Neovim's RPC UIs and Kakoune's shell-out model: one API for the UI and for scripts, no hook taxonomy. Ship's `watch` prints the real state, not derived events; plugins work out their own events.
- Tags follow Herdr's metadata tokens, tmux user options and kitty/WezTerm user vars: short display-only strings on panes and tabs. Like Herdr, Ship keeps them out of agent attention.
- The plugin, not config, chooses a tag's colors, as in i3bar blocks and SketchyBar. Herdr's config-side style rules (`gt = 80`) are left out.
- Names are free-form with a documented `plugin.key` convention, as Docker does, rather than enforced structure as Kubernetes does.
- Setting tags extends `PATCH` with JSON Merge Patch (RFC 7396), which pane and tab `set` already follow.

## User stories

### P1: Follow Ship from a script

A plugin wants to react to Ship's state, such as a pane changing directory, without polling.

- Given a running server, when you run `ship watch`, then it prints one JSON line holding `protocol_version`, then one JSON line per published state.
- Given the default feed, when the script reads slowly, then each line it reads is the newest state, intermediate states may be skipped, and it never falls behind.
- Given `--feed every`, when states are published, then it prints every one in order; if it falls too far behind, it prints why it ended and exits with an error.
- Given panes are drawing output, when `watch` runs, then it prints no screen contents or clipboard text.
- Given the server shuts down, when the stream ends, then `watch` prints why it ended and exits with an error.

Tested independently: run `ship watch | jq -c .`, create and rename a tab from another terminal, and see each change arrive.

### P2: Tag a pane or tab from a script

- Given a pane, when you run `ship pane set --pane p3 --tag git.branch=main --tag git.ahead=2`, then `ship pane get` and the next `watch` line show both tags.
- Given that pane, when you run `ship pane set --pane p3 --tag git.ahead`, then only `git.ahead` is removed.
- Given `--tag git.ahead=` with an empty value, when it runs, then the tag is kept with an empty value.
- Given `--clear-tags`, when it runs, then every tag on the pane is removed.
- Given `--name api --tag git.branch=main` in one call, when it runs, then both change in one published state.
- Given a write that changes nothing, when it runs, then no new state is published.
- Given a name containing `=` or a control character, when it runs, then it fails and nothing changes.
- Given the same flags on `ship tab set`, when they run, then the tab gets tags the same way.
- Given a tagged pane closes, or the server restarts, when you look again, then the tags are gone.

Tested independently: set, clear and overwrite tags with `ship pane set` and `ship tab set`, checking `get` after each.

### P3: Show tags in the sidebar

- Given `[client.tabs] rows = [["name"], [{ tag = "git.branch" }]]`, when a tab has `git.branch`, then its value shows on a second line under the tab's name.
- Given a tab without `git.branch` whose first pane has it, when the sidebar draws, then the tab shows the pane's value, the way an unnamed tab shows its first pane's directory. `ship tab get` still shows only the tab's own tags.
- Given a row's tags are missing, when the sidebar draws, then they and their separators disappear, and a row with nothing left disappears.
- Given `[client.agents] rows` contains `{ tag = "..." }`, when an agent's pane has that tag, then it shows in the agent's entry.
- Given a tag with `fg = "white"` and `bg = "red"`, when it draws, then only its text takes those colors; separators and the rest of a selected row keep the row's style.
- Given no sidebar config, when the sidebar draws, then it looks as it does today.

Tested independently: set tags by hand, add rows to the config, and look.

### P4: Show git branches with the example plugin

- Given `examples/plugins/git-branch.sh` is running, when a pane is in a git checkout, then the pane carries a `git.branch` tag with the current branch, and its tab shows it through its first pane.
- Given you `cd` into another repo or switch branches, when the shell's prompt returns, then the tag follows.
- Given the pane leaves any checkout, when its directory changes, then the tag is removed.
- Given `examples/plugins/README.md`, when you read it, then it says how to run the plugin and which rows config shows its tags.

Tested independently: run the example on Linux and macOS and use it for daily work.

## Amendments (2026-11-04)

Decided with Cyan during slices 1 and 2. Each settles an open unknown and adds to the marked requirement.

- **P1. Tag values refuse control characters and lose bidirectional formatting characters** (OU-2, FR-005). A tag value with a control character fails with 400 `validation` and changes nothing, as an invalid name does. Bidirectional formatting characters are removed from a value before it is stored, so the stored value and the change test both see the cleaned text. This is the program-status rule: text drawn outside the terminal grid can't move the cursor or reorder what is drawn after it, and a script that sends a control character learns about its bug.
- **P2. Colors on the CLI are separate flags keyed by tag name** (OU-1, FR-008). `ship pane set` and `ship tab set` accept repeatable `--tag-fg NAME=COLOR` and `--tag-bg NAME=COLOR`, which color the `--tag NAME=VALUE` of the same name in the same call, whatever the order. A color for a name that call doesn't set, including a bare `--tag NAME`, is an error and changes nothing, because a write replaces a tag whole. A repeated color flag for one name takes its last occurrence. A full JSON form was weighed and left to a general `ship api` passthrough, outside this change ([#113](https://github.com/shipmux/ship/issues/113)), so the CLI keeps one shell-safe form.

## Non-goals

- An event system, derived events, a plugin SDK, runtime, install command or marketplace.
- A compatibility promise for `watch` output before 1.0. Each line set is the real state; `protocol_version` says which shape it is. Revisit an additive-only promise at 1.0.
- Tag ownership: tags cleared when the client that wrote them disconnects. Deferred; adding it later breaks nothing.
- Client names for attachments (`ship watch --name`) and a list of attachments.
- Limits on how many tags exist, or on name and value length.
- Config-side style rules, hex or RGB colors, and bold, dim or other modifiers.
- Tags driving attention, notifications or sounds. Those stay with program status (OSC 7501).
- Tabs showing a rollup of all their panes' tags. A tab falls back to its first pane only.
- Tags set by a pane's program through an escape sequence.
- Clearing tags by prefix.
- Saving tags across a server restart.
- Screens in `watch`, a server startup command or supervised programs, plugin config layers, floating panes ([#86](https://github.com/shipmux/ship/issues/86)), exposing selected text, OSC 8 hyperlinks and bells ([#110](https://github.com/shipmux/ship/issues/110)).
- Git support in Ship. The example is not installed or run by default.

## Functional requirements

### Watching

- FR-001: `ship watch` MUST print a first JSON line holding the server's `protocol_version`, then one JSON line per published state, unchanged from what clients receive.
- FR-002: `ship watch` MUST accept `--feed latest` and `--feed every`, defaulting to `latest`. `latest` MUST deliver the newest state and may skip intermediate ones; `every` MUST deliver every state in order.
- FR-003: `ship watch` MUST NOT print screen contents or clipboard text.
- FR-004: When its stream ends (the server shuts down, or an `every` feed falls behind), `ship watch` MUST print why and exit with an error. See OU-3.

### Tags

- FR-005: Panes and tabs MUST carry tags: a map from name to a value string with an optional `fg` and `bg`, shown in `ship pane get`, `ship tab list`, and the state every client and watcher receives. A pane or tab with no tags MUST omit the field.
- FR-006: A tag name MUST be any non-empty string without `=` or control characters. Ship MUST give no meaning to `.` or any other character.
- FR-007: `fg` and `bg` MUST be one of the 16 named terminal colors when present.
- FR-008: `ship pane set` and `ship tab set` MUST accept a repeatable `--tag NAME=VALUE` that sets a tag, a bare `--tag NAME` that removes one, and `--clear-tags` that removes all. A repeated name MUST take its last value. An empty value MUST set an empty tag. Setting colors from the CLI: see amendment P2.
- FR-009: Writing a tag MUST replace its value and colors whole; the last write wins.
- FR-010: Over HTTP, `PATCH` on a pane or tab MUST accept `tags` as a merge patch: an absent name is unchanged, a tag sets it, `null` removes it, and `tags: null` removes all.
- FR-011: Tag changes and other attribute changes in one request MUST be published as one state. A request that changes nothing MUST publish nothing.
- FR-012: Tags MUST be removed when their pane or tab closes, and MUST NOT be saved or restored across a server restart.
- FR-013: Tags MUST NOT affect agent attention, notifications or sounds.

### Sidebar

- FR-014: `[client.tabs] rows` MUST lay out each tab's sidebar entry as lines of items. A bare string MUST name a built-in field, starting with `name`, and `{ tag = "..." }` MUST name a tag. The default MUST be `[["name"]]`.
- FR-015: In tab rows, a tag MUST show the tab's own value, else its first pane's value for the same name, as the tab's name falls back to its first pane. The fallback is a display rule: `ship tab get` and `watch` MUST show only the tab's own tags.
- FR-016: `[client.agents] rows` MUST also accept `{ tag = "..." }`, naming a tag on the agent's pane.
- FR-017: A missing tag MUST disappear with its separator, and a line with no values MUST disappear. An unknown bare string MUST be a config error.
- FR-018: A tag's `fg` and `bg` MUST apply to its value only. Separators and the rest of a selected row MUST keep the row's style.

### Example and docs

- FR-019: `examples/plugins/git-branch.sh` MUST tag each pane with the git branch of its directory and remove the tag outside a checkout. `examples/plugins/README.md` MUST say how to run it and how to show its tags.
- FR-020: The scripting docs MUST cover `watch` and tags, including: ignore fields you don't recognize, name tags `plugin.key`, `ship.*` is reserved for Ship, and there is no compatibility promise before 1.0. The configuration page MUST document the rows settings.

## Success criteria

- SC-001: A tag set from the CLI appears in an attached client's sidebar as quickly as a tab rename does.
- SC-002: With the git example running, the branch tag follows a `cd` or branch switch without lag Cyan notices in daily use.
- SC-003: With five `watch` processes attached, typing latency in the benchmark harness stays within noise of none attached.
- SC-004: With no sidebar config, the sidebar is unchanged from today.
- SC-005: P1 to P4 pass on Linux and macOS.

## Open unknowns

- OU-1: **Colors on the CLI.** Decided by amendment P2: the separate flags.
- OU-2: **Control characters in values.** Decided by amendment P1: the program-status rule.
- OU-3: **How `watch` reports its end.** The attach stream already ends with a reason (`server_shutdown`, `lost`). Recommendation: print that as the last JSON line, then exit with an error. Blocks FR-004.
