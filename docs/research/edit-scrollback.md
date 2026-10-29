# Edit scrollback

Status: undecided. Recorded on 2026-10-27 for [#85](https://github.com/shipmux/ship/issues/85). Nothing here authorizes implementation.

## The question

#85 replaces built-in search and copy mode with one action that opens a pane's history in the user's editor. The direction is agreed. Open: where the editor pane goes, how the editor is chosen and run, what text the file holds, and how the temp file is handled.

## Summary

Every reference that has this feature does the same three things: dump the history to plain text, run the editor on the file in a pane that sits over the original, and throw away whatever the editor saves. They differ in where that pane lives and how carefully they clean up.

| | Pane placement | Editor order | Runs through shell | Wrapped lines | Temp file |
|---|---|---|---|---|---|
| Zellij | Replaces the original in its layout slot; original parked as "suppressed" | config, `EDITOR`, `VISUAL`, `vi` | No, split on whitespace | Joined | `/tmp/<uuid>.dump`, umask permissions, never deleted |
| Herdr | Split from the original, then zoomed; focus and zoom restored on exit | `${EDITOR:-vi}` on Unix; `VISUAL`, `EDITOR`, `notepad` on Windows | Yes, `/bin/sh` | Joined | `create_new`, mode 0600, deleted on exit |
| tmux 3.8 | Modal floating pane at 90% of the window | `editor` option, set from `VISUAL` then `EDITOR` at start | No, `"editor path"` formatted | Selection or OSC 133 output only | `mkstemp` in `/tmp`, deleted on exit |
| kitty | Overlay window over the original | `scrollback_pager` (default `less`) | No, argv | Not joined (one line per row) | None, piped on stdin |
| WezTerm | No built-in; docs example spawns `vim` in a new window from Lua | Hard-coded in the example | n/a | Either, via `get_lines_as_text` or `get_logical_lines_as_text` | `os.tmpname()`, removed after a 1 s sleep |

Ship already has the pieces Herdr uses: splits, server-authoritative zoom, panes that close when their program exits, and a history read that can join wrapped lines. Herdr's shape fits best for now. Floating panes ([#86](https://github.com/shipmux/ship/issues/86)) would be the better home later, as in tmux and kitty.

## Zellij

Source: zellij-org/zellij at `1f3df93`, paths under `zellij-server/src/`.

- **Binding:** `e` in scroll mode runs `EditScrollback`, then returns to normal mode (`zellij-utils/assets/keybinds/default.kdl:92-94`). The CLI and plugins can target a pane by ID.
- **Dump:** scrollback plus viewport, plain text, joined at logical lines, trailing spaces trimmed on the last row of each line (`panes/grid.rs:655-682`, `:2372-2384`). An `ansi` variant exists; issues [#5089](https://github.com/zellij-org/zellij/issues/5089) and [#5256](https://github.com/zellij-org/zellij/issues/5256) report it mangling sequences and adding blank lines.
- **File:** `std::env::temp_dir()/<uuid>.dump`, written with `File::create` (`tab/mod.rs:7006-7007`, `os_input_output.rs:719-727`). Nothing deletes it. [#2661](https://github.com/zellij-org/zellij/issues/2661) reports the files are left world-readable in `/tmp`.
- **Editor:** `scrollback_editor`, else `EDITOR`, else `VISUAL`, else `vi` (`os_input_output.rs:163-169`, `screen.rs:10572-10578`). That's the reverse of the usual `VISUAL`-first order. The string is split on whitespace, with no shell and no quote handling (`os_input_output.rs:122-139`). [#5670](https://github.com/zellij-org/zellij/issues/5670): "$EDITOR isn't respected".
- **Line number:** passed only to editors recognized by basename: `+N file` for vim, nvim, emacs, nano and kak, `file:N` for hx and helix, nothing for anything else (`os_input_output.rs:181-201`).
- **Placement:** the editor pane takes the original's slot, and the original is stored in a suppressed-panes map keyed by the editor's ID, to be swapped back when the editor closes (`tab/mod.rs:3522-3586`, `:9220-9230`). The restore path wasn't read. [#2286](https://github.com/zellij-org/zellij/issues/2286) reports panes freezing after the editor.

## Herdr

Source: herdrdev/herdr at `2713feb`.

- **Binding and API:** `prefix+e`, config key `keys.edit_scrollback` (`src/config/model.rs:1159`), and `Method::PaneEditScrollback`, focused pane only (`src/app/api/panes.rs:212-230`).
- **Dump:** `recent_unwrapped_text_snapshot(usize::MAX)`: all retained history, wrapped lines joined (`src/app/custom_commands.rs:307-335`).
- **File:** `create_new` with mode `0o600`, retried on collision (`custom_commands.rs`, `write_scrollback_temp_file`). The path is registered with the overlay pane and removed when the pane closes. The shell wrapper also runs `rm -f` after the editor.
- **Editor:** on Linux and macOS, `/bin/sh -c 'eval "${EDITOR:-vi} \"$scrollback_file\""'`. `VISUAL` is not read (`src/platform/linux.rs:319`, `src/platform/macos.rs:214-220`). On Windows, `VISUAL`, `EDITOR`, `notepad.exe` (`src/platform/windows.rs:1460-1483`).
- **Placement:** `spawn_overlay_argv_command` splits the focused pane, sets the tab's `zoomed = true`, and records the previous focus and previous zoom state so they are restored when the editor exits (`custom_commands.rs:441-525`).

## tmux

Source: tmux/tmux at `523ac69` (main, the 3.8 changes; not a tagged release).

- **Binding:** in copy mode, `e` (vi: `M-e`) opens the selection if there is one, else the last command's output from OSC 133 prompt marks (`key-bindings.c:611`, `:707`). It does not open the whole history.
- **Editor:** the `editor` server option, default `vi`, overridden at startup by `VISUAL` then `EDITOR` (`options-table.c:433-437`, `tmux.c:585-590`).
- **Placement and file:** `spawn_editor` writes a `mkstemp` file in `/tmp`, opens a centred modal floating pane at 90% of the window that floats over a zoomed pane, and runs `editor path` with cwd `/tmp` (`spawn.c:651-790`). The file is read back and unlinked on exit, but copy mode's callback just frees it, so edits go nowhere (`window-copy.c`, `window_copy_open_done`).
- **Before 3.8:** users script it, for example `capture-pane -pJS -` into a file, then `display-popup -E "$EDITOR file"`. `-J` joins wrapped lines (`cmd-capture-pane.c`).

## kitty and WezTerm

Source: kovidgoyal/kitty at `4f7dea3`, wez/wezterm at `e68a8ec`.

- **kitty `show_scrollback`:** pipes the history plus screen, with colors, on stdin to `scrollback_pager`, default `less --chop-long-lines --RAW-CONTROL-CHARS +INPUT_LINE_NUMBER` (`kitty/options/definition.py:735-752`). It opens as an overlay window on the current window (`kitty/boss.py:2617-2631`). No temp file.
- **kitty placeholders:** `INPUT_LINE_NUMBER`, `CURSOR_LINE` and `CURSOR_COLUMN` are substituted into each argv element after splitting (`boss.py:2617-2626`). This lets the user decide how their editor takes a line number, instead of Ship knowing each editor. `INPUT_LINE_NUMBER` is the line at the top of the viewport (`kitty/window.py:2292-2306`).
- **kitty wrapping:** wrapped rows come out as separate lines (`kitty/line.c:1145-1177`, `window.py:2292-2295`), which keeps line numbers equal to screen rows.
- **WezTerm:** no built-in action. The `pane:get_lines_as_text` docs include a Lua example that writes the scrollback to `os.tmpname()`, spawns `vim` in a new window, and deletes the file after `wezterm.sleep_ms(1000)` because the spawn can't be awaited. `get_logical_lines_as_text` joins wrapped rows, with a 1024-cell cap per line (`mux/src/pane.rs:440-520`).

## What Ship has

All on main at `2fa992f`.

- **History read:** `server.pane.read` and `GET /api/v0/panes/{id}/text` take `lines`, `unwrap` and `ansi` (`crates/ship-core/src/protocol.rs:270-278`). The formatter is Ghostty's: plain or VT, `with_unwrap` joins soft-wrapped lines, trailing spaces trimmed (`vendor/ratatui-ghostty/src/session.rs:869-895`). It's the last `lines` rows; history is capped by `scrollback_bytes`, 10 MB by default (`crates/ship-core/src/defaults.toml:12`). Reading works on exited panes too (`crates/ship-server/src/pane.rs:507-518`).
- **Spawning:** an explicit argv runs directly, without a shell (`crates/ship-server/src/pane.rs:299-311`). Nothing in `crates/` reads `EDITOR` or `VISUAL` today.
- **Zoom:** server-authoritative, `Tab.zoomed: Option<IdOf<Pane>>` (`crates/ship-core/src/model.rs:28-29`). Zoom-hidden panes keep their last terminal size (`crates/ship-server/src/state.rs:289-290`), so a split and zoom in one commit never resizes the original pane's program. Splits, closes, swaps and resizes clear zoom (`crates/ship-server/src/state/layout.rs:57`, `:83`, `:116`, `:158`, `:230`), and selecting a hidden pane unzooms.
- **Exit:** `exit_behavior` decides whether a pane closes when its program exits (`crates/ship-core/src/model.rs:66-89`). A closed pane's selection moves to its layout successor (`crates/ship-server/src/state.rs:209-239`).
- **Several clients:** the layout and zoom are shared, so another client on the same tab sees the editor too. Selection is per attachment.

## Options for Ship

### A. Split and zoom, like Herdr (leaning)

`server.pane.edit_scrollback` reads the pane's history, writes the file, splits the pane with the editor, and zooms the editor, all in one commit. When the editor exits, its pane closes regardless of `exit_behavior`, and the tab's previous zoom comes back. The requesting client's selection returns to the original pane.

- Built from what exists. New state is a record of "this pane is an editor for that pane, with these temp files and this previous zoom", which the server drops when the editor pane goes.
- The original program isn't resized, because hidden panes keep their size.
- Other clients on the tab see the editor zoomed. That's consistent with zoom being shared today, but it's the one surprising part.
- Anything that clears zoom while the editor is open (a split, a resize, selecting the original) leaves the editor as an ordinary pane in the layout. It still closes and cleans up on exit.

### B. A new tab

The editor opens in a new tab beside the current one and closes on exit. No layout or zoom handling, but it moves the user away from the pane, and selection repair after the tab closes lands on a neighbouring tab unless the server restores it. Simplest to build and least like the references.

### C. Replace in place, like Zellij

A suppressed-pane slot in the layout. This is a new idea in Ship's model that splits, closes, restore and every client would have to understand. Zellij's open freeze report sits in this path. Not worth it for one feature.

### D. Wait for floating panes

tmux and kitty both put the editor over the pane instead of in the layout, and #86 would give Ship the same. Doing A now and moving the editor into a floating pane when #86 lands keeps the action and config unchanged.

## Smaller decisions

- **Editor order and running it:** `scrollback_editor` if set, else `VISUAL`, else `EDITOR`, else `vi`, read from the server's environment. Run it as `sh -c '<editor> "$1"' sh <file>`, as Git does, so values like `code --wait` or `emacsclient -t` work. Zellij's whitespace split and Herdr's `EDITOR`-only lookup are both reported as surprising. Windows has no `sh`; best effort there.
- **Text:** plain, wrapped lines joined, the whole retained history. Ship's read already does this with `unwrap` and `lines` set to the total. Colors stay out, since the ANSI path is where Zellij's open bugs are and editors show escapes as junk.
- **Temp file:** create-new, mode 0600, in a Ship-owned directory, deleted when the editor pane closes. A sweep at server start removes files left by a crash. This avoids Zellij's world-readable leftovers.
- **Edits:** discarded, as everywhere else. Saving the file does nothing to the pane.
- **Line number:** leave out at first. Once wrapped lines are joined, file lines no longer match screen rows, so a line number needs mapping. kitty's placeholder approach (the user writes `+{line}` or `{file}:{line}` themselves) is the cheap way to add it later without Ship knowing editors. It's still a small template, which [Prompts and plugins](prompts-and-plugins.md) warns tends to grow.

## Open

- Placement: A now, moving to floating panes with #86, or wait for #86.
- Whether other clients on the tab should see the editor zoomed, or whether that's acceptable until floating panes.
- Action and config names. `server.pane.edit_scrollback` and `[server.defaults.pane] scrollback_editor` follow Zellij and Herdr.
- Where the Ship-owned temp directory lives: the system temp dir with a per-user subdirectory, or the state directory.

## Evidence

Four read-only agents read shallow clones of each project at the SHAs above and returned file and line citations. I re-read the claims that matter for the decision: Zellij's editor order, line-number list and missing cleanup; Herdr's overlay, zoom restore, temp file mode and Linux editor path (an agent report had wrongly suggested Linux was unsupported); and every Ship citation. Nothing was run. Behavior of the Zellij restore path, kitty overlay exit, and the multi-client view in Zellij comes from reading code only.
