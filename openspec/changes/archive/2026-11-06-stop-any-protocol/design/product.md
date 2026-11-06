# Stop any protocol: product

Status: Locked on 2026-11-06, approved in chat ("sounds good. go for it").

## Summary

After an update that changes the protocol, the restart the error asks for works:

```console
$ ship
{"source":"internal",...,"message":"incompatible protocol: client ship 0.1.0 (protocol 23); server ship 0.1.0 (protocol 22)\nrestart the server: ship server stop && ship (ends running programs; tabs are restored)",...}
$ ship server stop          # before: the same error; now: stops it
$ ship
```

Before this change, `ship server stop` checked the protocol like every other command, so the error's own advice failed and the only way out was `kill`.

## Goal

A user can always inspect and stop the Ship server they have, whatever version it is. Every other command keeps refusing a server on another protocol. Settled in chat on 2026-11-06 ("i always noticed that the protocol bump prevents me from killing the server ... extremely annoying").

## User stories

### P1: Restart after an update

A user updates Ship while the old server runs. `ship` refuses with the restart guidance, and following it works.

### P2: Check what's running

`ship server status` prints the health of whatever Ship server answers, including its `protocol_version`, so a user or script can see the mismatch without the command failing.

## Non-goals

- Making other commands work across protocols. Their requests and answers change shape between protocols.
- Restarting the server automatically, or from the client's mismatch screen.
- Servers from before health carried `log` and `socket` (2026-10-27).

## Functional requirements

- **FR-001** `ship server stop` stops any Ship server, whatever its protocol.
- **FR-002** `ship server status` prints the health of any Ship server, whatever its protocol, and exits 0.
- **FR-003** Both still refuse a server whose health is malformed or names another service, and neither sends anything after that refusal.
- **FR-004** Every other command keeps refusing a server on another protocol with the existing restart guidance.

## Success criteria

- **SC-001** A client at protocol 23 runs `server status` and `server stop` against a server at protocol 22: status prints `"protocol_version":22` and exits 0, stop exits 0, and the server is gone.
- **SC-002** Against the same server, `tab list` and `server log` fail with the mismatch error.
- **SC-003** Against a socket whose health says `"service":"other"`, `server stop` fails and sends no stop request.

## Amendments
