# Design

The design for this change lives in three papers under `design/`. They are the source of truth, and this file only points at them.

- [`design/product.md`](design/product.md): a background update check (TUI client only, at startup and hourly, shared across clients), the version in the sidebar's bottom row, a one-time mode-bar note, an updates panel with release notes from `≡ → updates`, `ship update` and `ship update --check`, clearer client/server sync messages, and an install script. Locked, with an approved amendment settling OU-1, OU-2, OU-4 and the notes source.
- [`design/architecture.md`](design/architecture.md): shape A. The client checks in-process through a cache file that doubles as the one-hour stamp; `ship update` in the binary uses `self_update` behind a `self-update` feature; the server and protocol don't change. Locked, with an approved amendment moving the version probe into `ship-client` and running it after the swap.
- [`design/program.md`](design/program.md): the file map and six slices: version and mismatch messages, `ship update --check` with config and cache, `ship update`, the notice, the panel, then install, releases and docs. Locked.
