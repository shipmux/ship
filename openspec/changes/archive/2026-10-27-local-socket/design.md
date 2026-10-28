# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked all three on 2026-10-27. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P3, FR-001 to FR-012, success criteria SC-001 to SC-006, U-1 to U-4 resolved.
- [`design/architecture.md`](design/architecture.md): shape A, a tokio Unix socket guarded by a lock file, interprocess named pipes on Windows only, and key decisions K-1 to K-7.
- [`design/program.md`](design/program.md): the touched files and signatures, slices 0 to 4, O-1 resolved (allow 0BSD), and the deviation log.

## Context

The server listens on TCP `127.0.0.1:43179` with no login, so any program on the machine, including other users' programs, can drive it, and two users collide on the port. See the product paper's Summary.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (only I can reach my server, Ship works as before, a crash doesn't leave me stuck), P2 (several servers side by side, a server on another machine) and P3 (Windows builds).

**Non-goals:** see the product paper's Non-goals.

## Decisions

1. **Unix socket only, no TCP listener** (product FR-001). HTTP stays the protocol, and the client uses `http://localhost/` as its base.
2. **Names pick the socket, the saved tabs and the logs** (FR-003). Clients choose by `--server-url`, `--server-name`, `SHIP_SERVER_URL`, `SHIP_SERVER_NAME`, then `default` (FR-007).
3. **A lock file decides who owns a name on Unix** (architecture K-1, K-2). It's held for the server's life and released by the kernel on any death. The pipe's first-instance flag does this on Windows.
4. **One `Endpoint` in ship-core** (K-3). The client checks the socket directory too.
5. **reqwest stays the client** (K-4). interprocess is a Windows-only dependency (K-5).
6. **`local_only` stays** (K-6). Windows CI is allowed to fail (K-7).
7. **Protocol 19**, since health gains `socket`.

## Risks / Trade-offs

- [Unix sockets regress latency] → Slice 0 measures TCP first, and slice 2 compares (SC-003).
- [Windows is untested by hand] → CI shows build and test results. Behavior is reported unverified.
- [macOS temp cleanup removes an idle server's socket] → Unverified. Fallbacks are in architecture R-3.
- [A long name overflows the socket path limit] → Names are capped at 32 bytes.

## Migration Plan

Rename `tabs/43179.json` to `tabs/default.json` by hand (product non-goal: no automatic move). Client and server come from the same build, and protocol 19 refuses a mismatch with a clear error.
