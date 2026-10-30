# Troubleshooting

<!--
Scope: Where the logs are, raising the log level, and recording a timeline for a bug report.
Not here: fixes for one feature's problems, which go on that feature's page.
-->

## Logs

The server writes logs to `~/.local/state/ship/logs/default/` (under `$XDG_STATE_HOME` if set; a named server uses its name in place of `default`). To follow along:

```sh
tail -F ~/.local/state/ship/logs/default/current.log
```

Windows log to `~/.local/state/ship/logs/client/`. Problems that don't show up on screen end up in one of these: a saved directory that disappeared is in the server log, and a sound file that wouldn't load is in the client log. Old logs are cleaned up automatically.

Logs don't record pane input or output, but programs started by [run actions](configuration.md#running-programs-from-keys) write their output here. Error messages can also include program names and paths, so look logs over before sharing.

## More detail

Turn up the server's logging without restarting it, and put it back when you're done:

```sh
ship server log debug
ship server log --reset
```

To start a server or window with more logging, set `SHIP_LOG`:

```sh
SHIP_LOG=debug ship
```

If a program renders wrong in a pane, the server log includes Ghostty's warnings about terminal features it doesn't support, which helps tell a terminal gap from a Ship bug.

## Recording a timeline

For something that feels slow, record a timeline and open it in [Perfetto](https://ui.perfetto.dev):

```sh
SHIP_TRACE=/tmp/ship-trace ship server --name trace --starter
SHIP_TRACE=/tmp/ship-trace ship --server-name trace   # another terminal
```

Do the slow thing, detach with `alt-q`, then stop that server with `ship --server-name trace server stop`. The timelines are in `/tmp/ship-trace/`. They aren't size-limited, so keep recordings short.
