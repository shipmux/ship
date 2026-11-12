# Settle the wire API: product

Status: Locked on 2026-11-11 (Plannotator, "LGTM"), amended through A-2.

## Summary

The JSON shapes a release freezes get settled before 0.1.0, and a released client stops breaking on harmless additions from a newer server:

```diff
 # a newer server adds an event type
 $ ship watch
 {"type":"state","data":{…}}
-error: cannot decode attach event          # today: watch exits; the full client reattaches in a loop
+{"type":"bell","data":{"pane":"pane:…"}}   # printed as sent; the full client skips it
```

```diff
 $ ship pane get --pane "$P" | jq -c '{status, process}'
-{"status":{"state":"running"},"process":null}
+{"status":null,"process":{"state":"running"}}
```

```diff
-POST /api/v0/server/stop
+POST /api/server/stop
```

Nothing is set in stone: Ship tries to be polite in the common case and breaks, with a stated reason, when compatibility isn't worth its code. Settled in chat on 2026-11-11 ("nothing is set in stone, but we'll try to be polite and work in the common case").

## Goal

Once Ship is released, every protocol bump forces users to stop the server, which ends their running programs. Today the client refuses any server whose `protocol_version` differs (`crates/ship/src/main.rs:106`), and an unknown event type is a decode error (`crates/ship-client/src/api.rs:276-282`), so even a pure addition like a bell event needs a bump. This change makes the common, additive case need no bump, and settles the names and paths scripts will read before they are released. The budget is small: code that exists only for compatibility stays in the tens of lines ("as a general rule want MINIMAL backwards compat code ... it should be in the TENS of lines at MOST").

## Prior art

