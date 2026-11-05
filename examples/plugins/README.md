# Example plugins

A Ship plugin is a script that acts like any other client: it reads Ship's state from `ship watch` and puts text on tabs and panes with `ship pane set` and `ship tab set`. There's nothing to install. [Writing a plugin](../../docs/site/scripting/writing-a-plugin.md) covers both commands.

## git.sh

Tags each pane in a git checkout with its branch, and with the state of the branch's pull request on GitHub:

```text
 labels-and-watch
   labels-and-watch · ○ #112 ✓
 self-update
   self-update · ◆ #111
```

It needs `jq` and `git`, and [`gh`](https://cli.github.com) signed in for pull requests; without `gh` it tags branches only. Start it in the background once Ship's server is running:

```sh
sh examples/plugins/git.sh &
```

Then show the tags under each tab's name in `~/.config/ship/config.toml`:

```toml
[client.tabs]
rows = [["name"], [{ tag = "git.branch" }, { tag = "git.pr" }]]
```

A tab shows its first pane's tags. The same `{ tag = "git.branch" }` works in `[client.agents] rows`, to show each agent's branch.

`git.branch` is the branch in cyan, or the short commit when it's detached. `git.pr` is the pull request's state, number and checks, colored the way GitHub colors them:

| Pull request | Tag | Color |
|---|---|---|
| open, checks passing | `○ #112 ✓` | green |
| open, checks failing | `○ #112 ✗` | red |
| open, checks running | `○ #112 ●` | yellow |
| draft | `◇ #112 ✓` | grey |
| merged | `◆ #112` | magenta |
| closed | `× #112` | red |

With a [Nerd Font](https://www.nerdfonts.com), `GIT_ICONS=nerd sh examples/plugins/git.sh &` uses GitHub's own icons instead.

Both tags disappear when the pane leaves a checkout, and `git.pr` when the branch has no pull request. The branch follows a `cd` as soon as your shell reports the new folder. A `git switch` shows up when the prompt returns if your shell puts the running command in the window title, as many prompt setups do; otherwise it waits until the pane next changes. Pull requests are checked again every 30 seconds, since CI can finish while the pane sits still.

The script ends when the server stops, so start it again with the server.
