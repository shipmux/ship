# Windows and the server

<!--
Scope: The window and server split, starting and stopping the server, where its socket is, named servers, and using Ship on another machine.
Not here: saved tabs (saved-tabs.md), the HTTP protocol (openspec/specs/).
-->

Ship runs in two parts. A background server owns your tabs, panes and the programs in them. Running `ship` opens a window onto that server. Closing the window, or pressing `alt-q`, leaves everything running, and running `ship` again brings you back.

The first `ship` starts the server for you. It restores your [saved tabs](saved-tabs.md), or opens one tab with a shell if there's nothing to restore. `ship server status` tells you whether a server is running, and `ship server stop` shuts it down, ending every program inside.

You can open Ship from several terminals at once. The windows share the work: the same tabs, the same programs, the same output. Each window picks its own tab and pane, folds its own sidebar and keeps its own mode. Panes are sized to fit the smallest window looking at that tab, so a bigger window shows some empty space around them.

If a window loses its connection to the server, it reconnects on its own.

## Where the server lives

The server listens on a socket, a file that only your user account can open. `ship server status` prints its path in `socket`. On Linux it's usually `$XDG_RUNTIME_DIR/ship/default.sock`, and on macOS it's under `$TMPDIR/ship/`. Other people on the same machine get their own server and can't connect to yours.

## Running more than one server

Each server has a name, and `default` is the one bare `ship` uses. To keep a separate set of tabs, pick another name:

```sh
ship --server-name scratch
```

That starts the `scratch` server if it isn't running, with its own [saved tabs](saved-tabs.md) and logs. CLI commands take `--server-name` too, or set `SHIP_SERVER_NAME` once. Inside a pane, commands reach the server that owns the pane unless you pass `--server-name` or `--server-url`.

## Using Ship on another machine

The simplest way is to run Ship over SSH:

```sh
ssh otherbox -t ship
```

To use your local terminal's window instead, forward the other machine's socket. Run `ship server status` there to find its path, then:

```sh
ssh -N -o StreamLocalBindUnlink=yes -L ~/.ship-otherbox.sock:/run/user/1000/ship/default.sock otherbox
```

Then, in another terminal:

```sh
ship --server-url unix://$HOME/.ship-otherbox.sock
```

Run `ship` on the other machine once first so its server is up. Passing `--server-url` makes Ship connect only to that socket, so it won't quietly start a local server if the tunnel isn't up. All the CLI commands take `--server-url` too, or set `SHIP_SERVER_URL` once. `StreamLocalBindUnlink` lets you rerun the `ssh` command after it exits, since it would otherwise refuse to replace the socket file it left behind.

The server refuses web pages and requests passed on by proxies, so opening it in a browser or putting it behind a proxy such as Tailscale Serve or Caddy won't work. Use SSH to reach it from elsewhere.

Sounds and clipboard copies happen on the machine you're sitting at. Through a tunnel, `ship server stop` stops the server but may report that it's still running, because the tunnel keeps accepting connections after the server is gone.