- **tmux** and **Zellij** refuse a client and server whose versions differ. That is Ship's behavior today, and it stays the behavior for real breaks.
- **Herdr** negotiates capabilities on connect (from #88's notes). Ship doesn't take this on: it is the kind of compatibility code this change keeps out.
- **Web APIs and Docker** put a version in the path (`/v1/`, `/v1.43/`) so one server can answer several client generations at once. Ship's server speaks one protocol and a real mismatch means restarting it, so a path version would only repeat `protocol_version`. Adding one later, if Ship ever serves two versions at once, is additive.
- **Plugin conventions** in `docs/site/scripting/writing-a-plugin.md` already tell scripts to ignore fields and line types they don't recognize. This change makes Ship's own client follow the same rule.

## User stories

### P1: A released client keeps working against a newer server with additions

Given a 0.1.0 client attached to a newer server whose protocol number is unchanged, when the server sends an event type, a field, an end reason or a reported state the client doesn't know, then the client keeps running: it skips the unknown event, ignores the unknown field, reconnects on the unknown end reason and shows the unknown state like a pane with no report.

Tested by feeding the client an attach stream with an unknown event type, an unknown `ended` reason and an unknown `program` state.

### P2: `ship watch` passes through what it doesn't know

Given a plugin following `ship watch`, when the server sends an event type `watch` doesn't know, then `watch` prints the line exactly as the server sent it and keeps going. It still exits on `ended`.

Tested by a stream containing an unknown type between two `state` lines; all three lines come out byte for byte.

### P3: A real break says what to do instead of looping

Given a client attached to a server, when an event fails to decode and the server's `/health` reports another protocol, then the client exits with the same restart guidance `ship` prints at startup. ~~When the protocol matches, it keeps reconnecting with growing backoff rather than at the shortest delay.~~ (A-2) When the protocol matches, it exits with the decode error: reattaching would get the same bytes.

Tested by swapping the server for one on another protocol while a client is attached.

### ~~P4: A mistyped request field fails loudly~~ (A-1)

Dropped in A-1: request bodies keep ignoring unknown fields, as today. See Non-goals.

```console
$ curl --unix-socket "$SOCK" -X PATCH http://localhost/api/panes/$P -d '{"nmae":"x"}'
400 {"type":{"kind":"validation"},…,"data":{"errors":[{"pointer":"/nmae","detail":"unknown field `nmae`, …"}]}}
```

~~Given any request body, when it has a field the server doesn't know, then the server answers 400 `validation` with the field's pointer and changes nothing. Today `PATCH /api/v0/panes/{id}` with `{"nmae":"x"}` answers 200 and does nothing (`crates/ship-core/src/protocol.rs:251-266`). A newer client sending a new field to an older server gets the same 400 rather than a silent no-op.~~ (A-1)

~~Tested per body type with one unknown field.~~ (A-1)

### P5: Scripts read one meaning of "status"

```console
$ ship pane get --pane "$P" | jq -c .process
{"state":"exited","code":3}
$ ship pane get --pane "$P" | jq -r '.program[""].state'
done
```

Given a pane, `.process` says whether its program is running or how it exited, and "status" only ever means what a program reported (the status record, `program`). `ship pane wait --status` and `send --if` keep their names and values.

Tested by reading a running pane, an exited pane and the CLI wait flags.

## Interface

**Pane JSON.** The process field is renamed; its shape is unchanged (`crates/ship-core/src/model.rs:62`, `168-182`):

```diff
-"status": {"state": "running"}
-"status": {"state": "exited", "code": 3, "signal": "SIGTERM"}
+"process": {"state": "running"}
+"process": {"state": "exited", "code": 3, "signal": "SIGTERM"}
```

**HTTP paths.** The `v0` segment goes; `/health` is unchanged:

```diff
-/api/v0/tabs, /api/v0/panes/{id}, /api/v0/attach, …, /api/v0/server/stop
+/api/tabs,    /api/panes/{id},    /api/attach,    …, /api/server/stop
```

**Protocol number.** This change bumps `PROTOCOL_VERSION` (23 to 24), since the rename and the paths break older peers.

**Plugin docs.** The promise in `docs/site/scripting/writing-a-plugin.md:88` changes:

```diff
-- Before 1.0, the JSON shapes may change between versions. `ship server status` reports the `protocol_version` a script can check.
+- New fields and line types can appear in any release; ignore what you don't recognize. A change that breaks existing shapes bumps `protocol_version`, which `ship server status` reports, and the release notes say why.
+- Object key order carries meaning: tabs, panes and status records are listed in order. Read them with `keys_unsorted` or `to_entries`, not `keys`.
```

**Glossary.** `GLOSSARY.md` gains **Process state**: whether a pane's program is running, or its exit code and signal once it exited. _Avoid_: Status (alone), as today.

## Non-goals

- Bells (#110), subscriptions, image support (#122) and floating panes (#86). Each is its own change and will be the first to use these rules.
- Settling the attachment, view, viewer and window naming. Deferred to `ship api` (#113).
- Capability negotiation, a minor version, or a feature list in `/health`.
- Serving more than one protocol from one server, or keeping old paths as aliases.
- Any guarantee for the saved-tabs file. A file that doesn't parse leaves the server starting empty, as today (`crates/ship-server/src/saver.rs:44-68`).
- A special freeze on `/health` or the stop route. They are as stable as the rest of the API: kept unless there's a good reason, and a break is called out.
- Rejecting unknown request fields (A-1). Bodies keep ignoring them, as `session-structure`'s Unknown request fields already says. Strict bodies break the dumb pipe: a script that reads a pane, edits it with `jq` and sends it back, or copies a pane's settings into `{"pane": …}`, would have to pick out the fields first. Ignoring unknown fields is also the common default (GitHub, Docker and most servers built on Go's `encoding/json` or serde), where Stripe and Kubernetes' opt-in strict field validation are the deliberate exceptions. The surfaces people type into, the CLI's flags and the config files, already reject unknown names, so typos left silent are those in hand-written JSON. The cost: a new field in an existing request body is silently dropped by an older server, so it needs a protocol bump when that would misbehave, per the AGENTS.md rule. A schema check in `ship api` (#113) can catch typos on the client later, without the server taking a side.
- Tolerating unknown values in enums whose meaning changes what the client draws (`Layout`, the process state, `Attr`, `CursorShape`, `BlockedKind`). A new value there is a bump.

## Functional requirements

- **FR-001** The full client and every attach-stream reader in `ship` (including `pane wait` and `send --wait`) SHALL skip attach-stream events whose `type` they don't know, and keep the stream open.
- **FR-002** `ship watch` SHALL print every attach-stream line exactly as the server sent it, including unknown types, and SHALL still exit as today on `ended`.
- **FR-003** A client SHALL treat an `ended` reason it doesn't know as a stream to reconnect, not as a shutdown.
- **FR-004** A client SHALL show a `program` record whose `state` it doesn't know like a pane without that record.
- **FR-005** When an attach event still fails to decode, the full client SHALL check `/health`. If the server's protocol differs, it SHALL exit with the existing mismatch message. ~~Otherwise it SHALL reconnect with growing backoff.~~ (A-2) Otherwise it SHALL exit with the decode error.
- ~~**FR-006** Every request body SHALL be rejected with 400 `validation` when it contains a field the server doesn't know, naming the field's JSON pointer, and SHALL change nothing.~~ (A-1) Dropped: request bodies keep ignoring unknown fields.
- **FR-007** Pane JSON SHALL carry the program's lifecycle as `process`, with the shape `status` has today, and SHALL NOT carry `status`.
- **FR-008** Every HTTP route under `/api/v0/` SHALL move to `/api/` with the same method, body and answer. `/health` SHALL stay where it is.
- **FR-009** `ship server status` and `ship server stop` SHALL keep accepting a server of any protocol, as today. The spec's requirement that `/health` and the stop route never change SHALL become "kept stable like the rest of the API".
- **FR-010** `PROTOCOL_VERSION` SHALL be bumped for this change.
- **FR-011** The plugin docs SHALL state the new promise and the key-order rule from Interface. The glossary SHALL define Process state.
- **FR-012** `examples/plugins/git.sh` SHALL pin the protocol this change ships (it says 22 today, `examples/plugins/git.sh:17`). The `ended` example in `openspec/specs/session-observation/spec.md:21` SHALL show the shape the server sends, `{"type":"ended","data":{"reason":"lost"}}`.

## Success criteria

- **SC-001** A client fed an attach stream with an unknown event type, an unknown `ended` reason and an unknown reported state keeps its screen, reconnects once on the `ended`, and draws the pane like one without a report.
- **SC-002** `ship watch` against that stream prints every line byte for byte.
- **SC-003** With a client attached, replacing the server by one on another protocol makes the client exit with the mismatch message within one reconnect, with no repeated attaches in the server log.
- ~~**SC-004** `PATCH /api/panes/{id}` with `{"nmae":"x"}` answers 400 with pointer `/nmae`, and the pane is unchanged. Every other request body type behaves the same with one unknown field.~~ (A-1) Dropped with FR-006.
- **SC-005** `ship pane get` prints `process` and no `status`. `ship pane wait --status exited` still ends when the program exits.
- **SC-006** No path in `crates/`, `docs/site/`, `openspec/specs/` or `examples/` still contains `/api/v0`.
- **SC-007** Code that exists only for compatibility (FR-001 to FR-005) adds at most tens of implementation lines.

## Open unknowns

None.

## Amendments

### A-2 (2026-11-11): exit when an event doesn't decode

A decode failure on a server whose protocol matches is a bug or an unbumped break, and a reattach gets the same bytes, so retrying only hides it while the screen sits stale. The full client now exits with the decode error; reconnecting stays for a cut stream or `ended`, which a fresh attach fixes. Approved in chat: "i thinnk that makes more sense. go for it!" Touches P3, FR-005.

### A-1 (2026-11-11): request bodies stay lenient

Dropped P4, FR-006 and SC-004 and added the reasoning under Non-goals. Strict bodies would force get, edit, send-back scripts to pick fields, and ignoring unknown fields is what most HTTP APIs do. Approved in chat: "i feel like keeping it dumb is what most http endpoints do for the most part too, right? … it's friendlier?", then "fair enough". Touches P4, FR-006, SC-004, Non-goals.
