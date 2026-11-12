# Scripting Ship

<!--
Scope: What every `ship` command shares for scripts (JSON output, explicit IDs, the `--pane` default) and the list of recipes.
Not here: the recipes themselves.
-->

Use `ship` commands to open panes, send work and read the results. Commands take explicit IDs, so changing your selection doesn't redirect a script.

```sh
ship pane get --pane "$PANE" | jq .process  # inspect a pane as JSON
ship pane read --pane "$PANE" --lines 40   # read its output as text
```

Commands that return tabs, panes or views print JSON, `pane read` prints text, and commands with no result, such as closing a pane or sending without `--wait`, print nothing on success. [Command errors](reading-command-errors.md) explains failures.

In these examples, `$PANE` holds an ID from `ship pane create` or `ship tab list`, and [`jq`](https://jqlang.org) reads fields from JSON. Inside a Ship pane, you can leave out `--pane` to target that pane, not whichever pane gets selected next.

Tab, pane and view commands start the named server if it isn't running, just like bare `ship`. To check without starting one, use `ship server status`. Passing `--server-url` also connects only and never starts a server.

- [Reading output](reading-output.md)
- [Typing into a pane](typing-into-a-pane.md)
- [Handing work to another agent](handing-work-to-another-agent.md)
- [Reading command errors](reading-command-errors.md)
- [Steering a window](steering-a-window.md)
- [Writing a plugin](writing-a-plugin.md)
