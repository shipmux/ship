# Tabs

<!--
Scope: Nested tabs, the sidebar tree, the tab keys and mouse actions, and the matching CLI commands.
Not here: keeping tabs across restarts (saved-tabs.md).
-->

Keep related work under one tab, with smaller tasks nested below it. For example:

```text
▾ ship
  ▾ api
  ▸ docs
```

Each tab can hold panes, other tabs, or both, and in the sidebar `▾` means unfolded and `▸` means folded. A tab without a name shows its first pane's directory as its label.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-left` / `alt-right` | Previous / next tab |
| `alt-t` | [Tabs mode](keys.md#resize-tabs-and-panes-modes) to navigate, fold, move or rename tabs; `esc` leaves |
| `alt-b` | Hide or show the sidebar |
| `alt-x` | Close the tab and everything in it |

Coming back to a tab puts you on the pane you last used there. You can also click a tab to select it, click its `▾`/`▸` to fold it, or drag it to reorder or nest it.

## Renaming

Press `alt-t` then `r`, or right-click the tab's sidebar row and choose **rename**. The field starts with the saved name, or shows the automatic label dimmed when the tab is unnamed. Type a name and press Enter to save, or Esc to cancel. `ctrl-u` clears the field, and pressing Enter with it empty brings back the automatic label.

While the field is open, keys edit the name rather than navigate. Esc or any click cancels without saving, so click again to select somewhere else. When the field closes, you're still in the mode you opened it from, so press `esc` to leave tabs mode.

## From the CLI

Create commands print the new tab or pane as JSON, so `jq` can pull out IDs for the next step:

```sh
PROJECT=$(ship tab create --name project | jq -r .id)
REVIEW=$(ship tab create --name review "$PROJECT" | jq -r .id)   # nested under project
ship pane create --tab "$REVIEW" --name checks
ship tab list
ship tab set --tab "$REVIEW" --name code-review
ship tab set --tab "$REVIEW" --name   # bring back the automatic label
ship tab close --tab "$PROJECT"   # closes review and its pane too
```

`ship tab move` reorders a tab or gives it a new parent. A bare `--name`, as in `ship tab set --tab "$REVIEW" --name`, clears the name and brings back the automatic label, while `--name ""` sets an empty label.
