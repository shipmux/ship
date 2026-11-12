# Writing a plugin

<!--
Scope: Following Ship from a script with `ship watch`, and showing text on tabs and panes with tags: the conventions (ignore unknown fields, `plugin.key` names, `ship.*` reserved, no compatibility promise before 1.0) and the git example.
Not here: the sidebar rows settings (configuration.md), JSON shapes (openspec/specs).
-->

A plugin is a script that puts short text on your tabs and panes. The git example, for instance, shows each tab's branch and pull request under its name in the sidebar:

```text
     api
     fix-login · ○ #118 ✓
     docs
     main
```

A plugin uses `ship` like any other client. Ship has no plugin runtime or install step: you run the script, it follows Ship's state with `ship watch` and puts text on tabs and panes with tags.

## Watching

`ship watch` prints Ship's state as JSON, one line per change, until you stop it:

```json
{"service":"ship","protocol_version":24,"version":"0.1.0",…}
{"type":"attached","data":{"attachment":"attachment:…","replica":{"revision":1,"tabs":{…},…}}}
{"type":"state","data":{"revision":2,"tabs":{…},…}}
```

The first line is what `ship server status` prints. The lines after it are what Ship's own window receives: `attached` holds the first state, and each later `state` line holds a newer one. To get only the states, one per line:

```sh
ship watch | jq -c '.data.replica // select(.type == "state").data'
```

Watching is read-only. It doesn't select or resize anything, and it never sees screen contents or clipboard text. It never starts a server either, so it fails if none is running.

If your script is slow, `ship watch` skips ahead to the newest state. To get every state in order, use `ship watch --feed every`. That stream ends if your script falls too far behind.

When the stream ends, the last line says why, and `watch` exits with an error:

```json
{"type":"ended","data":{"reason":"server_shutdown"}}
```

The reason is `server_shutdown` when the server stops, and `lost` when an `every` feed fell behind.

## Tags

A tag is a named piece of short text on a tab or pane. Set one with `--tag NAME=VALUE`, and read it back from `get`:

```sh
ship pane set --pane "$PANE" --tag git.branch=main
ship pane get --pane "$PANE" | jq -c .tags
```

```json
{"git.branch":{"value":"main"}}
```

`--tag` repeats, so one call can set several. A bare `--tag NAME` removes that tag, and `--clear-tags` removes them all:

```sh
ship pane set --pane "$PANE" --tag git.branch=main --tag git.ahead=2
ship pane set --pane "$PANE" --tag git.ahead
ship pane set --pane "$PANE" --clear-tags
```

`ship tab set --tab "$TAB"` takes the same flags. The value is everything after the first `=`, so it can hold `=` and can be empty. If a call names a tag twice, the last one wins.

To color a tag, add `--tag-fg NAME=COLOR` or `--tag-bg NAME=COLOR` beside the `--tag` that sets it. The colors are the 16 terminal names: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, and each of those with `bright_` in front.

```sh
ship pane set --pane "$PANE" --tag gh.checks=failing --tag-fg gh.checks=white --tag-bg gh.checks=red
```

Setting a tag replaces all of it, colors included. So `--tag gh.checks=ok` on its own leaves the tag uncolored, and a color flag only works beside a `--tag` for the same name.

A tag lasts as long as its tab or pane. Tags aren't saved, so after the server restarts a plugin sets them again. If two scripts write the same tag, the last write wins.

## The git example

[`examples/plugins/git.sh`](../../../examples/plugins/git.sh) tags each pane in a git checkout with its branch as `git.branch`, and with its pull request's state and checks as `git.pr`. Start it once the server is running:

```sh
sh examples/plugins/git.sh &
```

Then show the tags under each tab's name, with [tab rows](../configuration.md#tab-rows):

```toml
[client.tabs]
rows = [["name"], [{ tag = "git.branch" }, { tag = "git.pr" }]]
```

On each state from `ship watch`, the script asks git for the branch of every pane's `dir`, and runs `ship pane set` only for panes whose tags differ. It keeps no copy of the tags itself. Pull requests can change on GitHub without anything changing in Ship, so it also checks them with `gh` every 30 seconds. [Its README](../../../examples/plugins/README.md) lists what each tag looks like and when it updates.

## Conventions

- Name tags `plugin.key`, such as `git.branch`. Ship gives the `.` no meaning; it keeps plugins from colliding. Names starting with `ship.` are reserved for Ship.
- Ignore JSON fields and `watch` line types you don't recognize. New ones can appear in any release, and ignoring them keeps a newer Ship from breaking your script.
- A change that breaks existing shapes bumps `protocol_version`, which `ship server status` reports, and the release notes say why.
- Object key order carries meaning: tabs, panes and status records are listed in order. Read them with `keys_unsorted` or `to_entries`, not `keys`.
