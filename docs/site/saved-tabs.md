# Saved tabs

<!--
Scope: What comes back after a restart and what doesn't, where the file lives, and how to start fresh.
Not here: the file's format (openspec/specs/session-restore).
-->

After a server restart or a reboot, Ship brings back your tabs, their nesting and names, the splits and their sizes, pane stacks and their front panes, zoom, and each pane's last directory. Panes also keep their [exit behavior](panes.md#when-a-program-exits) and [history size](configuration.md#server-settings). Every pane starts a fresh shell. Running programs, agent conversations and scrollback don't come back.

Ship saves a few seconds after each change, and `ship server stop` saves right away. The file lives at `~/.local/state/ship/tabs/default.json` (under `$XDG_STATE_HOME` if you set it). A [named server](windows-and-the-server.md#running-more-than-one-server) uses its own name in place of `default`. On Windows, saved tabs live under Ship's platform data directory instead.

If a pane's directory is gone, it opens in your home directory instead.

If the saved file can't be read or parsed, Ship logs the error and restores no tabs. Normal startup then creates a fresh shell tab. Saving can replace a damaged file, keeping its old contents as `.prev` if they can be read. If you're recovering tabs, copy both files somewhere safe before making more changes.

## Getting back the previous tabs

The first save after each server start keeps the file it replaced as `default.json.prev`. To go back to it:

```sh
ship server stop
SAVED="${XDG_STATE_HOME:-$HOME/.local/state}/ship/tabs/default.json"
cp "$SAVED.prev" "$SAVED"
ship
```

Copy the `.prev` file somewhere safe if you want to keep it, since the next run replaces it.

## Starting fresh

`ship server --no-restore` starts a server in the foreground without loading the saved tabs. Add `--starter` to get one shell tab. Your first change then replaces the saved tabs, keeping the old ones as `.prev`.

To try things without touching your usual tabs, use a server with another name:

```sh
ship --server-name scratch
```

It has its own saved tabs, and opens with one shell tab the first time.
