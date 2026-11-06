# Stop any protocol: program

Status: Locked on 2026-11-06, approved in chat ("sounds good. go for it").

## Summary

One slice, in `crates/ship/src/main.rs` and `crates/ship/src/local.rs`:

```diff
-async fn health(client: &Client) -> Result<HealthResponse> {
+async fn health(client: &Client, accept: Accept) -> Result<HealthResponse> {
     ...
-    if response.protocol_version != PROTOCOL_VERSION {
+    if matches!(accept, Accept::Compatible) && response.protocol_version != PROTOCOL_VERSION {
```

## Rationale

The protocol comparison is one line in one function, so the choice is a parameter on that function rather than a second check. `Accept` is an enum, not a `bool`, so each call site says which it means.

## Skeleton map

### Changed

#### `crates/ship/src/main.rs`

- `enum Accept { Compatible, AnyProtocol }`, documented as above.
- `health`, `checked` and `checked_target` take `accept`.
- `server stop` and `server status` pass `AnyProtocol`; `server log`, `watch`, the full-screen client with a URL and `connect` pass `Compatible`.
- `update_server` calls `health(…, Accept::AnyProtocol)` in place of its own service check.

#### `crates/ship/src/local.rs`

`ensure_running` passes `Compatible` in both of its health checks.

### Untouched

The server, the protocol, `updates::mismatch` and its restart guidance.

## Build order

### 1. Stop and status accept any protocol

Built as above. Verified with a protocol-22 server on its own name and a protocol-23 client, and with a fake socket whose health names another service.

## Deviation log
- The first delta extended "Health contract and result" and "Current-client health preflight", which pushed both past openspec's 500-character limit. The rule moved into its own requirement, "Restart across protocols", with the same meaning.
