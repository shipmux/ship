# Writing a plugin

<!--
Scope: Following Ship from a script with `ship watch`, and showing text on tabs and panes with tags: the conventions (ignore unknown fields, `plugin.key` names, `ship.*` reserved, no compatibility promise before 1.0) and the git example.
Not here: the sidebar rows settings (configuration.md), JSON shapes (openspec/specs).
-->

A plugin is a script that uses `ship` like any other client. Ship has no plugin runtime or install step: you run the script, it follows Ship's state with `ship watch` and puts text on tabs and panes with tags.

## Watching

`ship watch` prints Ship's state as JSON, one line per change, until you stop it:

```json
{"service":"ship","protocol_version":23,"version":"0.1.0",…}
{"type":"attached","data":{"attachment":"attachment:…","replica":{"revision":1,"tabs":{…},…}}}
{"type":"state","data":{"revision":2,"tabs":{…},…}}
```

The first line is what `ship server status` prints. After it comes the same stream the Ship client receives: `attached` holds the first state, and each `state` line holds a newer one. This filter turns them into one state per line:

```sh
ship watch | jq -c '.data.replica // select(.type == "state").data'
```

Watching changes nothing in Ship: it doesn't select or resize anything, and it gets no screen contents or clipboard text. It never starts a server either, so it fails if none is running.

A reader that falls behind gets the newest state and skips the ones in between. To see every state in order, use `ship watch --feed every`; it ends if the reader falls too far behind.

When the stream ends, the last line says why, and `watch` exits with an error. The reason is `server_shutdown` when the server stops, and `lost` when an `every` feed fell behind:

```json
{"type":"ended","data":{"reason":"server_shutdown"}}
```

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

Setting a tag replaces it whole, so a later `--tag gh.checks=ok` without color flags leaves it uncolored. For the same reason, a color flag needs a `--tag` for its name in the same call.

Tags last as long as their tab or pane and aren't saved: after the server restarts, a plugin sets them again. Whichever script writes a tag last wins, since tags have no owner.

## The git example

[`examples/plugins/git.sh`](../../../examples/plugins/git.sh) tags each pane in a git checkout with its branch as `git.branch`, and with its pull request's state and checks as `git.pr`. Start it once the server is running, and show the tags under each tab's name:

```sh
sh examples/plugins/git.sh &
```

```toml
[client.tabs]
rows = [["name"], [{ tag = "git.branch" }, { tag = "git.pr" }]]
```

For each state from `ship watch`, it asks git for the branch of every pane's `dir` and runs `ship pane set` only where that differs from the pane's tags, so the script never keeps a copy of them. Pull requests change on GitHub without anything changing in Ship, so it also looks them up with `gh` every 30 seconds. [Its README](../../../examples/plugins/README.md) lists what each tag looks like and when it updates.

## Conventions

- Name tags `plugin.key`, such as `git.branch`. Ship gives the `.` no meaning; it keeps plugins from colliding. Names starting with `ship.` are reserved for Ship.
- Ignore JSON fields and `watch` line types you don't recognize, so a newer Ship doesn't break your script.
- Before 1.0, the JSON shapes may change between versions. `ship server status` reports the `protocol_version` a script can check.
