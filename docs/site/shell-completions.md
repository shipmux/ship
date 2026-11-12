# Shell completions

<!--
Scope: Loading `ship completions` in each supported shell.
Not here: the commands themselves (`ship --help`).
-->

`ship completions <shell>` prints a script that lets your shell tab-complete Ship's commands, flags and fixed values. It supports bash, zsh, fish, elvish and PowerShell, and the setup for bash, zsh and fish is below.

For bash, add this to `~/.bashrc`:

```bash
source <(ship completions bash)
```

For zsh, add this to `~/.zshrc` after `compinit`:

```zsh
source <(ship completions zsh)
```

For fish, run this once:

```fish
ship completions fish > ~/.config/fish/completions/ship.fish
```

The bash and zsh lines regenerate the script each time a shell starts, so it stays current after you update Ship. The fish file doesn't, so rerun that command after an update. Completion covers names Ship knows ahead of time, so it doesn't look up tab or pane IDs.
