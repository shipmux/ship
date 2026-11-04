#!/bin/sh
# Installs the latest Ship release to ~/.local/bin:
#   curl -fsSL https://raw.githubusercontent.com/shipmux/ship/main/install.sh | sh

set -eu

releases=https://github.com/shipmux/ship/releases/latest/download
from_source=https://github.com/shipmux/ship/blob/main/docs/site/building-from-source.md
bin_dir="$HOME/.local/bin"

fail() {
    printf 'ship install: %s\n' "$1" >&2
    exit 1
}

# One function, called on the last line, so a download cut short runs nothing.
main() {
    platform="$(uname -s) $(uname -m)"
    case "$platform" in
        "Linux x86_64") target=x86_64-unknown-linux-gnu ;;
        "Linux aarch64" | "Linux arm64") target=aarch64-unknown-linux-gnu ;;
        "Darwin arm64") target=aarch64-apple-darwin ;;
        "Darwin x86_64") target=x86_64-apple-darwin ;;
        *) target= ;;
    esac
    # The Linux archives link glibc.
    if [ "$(uname -s)" = Linux ] && ldd --version 2>&1 | grep -qi musl; then
        platform="$platform (musl)"
        target=
    fi
    [ -n "$target" ] || fail "no prebuilt ship for $platform; build it from source: $from_source"

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    archive="ship-$target.tar.gz"
    for asset in "$archive" "ship-$target.sha256"; do
        curl -fsSL -o "$tmp/$asset" "$releases/$asset" ||
            fail "cannot download $releases/$asset"
    done

    if command -v sha256sum >/dev/null; then
        actual=$(sha256sum "$tmp/$archive")
    else
        actual=$(shasum -a 256 "$tmp/$archive")
    fi
    expected=$(cut -d ' ' -f 1 "$tmp/ship-$target.sha256")
    [ "${actual%% *}" = "$expected" ] ||
        fail "$archive doesn't match its published checksum; installed nothing"

    tar -xzf "$tmp/$archive" -C "$tmp" ship
    mkdir -p "$bin_dir"
    # Renamed within the directory, so a running ship keeps its file.
    cp "$tmp/ship" "$bin_dir/.ship.new"
    if ! version=$("$bin_dir/.ship.new" --version); then
        rm -f "$bin_dir/.ship.new"
        fail "the downloaded ship doesn't run here; installed nothing"
    fi
    mv -f "$bin_dir/.ship.new" "$bin_dir/ship"
    printf 'installed %s\n' "$version"

    case ":$PATH:" in
        *":$bin_dir:"*) ;;
        *) printf "%s is not on PATH; add this line to your shell profile:\n  export PATH=\"%s:\$PATH\"\n" "$bin_dir" "$bin_dir" ;;
    esac
}

main
