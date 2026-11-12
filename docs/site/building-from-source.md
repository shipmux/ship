# Building from source

<!--
Scope: Build prerequisites, installing Ship from a checkout, building without installing, updating, and common build or install problems.
Not here: first-session keys (README.md), server management (windows-and-the-server.md), packaging or cross-compilation.
-->

Linux and macOS can install a prebuilt release with the [install command](../../README.md#installation) instead. To build from source on Linux or macOS, with the [prerequisites](#prerequisites) installed:

```sh
git clone https://github.com/shipmux/ship.git
cd ship
cargo install --path crates/ship
ship --version
```

Cargo installs `ship` into `~/.cargo/bin` by default. Keep that directory on your `PATH` so you can run it from any terminal.

Run `ship` to open a shell, then follow the [first session](README.md#first-run). On macOS, that includes setting your terminal to send Option as Alt.

## Prerequisites

- [Rust and Cargo](https://www.rust-lang.org/tools/install), preferably installed through `rustup`. The checkout's `rust-toolchain.toml` selects the Rust version, which `rustup` downloads if needed.
- [Zig](https://ziglang.org/download/) 0.17 on your `PATH`, to build Ghostty's terminal library. You don't need to use Ghostty as your terminal.
- `git` and a network connection. The first build downloads Rust dependencies and Ghostty's source.
- A C compiler and linker, plus the platform dependencies below.

You can check the main tools before building:

```sh
cargo --version
zig version
git --version
```

### Linux

Install your distribution's C build tools, `pkg-config` and ALSA's development headers, which Ship needs for sound. On Debian and Ubuntu:

```sh
sudo apt-get update
sudo apt-get install build-essential pkg-config libasound2-dev
```

On Fedora, the ALSA development package is called `alsa-lib-devel`.

### macOS

Install Apple's Command Line Tools if you don't already have them:

```sh
xcode-select --install
```

## Build without installing

From the checkout, build an optimized binary and run it directly:

```sh
cargo build --release -p ship
./target/release/ship --server-name dev
```

This leaves your installed `ship` alone. The command uses a separate [named server](windows-and-the-server.md#running-more-than-one-server) called `dev` to keep the session apart from your usual work. For development, `cargo build -p ship` writes `target/debug/ship` instead.

## Update

An update installs the new version but leaves a running server on the old one. [Restarting the server](windows-and-the-server.md#after-updating-ship) ends its programs, so do it when it suits you.

### Installed from a release

Run `ship update`. It downloads the latest release for your platform, checks it against the release's published checksum and replaces the `ship` you ran. `ship update --check` only says whether there's a newer release.

Ship also looks for new releases on its own, at startup and about once an hour. When there's one, the bottom row of the sidebar changes from a dim `v0.1.0` to this, in cyan:

```text
 v0.1.0 → v0.2.0
```

The first time a release is found on a machine, the status line also says `ship v0.2.0 is available · ≡ updates`. Click the version row, or choose **updates** from the `≡` menu beside `tabs`, to open the updates panel:

```text
┌ updates ───────────────────────────────────────────────── esc ─┐
│ client   v0.1.0                                                │
│ server   v0.1.0                                                │
│ latest   v0.2.0                         checked 2h ago [check] │
│          v0.2.0 is available                                   │
│          the server restarts afterwards to stay in sync        │
│                                                       [update] │
│────────────────────────────────────────────────────────────────│
│ v0.2.0 · 2026-11-20                                            │
│ https://github.com/shipmux/ship/releases/tag/v0.2.0            │
│ (release notes, which scroll)                                  │
└────────────────────────────────────────────────────────────────┘
```

The panel lists your client's version, the running server's and the latest release, with when it was last checked. Below them are the notes for every release newer than yours, newest first.

- `[check]`, or `c`, asks again right away.
- `[update]`, or `u`, runs `ship update` in a tab named `update`, so you can watch it. When it finishes, the panel says `installed v0.2.0` and whether the server needs a restart. If it fails, the panel says `update failed` and points to that tab.
- The arrow keys, `PageUp` and `PageDown` scroll the notes, and `esc` or a click outside closes the panel.

A build that can't update itself, such as one from source, shows `[update]` dimmed and says to update with your package manager. To turn the automatic checks off, or change how updates run, see [Updates in the configuration](configuration.md#updates).

### Built from source

From your existing checkout:

```sh
git pull --ff-only
cargo install --path crates/ship
```

A build from source can't replace itself, so `ship update` just tells you to update it the way you installed it, which here means the two commands above. `ship update --check` and the updates panel still tell you whether there's a newer release.

If you're packaging Ship, set `SHIP_UPDATE_COMMAND` and `SHIP_LATEST_COMMAND` when you build it, and `ship update` runs your package manager's upgrade instead and asks it for the latest version. [Updates](configuration.md#updates) covers both.

## If something goes wrong

- **`zig` is missing or its version is wrong.** Run `zig version` in the same shell as Cargo and check that it reports 0.17. An older Zig elsewhere on your `PATH` can take precedence.
- **The linker or C compiler is missing.** Install your platform's build tools from the prerequisites above.
- **Linux reports missing ALSA headers or an `alsa` pkg-config error.** Install both `pkg-config` and the ALSA development package, not just the playback tools.
- **Installation succeeds, but `ship` isn't found.** Check that Cargo's install directory is on your `PATH`. If you've changed Cargo's install location, use that directory instead of `~/.cargo/bin`.

For problems after Ship starts, [Troubleshooting](troubleshooting.md) explains where to find the logs.
