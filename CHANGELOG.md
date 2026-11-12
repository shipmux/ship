# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/shipmux/ship/releases/tag/v0.1.0) - 2026-11-12

Ship is a terminal multiplexer, like tmux or Zellij, with a sidebar that shows what your coding agents are doing. Your programs run on a background server, so closing the window doesn't stop them. This is the first release, so everything is new, and these are the parts I'd try first.

### Highlights

- **Agent status in the sidebar**: see which agents are working, which are blocked on a question and which finished while you were looking elsewhere. A sound plays when one finishes or needs you, and `alt-a` jumps to it. Claude Code, Codex, Pi and crush report their own state, so there's nothing to set up for each agent. See [Agents](https://shipmux.github.io/ship/docs/v0.1.0/agents/).
- **Close the window, keep the work**: detach with `alt-q` or just close the terminal, then run `ship` again and the same programs are still there. You can also open several windows onto the same work. See [Windows and the server](https://shipmux.github.io/ship/docs/v0.1.0/windows-and-the-server/).
- **Nested tabs, splits and stacks**: keep a whole project under one tab with smaller tasks nested below it, and split panes when you want things side by side. `alt-s` stacks a shell in front of a busy pane without making anything smaller. See [Tabs](https://shipmux.github.io/ship/docs/v0.1.0/tabs/) and [Panes](https://shipmux.github.io/ship/docs/v0.1.0/panes/).
- **A CLI for scripts and agents**: `ship` commands open panes, send text or keys, wait for an agent's status report and read the output. An agent can use them to hand a task to another agent and read back the answer, with no SDK or plugin. See [Scripting Ship](https://shipmux.github.io/ship/docs/v0.1.0/scripting/) and [Handing work to another agent](https://shipmux.github.io/ship/docs/v0.1.0/scripting/handing-work-to-another-agent/).
- **Your layout back after a restart**: after a server restart or a reboot, your tabs, splits, stacks and directories come back with fresh shells. The programs that were running don't restart. See [Saved tabs](https://shipmux.github.io/ship/docs/v0.1.0/saved-tabs/).
- **Scrollback in your own editor**: `alt-e` opens a pane's history in your editor, so you search and copy with the keys you already know. Ship has no built-in search or keyboard copy mode. See [Edit scrollback](https://shipmux.github.io/ship/docs/v0.1.0/panes/#edit-scrollback).

### Install

On Linux (x86_64 or arm64) or macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/shipmux/ship/main/install.sh | sh
ship
```

The script puts `ship` in `~/.local/bin`, and `ship update` gets later releases. On macOS, set your terminal to use Option as Alt, or Ship's keys will type special characters instead. Windows is best effort and has no prebuilt release, so there you [build from source](https://shipmux.github.io/ship/docs/v0.1.0/building-from-source/), or use WSL.

### Added

- keep clients working when a newer server adds to the stream
- *(examples)* add the git plugin, tagging branches and pull requests
- *(watch)* add ship watch, the client's attach stream as JSON lines
- *(install)* add install.sh, the Intel macOS release and install docs
- *(updates)* show the version row and one-time update note in the client
- *(updates)* add ship update
- *(updates)* add release checking and shared cache
- *(updates)* add version probes and protocol mismatch guidance
- run server commands from bindings
- run client commands from bindings
- build on Windows over named pipes
- connect clients to named sockets
- serve named instances over Unix sockets
- speak HTTP/2 to the server, falling back to HTTP/1.1
- explain refused requests with field pointers
- *(diagnostics)* trace the keystroke and screen path
- *(vendor)* take program status and prompts from libghostty's callbacks
- gather pane settings under [server.defaults.pane]
- *(website)* a large app mark with the crew on deck, used in the README
- print shell completion scripts with ship completions
- print the built-in defaults with ship config defaults
- print CLI failures as JSON
- let pane wait hold for a first report and document pane automation
- log lifecycle events and request errors
- play sounds when agents finish or need attention
- restore saved tabs at startup
- record opt-in client and server timelines
- change server log filters without restarting
- save per-run logs and configure startup levels
- add right-click passthrough to mouse-aware panes
- add configurable context menus
- add sidebar tab drag-and-drop
- add border drags and client-local sidebar resizing
- add client view selection commands
- use single-action key bindings with optional hints
- add ship pane read
- start key splits in the pane's directory and add child tabs
- add pane zoom, swap and edge resize
- add the tab sidebar and status-line overlay
- draw split panes together with directional focus
- implement server-owned pane splits
- keymap and modes, with Alt chords to actions and C-b gone
- config path and the server's configured shell
- one command tree for the CLI's tab and pane commands
- start the default server with a starter tab
- replace sessions with top-level tabs
- the full-screen client
- input, view and size for attached panes
- pane screens on the attach stream
- every pane runs a program
- optional tab and pane names, vendor ratatui-ghostty
- compress attach streams and shut down cleanly
- say why an attach stream ends
- reconnect observers after a dropped connection
- add per-observer selection and session switching
- add live session observation with ship attach
- add logical panes through the CLI and /api/v0
- add recursive tabs and tab moves through the CLI and /api/v0
- add session CRUD through the CLI and /api/v0
- complete schema inspection and client-update verification
- add the verified health exchange and local server lifecycle
- establish the verified Ship foundation

### Fixed

- send unambiguous keys and stop servers on any protocol ([#120](https://github.com/shipmux/ship/pull/120))
- run a key's list in order, and size panes to the client's area

### Other

- crate metadata and ignore profiling builds
- cover releases and ship update, and make the guide read plainer
- pre-release fixes for switching, attach and cold start
- refresh guides and source build instructions
- finish consolidate slice 1 cleanups and verify
- consolidate slice 1 shared shapes, lints and relay
- tidy up release loose ends
- explain pane defaults and exit behavior
- give each page a scope and move the docs rules to docs/agents/docs.md
- trim the README to a pitch and links, move keys and the CLI demo into the guide
- rewrite the README and guide intro in plainer words, with real examples
- split the guide into docs/site pages
- name the health service in one shared constant
- name the completions arguments and take the binary name from Cargo
- point links at the shipmux organization
- license Ship under GPL-3.0-or-later
- rewrite the guide around tasks and refresh the README
- document agent status and record program status slice 5
- document session restore and sync specs
- complete server-selection documentation and verification
- rewrite README for new users and move detail to a guide
- finish scrollback docs and preserve deferred design notes
- read the config with toml and bind keys to lists
- clear workspace Clippy warnings
- keys and configuration in the README; close 3.5
- describe running panes in the README and drop sessions from spec purposes
- describe top-level tabs instead of sessions
- rewrite README for humans and add the ship logo
