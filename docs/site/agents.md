# Agents

<!--
Scope: How agents show up: the sidebar entry and its states, sounds and notifications, and what to check when a status looks wrong.
Not here: agent settings (configuration.md), driving agents from scripts (scripting/).
-->

See which agents are busy and which need an answer without visiting every pane. The lower half of the sidebar shows entries like this:

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

Press `alt-a` to jump to the next agent that needs you. If none needs you, it cycles through all agents. Click an entry to go to its pane.

**Unseen** is tracked per window. An agent that finishes while its pane isn't on screen in a focused window stays `◆` until you look at it. A window you attach later starts with nothing marked unseen.

## Which agents appear

Agents report their own state using [OSC 7501, the Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status). Claude Code, Codex, Pi and crush already do, so they work with no setup. Ship has no list of agents to update; programs that report their own status work without an agent-specific integration.

Each pane's reporting program gets one entry. Subagents reported within that program don't get separate entries.

## Sounds and notifications

Ship plays a sound when an agent finishes, or when one gets blocked or hits an error, as long as you're not already looking at it. It waits a second first, so an agent that pauses briefly and carries on stays quiet. With no working audio device, Ship carries on without sound.

Set `popups = true` to also get desktop notifications through your terminal, like `claude needs you · ship › api: Apply these changes?`. Ghostty and iTerm2 show them. The [agent settings](configuration.md#agent-settings) cover sounds, popups and what each entry shows.

## When an agent's status looks stuck

An agent that crashes can't report that it stopped, so its last state hangs around. Ship can clear stale working or blocked states when your shell shows its next prompt, but only if the shell marks its prompts. Error states stay visible. Bash and zsh don't mark prompts by default. For zsh, add this to `~/.zshrc`:

```zsh
_prompt_mark() { print -n '\e]133;A\e\\' }
precmd_functions+=(_prompt_mark)
```

Agents running inside tmux inside Ship can't report anything, because tmux doesn't pass the protocol through.
