# Stop any protocol: architecture

Status: Locked on 2026-11-06, approved in chat ("sounds good. go for it").

## Summary

The client's health check takes what it accepts:

```rust
enum Accept {
    /// Ship servers speaking this client's protocol.
    Compatible,
    /// Any Ship server. `server stop` and `server status` must reach one on
    /// another protocol, since that's the server a restart replaces.
    AnyProtocol,
}

async fn health(client: &Client, accept: Accept) -> Result<HealthResponse>;
```

`server stop` and `server status` pass `AnyProtocol`; everything else passes `Compatible`.

## Context and constraints

- `health` in `crates/ship/src/main.rs` is the one compatibility check. It refuses another service, then another protocol. `checked` and `local::ensure_running` call it.
- `update_server`, in the same file, already accepts any protocol for `ship update`'s message, with its own copy of the service check.
- The current spec's "Stop preflight" scenario requires stop to refuse another protocol. It came from layout-sidebar's correction, which routed every explicit command through the same preflight. Its evidence gives no stop-specific reason.
- Accepting another protocol is only safe for requests whose shape doesn't change between protocols.

## Candidate shapes

### A. `Accept` on the shared check (winner)

One check, with the protocol comparison skipped for `AnyProtocol`. `update_server` uses it too, which removes its copy of the service check.

### B. Best effort with a warning for every server command (rejected)

`server log` sends and parses a filter whose shape can change. Against another protocol it could fail on a parse error or do something different, and a warning on `stop` tells the user what they already know.

### C. Stop by signal from the PID (rejected)

Platform-specific, needs the PID from a server that may be unreachable, and skips the server's own shutdown path for a remote socket.

## Decision

Shape A.

## Key decisions

- **KD-1** `GET /health` and `POST /api/v0/server/stop` keep their shape in every protocol: health keeps `service`, `protocol_version`, `version`, `log` and `socket`, and stop takes no body and answers 202. Fields may be added; none are removed or retyped.
- **KD-2** No protocol bump. Nothing on the wire changes.
- **KD-3** `server status` prints another protocol's health without a warning, since `protocol_version` is in what it prints.

## Risks and unknowns

- A future protocol that has to change health or stop must keep the old shape alongside, or this breaks for the release after it. KD-1 makes that a spec rule.

## Amendments
