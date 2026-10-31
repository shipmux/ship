# Building from source

<!--
Scope: Build prerequisites, installing Ship from a checkout, building without installing, updating, and common build or install problems.
Not here: first-session keys (README.md), server management (windows-and-the-server.md), packaging or cross-compilation.
-->

Ship has no releases yet, so installation is from source. On Linux or macOS, with the [prerequisites](#prerequisites) installed:

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

From your existing checkout:

```sh
git pull --ff-only
cargo install --path crates/ship
```

Reinstalling doesn't replace a running server. When you're ready to end its programs, [stop the server](windows-and-the-server.md) and run `ship` again to use the new version.

## If something goes wrong

- **`zig` is missing or its version is wrong.** Run `zig version` in the same shell as Cargo and check that it reports 0.17. An older Zig elsewhere on your `PATH` can take precedence.
- **The linker or C compiler is missing.** Install your platform's build tools from the prerequisites above.
- **Linux reports missing ALSA headers or an `alsa` pkg-config error.** Install both `pkg-config` and the ALSA development package, not just the playback tools.
- **Installation succeeds, but `ship` isn't found.** Check that Cargo's install directory is on your `PATH`. If you've changed Cargo's install location, use that directory instead of `~/.cargo/bin`.

For problems after Ship starts, [Troubleshooting](troubleshooting.md) explains where to find the logs.
