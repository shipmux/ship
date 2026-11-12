# Agents

<!--
Scope: How agents show up: the sidebar entry and its states, sounds and notifications, and what to check when a status looks wrong.
Not here: agent settings (configuration.md), driving agents from scripts (scripting/).
-->

The lower half of the sidebar lists your agents, so you can see which are busy and which need an answer without visiting every pane. An entry looks like this:

```text
▲ claude blocked
ship › api
```

| Icon | State | Meaning |
| --- | --- | --- |
| `▲` red | blocked | Waiting on you, like a permission prompt |
| `✖` magenta | error | Something went wrong |
| `◆` cyan | unseen | Finished while you weren't looking |
| `●` yellow | working | Busy |
| `○` green | idle | Done and seen, or waiting for a prompt |

The ones that need you sort to the top. Tabs show the most urgent icon from anything inside them, so you can spot a blocked agent even in a folded tab.

Press `alt-a` to jump to the next agent that needs you, and if none does it cycles through all of them. Click an entry to go to its pane.

**Unseen** is tracked per window. An agent that finishes while its pane isn't on screen in a focused window stays `◆` until you look at it, and a window you attach later starts with nothing marked unseen.

## Which agents appear

Agents report their own state using [OSC 7501, the Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status). Claude Code, Codex, Pi and crush already do, so they work with no setup. Ship has no list of agents to update, so any program that reports its own status works without an agent-specific integration.

Each pane's reporting program gets one entry, and subagents reported within that program don't get entries of their own.

## Sounds and notifications

Ship plays a sound when an agent finishes, or when one gets blocked or hits an error, as long as you're not already looking at it. It waits a second first, so an agent that pauses briefly and carries on stays quiet. If there's no working audio device, Ship just skips the sound.

Set `popups = true` to also get desktop notifications through your terminal, like `claude needs you · ship › api: Apply these changes?`. Ghostty and iTerm2 show them, and the [agent settings](configuration.md#agent-settings) cover sounds, popups and what each entry shows.

## When an agent's status looks stuck

An agent that crashes can't report that it stopped, so its last state hangs around. Ship can clear a stale working or blocked state when your shell shows its next prompt, but only if the shell marks its prompts, and bash and zsh don't by default. Error states stay visible. For zsh, add this to `~/.zshrc`:

```zsh
_prompt_mark() { print -n '\e]133;A\e\\' }
precmd_functions+=(_prompt_mark)
```

Agents running inside tmux inside Ship can't report anything, because tmux doesn't pass the protocol through.
