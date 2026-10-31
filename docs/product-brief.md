# Ship product brief and roadmap

Ship keeps shells, editors and coding agents together in a terminal. Work keeps running after you close the window, and the sidebar shows which agents need you.

The aim is to recover the daily-use value of [Herdr](https://github.com/herdrdev/herdr) with much less owned code and maintenance. Ship is Cyan's side project, not a feature-for-feature rewrite. The name started as a joke about shipping software and grew into the ship and crew theme.

This is product direction, not an implementation contract or a feature reference. The [guide](site/README.md) explains today's behavior, and the [README](../README.md#not-built-yet) lists current limitations. Only locked OpenSpec papers with Cyan's recorded approval authorize implementation. Roadmap items below have no calendar commitments.

## Keep the core small

Ship owns how tabs, panes and agent status fit together. Libraries handle terminal emulation, PTYs, serialization, transport, sound and other solved problems. Use Rust types, derives and macros to express each rule once, not a generic framework for hypothetical features.

The budget is roughly 20,000 lines of implementation Rust, a conservative ceiling rather than a target. Working behavior, clear boundaries and maintenance burden matter more than an exact count. Report implementation Rust and tests separately.

Profile before consequential architecture choices. Terminal diffs looked cheaper, but compressed full snapshots measured smaller and faster in the original experiments. For everyday work, judge by feel too. A workflow that suddenly feels slow is a bug even when it works. The [transport research](research/terminal-transport-handoff.md) preserves those measurements, not a current protocol specification.

## Local work first

The server owns tabs, panes, running programs and shared state. Each attached window has its own selection, held by the server, while navigation, modes and sidebar presentation stay client-local. There is no workspace entity; top-level tabs are the top-level container.

The client reaches the server through explicit commands, responses and events. Live resources stay private to the server so the boundary can cross a network later. Today's server uses an owner-restricted local socket rather than the early loopback TCP design. [Windows and the server](site/windows-and-the-server.md) covers local use and SSH access.

Keep these promises separate:

| What happens | What Ship preserves |
| --- | --- |
| A window closes but the server stays running | The existing programs and their output |
| The server or machine restarts | Saved tabs, pane layouts and directories, with fresh shells |
| An agent conversation needs resuming | Not handled by Ship |

Linux is Cyan's primary environment, and Ship has worked in his Linux use so far. Linux and macOS are priority platforms; others are best effort. Choose cross-platform dependencies, validate on the machine being used, and distinguish checks run for a change from earlier platform experience.

## The daily-driver foundation

The original milestones covered terminal work, detach and restore, agent awareness, and scripting. Those capabilities now have dedicated guide pages rather than a second feature list here:

- [Tabs](site/tabs.md) and [Panes](site/panes.md) cover organizing work, stacks, resizing, zoom and renaming.
- [Saved tabs](site/saved-tabs.md) explains restart recovery and its limits.
- [Agents](site/agents.md) explains status, attention and notifications. Agents report through the Program Status Protocol; Ship does not maintain per-agent screen rules.
- [Scripting](site/scripting/README.md) covers creating panes, sending work, waiting and reading results.
- [Configuration](site/configuration.md) covers keys, modes, menus and running external programs.

The later release-readiness work included keyboard tab movement, UI renaming and editing scrollback. These are also implemented and documented in the guide. Opening scrollback in an editor replaces the earlier plan for built-in search and keyboard copy mode.

## Roadmap

These are directions to design, not promises that their interfaces are settled. Their issues hold the remaining questions.

### Scripts as extensions

Let ordinary programs prompt, compute values and call `ship`, rather than adding an embedded scripting language. Client and server run actions are implemented; the remaining pieces include:

- Floating panes for prompts, pickers and quick shells ([#86](https://github.com/shipmux/ship/issues/86)).
- Display-only metadata labels on tabs and panes ([#87](https://github.com/shipmux/ship/issues/87)). Scripts would report things such as Git branches and pull request status, rather than Ship owning Git integration.
- Thin plugin packaging once the underlying command surface has settled, not a plugin runtime or marketplace service.

The intended check is that a script can show a branch or pull request status in the sidebar and open a picker without a Ship code change. Whether floating panes are shared or per window, and how programs report metadata, remain design questions.

### Several machines

Show several servers in one client, reached over SSH and listed together in the sidebar ([#88](https://github.com/shipmux/ship/issues/88)). Use `~/.ssh/config` rather than introducing another SSH configuration system. Restarting a server from the client ([#89](https://github.com/shipmux/ship/issues/89)) and self-update ([#36](https://github.com/shipmux/ship/issues/36)) belong to this direction too.

A dropped remote connection should not stop remote work or make local work unusable. Reconnection must not silently duplicate commands. Delivery, retry and remote access rules need design before implementation. Today's manual SSH socket forwarding is not this multi-server interface.

## Deliberately outside the core

Use Git or Worktrunk in ordinary panes for worktree management. Keep Git status in scripts rather than building it into Ship.

Agent conversation resume, hooks for agents without protocol support, advanced terminal graphics, saved-screen replay and live server upgrades are deferred. A browser client is later work too; try the native client through a web terminal before committing to a separate client.

Built-in search and keyboard copy mode are not the current direction. Use [scrollback in your editor](site/panes.md#edit-scrollback) instead. Broader themes and elaborate notification rules are not planned beyond a small configuration surface.

## Earlier design evidence

The [research index](research/README.md) links the original dependency probes and comparisons. They explain why options were considered, but their descriptions of unbuilt features, temporary paths and open decisions reflect the time they were written. The code and current OpenSpec records take precedence over those snapshots.
