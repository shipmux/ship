<!--
Scope: People deciding whether to try Ship: what it is and why, an honest split between what works and what isn't built, installation, and links into the guide.
Not here: keys, configuration and CLI walkthroughs (docs/site/).
-->

<div align="center">
<img src="website/public/marks/app-large.svg" height="216" width="216" alt="A pixel sailing ship with three crewmates on deck">

**Ship - A terminal workspace for you and your coding agents**

[Overview](#overview) •
[What works today](#what-works-today) •
[Installation](#installation) •
[Docs](#docs) •
[License](#license)

</div>

> [!NOTE]
> **Ship is a work in progress.** There are no releases yet, but you can build it from source and try it today.

[![Claude Code and Pi working side by side in nested tabs, with the sidebar marking each agent's state](https://shipmux.github.io/ship/demo/waterline.gif)](https://shipmux.github.io/ship/)

## Overview

Keep your editor, shells and coding agents in one place, without switching through every tab to find out who needs you.

Ship is a terminal multiplexer, like tmux or Zellij, with a sidebar that shows what your agents are doing. Your work runs on a background server, so closing the window doesn't stop it. Open Ship again and carry on where you left off.

```text
─ agents ─────────────
▲ claude blocked
ship › api
● codex working
ship › docs
◆ pi unseen
site
```

Claude needs an answer, Codex is still working, and Pi finished while you were looking elsewhere. Jump to the one that needs you instead of checking them all.

You can keep a whole project under one tab, split panes for things you want side by side, or stack a temporary shell over a busy pane without squeezing everything smaller. And your agents can use the same `ship` commands you do to open panes, hand work to each other and read the results. No SDK or plugin needed. [See the handoff workflow](docs/site/scripting/handing-work-to-another-agent.md).

### Why another multiplexer?

[Herdr](https://github.com/herdrdev/herdr) is a big inspiration. Ship aims to cover most of its daily-use value with less code to maintain, using libraries for the hard parts, including terminal emulation from [Ghostty](https://ghostty.org), and owning how tabs, panes and agents fit together.

The name started as a joke about shipping software faster, and the ship and crew theme grew out of that.

## What works today

- **See who needs you.** The sidebar shows agent status, with sounds when work finishes or needs an answer. Claude Code, Codex, Pi and crush report their state without a Ship-specific integration. [Agents](docs/site/agents.md)
- **Make room for a whole project.** Nest tabs, split or stack panes, zoom in, and move or rename things with the keyboard or mouse. [Tabs](docs/site/tabs.md) · [Panes](docs/site/panes.md)
- **Leave the window, not the work.** Detach and come back to the same running programs, or open several terminal windows onto one server. [Windows and the server](docs/site/windows-and-the-server.md)
- **Get your layout back after a restart.** Tabs, stacks, splits and directories return with fresh shells. Your old programs don't restart. [Saved tabs](docs/site/saved-tabs.md)
- **Search output in your own editor.** Open a pane's scrollback in your editor, or drag to copy straight from the terminal. [Scrollback and copy](docs/site/scrolling-copying-and-the-mouse.md)
- **Give scripts and agents the controls.** Create panes, send prompts or keys, wait for status reports and read output through the CLI. [Scripting Ship](docs/site/scripting/README.md)
- **Keep your habits.** Rebind keys, add a tmux-style prefix, customize menus or run your own scripts from a key. Config changes apply when you save. [Configuration](docs/site/configuration.md)

Agents report through the [Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status). Anything that reports its own status can work with Ship; there's no per-agent plugin list to maintain.

Linux and macOS are the priority platforms, and both have been used successfully. Windows is best effort.

### Not built yet

- Releases and packages. Installation is from source for now.
- Built-in history search and keyboard copy mode. Use [scrollback in your editor](docs/site/panes.md#edit-scrollback) instead.
- Resuming agent conversations or rerunning programs after a server restart. Saved tabs start fresh shells.
- A combined view of servers on several machines. You can already [run Ship over SSH or forward a remote socket](docs/site/windows-and-the-server.md#using-ship-on-another-machine).

## Installation

Install the [build prerequisites](docs/site/building-from-source.md#prerequisites), including Rust, Zig 0.17 and `git`, then:

```sh
git clone https://github.com/shipmux/ship.git
cd ship
cargo install --path crates/ship
ship
```

[Building from source](docs/site/building-from-source.md) covers platform setup, local builds, updates and build problems. The [first session](docs/site/README.md#first-run) picks up once Ship is installed.

## Docs

The [guide](docs/site/README.md) walks through a first session. From there:

- [Keys](docs/site/keys.md) for the everyday controls
- [Configuration](docs/site/configuration.md) to make it yours
- [Scripting Ship](docs/site/scripting/README.md) for scripts and agent handoffs
- [Troubleshooting](docs/site/troubleshooting.md) when something isn't working

## License

Ship is licensed under the [GNU General Public License v3.0 or later](LICENSE). The vendored [`ratatui-ghostty`](vendor/ratatui-ghostty) keeps its own MIT license.

Contributions are accepted under `MIT OR Apache-2.0`, as the pull request template states, so the project can change its license later without asking each contributor.
