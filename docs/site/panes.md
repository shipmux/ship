# Panes

<!--
Scope: Splitting, moving between, renaming, zooming and closing panes, what happens when a pane's program exits, and the matching CLI commands.
Not here: scrolling and copying (scrolling-copying-and-the-mouse.md), scripting recipes (scripting/).
-->

Every pane runs a program in its own terminal, your shell unless you say otherwise. By default, the pane closes when its program exits. You can also close it with `alt-shift-x`.

| Key | Does |
| --- | --- |
| `alt-\|` / `alt--` | Split right / down |
| `alt-s` | Stack a new shell in the same place |
| `alt-,` / `alt-.` | Previous / next pane in the stack |
| `alt-e` | Open the pane's scrollback in your editor |
| `alt-h` / `alt-j` / `alt-k` / `alt-l` | Move to the pane left, below, above or right |
| `alt-tab` | Next visible pane in the tab, skipping panes behind |
| `alt-p` | Panes mode: `h` / `j` / `k` / `l` select a neighbor; `shift-h` / `shift-j` / `shift-k` / `shift-l` swap with it; `,` / `.` cycle the stack; `r` renames; `esc` leaves |
| `alt-f` | Zoom the pane to fill the tab, or unzoom |
| `alt-r` | Resize mode: `h` / `j` / `k` / `l` move the pane's edge, `esc` leaves |
| `alt-shift-x` | Close the pane |

A new split or stacked shell opens in the directory you were working in, so after `cd ~/src/ship`, `alt--` gives you a shell there. You can also resize by dragging the border between two panes.

`alt-s` opens a shell in front of the current pane without splitting. The panes behind it keep running. Their titles appear in the border, front first and bold, separated by ` · `. A stack without a border gets a title row instead. Click a title to bring that pane forward, or use `alt-,` and `alt-.` to cycle. Everyone viewing the tab sees the same front pane.

Zoom belongs to the tab, so everyone looking at it sees the zoomed stack's front pane, and its sidebar row shows `Z`. Cycling, stacking and closing the front keep the stack zoomed while it has panes. Moving to another split, splitting, swapping or resizing unzooms first. A create that selects its new pane also unzooms if needed to show it.

The CLI can do the same things. Panes created from the CLI start in your current directory, or in `--cwd`:

```sh
TAB=$(ship tab create --name work | jq -r .id)
LEFT=$(ship pane create --tab "$TAB" | jq -r .id)
RIGHT=$(ship pane create --pane "$LEFT" --place right -- htop | jq -r .id)
ship pane create --pane "$LEFT" --place stack
ship pane cycle --pane "$LEFT" --direction next
ship tab set --tab "$TAB" --zoomed "$LEFT"      # zoom
ship tab set --tab "$TAB" --zoomed               # unzoom
ship pane swap --pane "$LEFT" --other "$RIGHT"
ship pane resize --pane "$LEFT" --direction right --amount 5
ship tab get --tab "$TAB" | jq .layout
ship tab close --tab "$TAB"
```

Move a pane onto another stack or into a new split without restarting its program:

```sh
ship pane move --pane "$PANE" --onto "$OTHER"
ship pane move --pane "$PANE" --beside "$OTHER" --direction right
```

`OTHER` can be in another tab, or in the same stack when taking a pane out into its own split. Use `--direction down` to put it below instead. The pane keeps its ID, output and running program. Clients with it selected follow it; the destination unzooms if needed to keep it visible. An emptied old slot disappears, but its tab stays.

## Renaming

Press `alt-p` then `r`, or right-click the pane and choose **rename**. Editing, saving and clearing work [the same way as tab renaming](tabs.md#renaming); clearing a pane's name restores its program's title, or its command if it has no title. Press `esc` after renaming to leave panes mode.

Moving to another pane cancels without saving. This includes another client bringing a different pane to the front of your stack, which also changes your selected pane.

## Edit scrollback

Press `alt-e`, or choose **edit scrollback** from the pane's right-click menu, to open its full history and screen in your editor. The editor opens in front of the pane without splitting it. Quit the editor to return; any edits are discarded, and a zoomed stack stays zoomed.

On Unix, Ship uses `VISUAL`, then `EDITOR`, then `vi`. Values with arguments, such as `code --wait`, work. For the shortcut, set these in the environment that starts the server. The temporary file is readable only by you and deleted when the editor exits; a later run cleans up files left by a crashed run. If the editor fails, its pane stays open showing the error until you close it.

To open a specific pane's scrollback from another terminal:

```sh
ship pane edit-scrollback --pane "$PANE"
```

Without `--pane`, the command reads the pane directly behind its own pane, as the shortcut does. With no pane behind it, it reports an error instead.

## When a program exits

| Exit behavior | Does |
| --- | --- |
| `close` (default) | Closes the pane when its program exits |
| `keep` | Keeps the pane's output and exit status until you close it |
| `keep_failed` | Closes after success; keeps the pane after a non-zero exit code or a signal |

Closing the front pane brings the next pane in its stack forward. When the stack's last pane closes, its neighbor takes the space and the selection moves there. A tab with no panes stays empty.

Set the behavior when creating a pane, or change it before the program exits:

```sh
ship pane create --tab "$TAB" --exit-behavior keep -- cargo test
ship pane set --pane "$PANE" --exit-behavior keep_failed
```

Changing the behavior of an already exited pane only stores the value; it doesn't close the pane. Kept panes can still be read, scrolled and copied, but no longer accept typing or resizing. [Reading output](scripting/reading-output.md) shows the create, wait, read and close workflow for scripts. To change the default for new panes, use [the config file](configuration.md#server-settings).
