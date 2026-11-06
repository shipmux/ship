# Proposal

Status: Locked on 2026-11-06. Assembled from `design/product.md`, `design/architecture.md` and `design/program.md`. The papers are the contract, and this proposal and the deltas under `specs/` are views of them.

## Why

The protocol-mismatch error tells users to run `ship server stop`, and `ship server stop` refused with the same error, so the only way to restart after an update was to kill the process.

## What Changes

- `ship server stop` and `ship server status` work against any Ship server, whatever its protocol.
- They still refuse malformed health and other services.
- `GET /health` and `POST /api/v0/server/stop` keep their shape in every protocol.

## Capabilities

### Modified Capabilities

- `health-exchange`: a new "Restart across protocols" requirement, and the preflight defers to it.

## Impact

- `crates/ship`: `Accept` on the health check. No protocol bump, and the server doesn't change.
