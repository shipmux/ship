# Tabs

<!--
Scope: Nested tabs, the sidebar tree, the tab keys and mouse actions, and the matching CLI commands.
Not here: keeping tabs across restarts (saved-tabs.md).
-->

Tabs can hold panes, other tabs, or both, so a big task can keep its smaller pieces underneath it. The sidebar on the left shows the tree. `▾` marks an open tab and `▸` a folded one. A tab without a name is labeled with its first pane's directory.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-left` / `alt-right` | Previous / next tab |
| `alt-t` | Tabs mode: `j` / `k` select tabs; `h` folds an open row or selects its parent; `l` unfolds a folded row or selects its first child; `shift-k` / `shift-j` move up / down among siblings; `shift-l` nests under the sibling above, `shift-h` moves out just after the parent; `r` renames; `esc` leaves |
| `alt-b` | Hide or show the sidebar |
| `alt-x` | Close the tab and everything in it |

Coming back to a tab puts you on the pane you last used there. You can also click a tab to select it, click its `▾`/`▸` to fold it, or drag it to reorder or nest it.

## Renaming

Press `alt-t` then `r`, or right-click the tab's sidebar row and choose **rename**. The field starts with the saved name, or shows the automatic label dimmed when the tab is unnamed. Type a name and press Enter to save, or Esc to cancel. `ctrl-u` clears the field; pressing Enter with it empty restores the automatic label.

Clicking or moving to another tab cancels without saving. Switching panes within the same tab keeps the edit open. When the field closes, you're still in the mode you opened it from; press `esc` to leave tabs mode.

## From the CLI

Every command prints JSON, so `jq` can pull out IDs for the next step:

```sh
PROJECT=$(ship tab create --name project | jq -r .id)
REVIEW=$(ship tab create --name review "$PROJECT" | jq -r .id)   # nested under project
ship pane create --tab "$REVIEW" --name checks
ship tab list
ship tab set --tab "$REVIEW" --name code-review
ship tab set --tab "$REVIEW" --name   # bring back the automatic label
ship tab close --tab "$PROJECT"   # closes review and its pane too
```

`ship tab move` reorders or reparents a tab. A bare `--name`, as in `ship tab set --tab "$REVIEW" --name`, clears the name and brings back the automatic label. `--name ""` sets an empty label.
