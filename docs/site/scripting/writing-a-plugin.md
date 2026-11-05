# Writing a plugin

<!--
Scope: Following Ship from a script with `ship watch`, and showing text on tabs and panes with tags: the conventions (ignore unknown fields, `plugin.key` names, `ship.*` reserved, no compatibility promise before 1.0) and the git example.
Not here: the sidebar rows settings (configuration.md), JSON shapes (openspec/specs).
-->

A plugin is a script that uses `ship` like any other client. Ship has no plugin runtime or install step: you run the script, it reads Ship's state as JSON and puts text on tabs and panes with tags.

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

## Conventions

- Name tags `plugin.key`, such as `git.branch`. Ship gives the `.` no meaning; it keeps plugins from colliding. Names starting with `ship.` are reserved for Ship.
- Ignore JSON fields you don't recognize, so a newer Ship doesn't break your script.
- Before 1.0, the JSON shapes may change between versions. `ship server status` reports the `protocol_version` a script can check.
