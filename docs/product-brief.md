# Ship: product brief and roadmap

Ship is a compact Rust terminal workspace for coding-agent work. It keeps shells, editors and agents together, lets work continue after the UI closes, shows which agents need attention, and gives agents a small automation interface.

The aim is to recover most of the daily-use value of Herdr with substantially less owned code and maintenance burden. Ship is a side project, not a feature-for-feature rewrite or an upstream refactor. The name treats agents as crewmates and plays on shipping software. The intended executable is `ship`; package and distribution identifiers remain to be checked.

This brief records product direction and a phased roadmap. It is not an implementation specification. Milestones describe proposed work, not completed capabilities, and have no calendar commitments. Scope remains revisable as working slices provide evidence.

Build through fine-grained, coherent commits. A commit need not be a complete vertical slice; a vertical slice demonstrates a narrow behavior end to end, and a milestone is a checkpoint for judging working behavior and choosing what comes next.

## Direction

### Own the domain, rent the mechanisms

Use existing libraries for solved capabilities. Ship should own session/tab/pane behavior, session rules and the composition of those capabilities, rather than bespoke implementations of serialization, terminal emulation, playback, clipboard access or transport.

Use idiomatic Rust types and abstractions to express invariants once. Use derives and macros when they remove independently maintained copies of the same information. Do not pursue fewer lines through opaque compression or a generic framework built for hypothetical features.

The starting stack is Rust with Clap, Tokio, tracing/tracing-subscriber, and shared error handling adapted from an earlier project's error module. Trial Ratatui/Crossterm, `libghostty-vt`, and the supplied `ratatui-ghostty` session before replacing their mechanics with owned code. Kameo is the actor direction, with the historical RelayBus/Sink typed-subscription pattern as a reference; initial actor topology remains a recommendation, not an approved design. `portable-pty` passed the scratch probe and remains the candidate for PTY integration; actual Ship integration and Linux validation remain. Rodio and Copypasta remain later candidates, not selections.

Ratatui draws the workspace; Ghostty interprets programs' terminal output and maintains screen state on the server. The intended reuse path converts the supplied session's rendered Ratatui cells into owned, serializable screen/cell types, then converts those types back for client rendering. The macOS scratch probe verified captured screen conversion and JSON patch application, and Cyan confirmed the patched wrapper's multiplexer example worked. The earlier structdiff and JSON Patch directions are superseded by compressed full snapshots for both structural state and terminal screens; optimize only after profiling the runnable workflow. See [transport evidence](research/terminal-transport-handoff.md). This is feasibility evidence, not complete platform/rendering coverage. See [terminal evidence](research/terminal-feasibility.md).

### Size is a design constraint, not the acceptance test

Aim for roughly 20,000 lines of authored production Rust while retaining most of the functionality Cyan actually uses. That budget is a hypothesis to test, not a proven estimate or a reason to omit necessary behavior.

Report production code, tests, generated code and dependency code separately. Also report the full authored total including tests, so reductions are not hidden by moving code between categories. Working behavior, understandable boundaries and maintenance burden matter more than hitting an exact number.

Start with zero authored test code and no speculative test scaffolding. Add tests only after encountering flaky behavior and agreeing that a specific test is warranted. Verify increments with builds, formatting, Clippy, and real workflows; temporary experiments may establish dependency behavior without adding permanent tests. Omitting tests reduces the authored total, not the separately reported production-code count.

### Platform policy

Linux is Cyan's primary environment. Linux and macOS are priority platforms; Windows and other platforms are best effort.

Choose cross-platform dependencies by default. Validate priority platforms deliberately and record other platform gaps without making them initial-release blockers. Do not write speculative platform-specific subsystems in advance.

### Local first, remote-compatible boundary

The initial product is local. Remote attachment is not required for the first daily-driver release.

Keep a clear client/server boundary that can later cross a network: explicit commands, responses and events; shared state distinct from client presentation state; live process and PTY resources private to the server. The client should not depend on direct access to server internals.

The initial architecture uses separate client and server processes from one executable, with HTTP request/response and SSE for server events. Bind to loopback by default; port customization is deferred. The server owns authoritative shared state. Clients apply compressed textual SSE full snapshots while keeping presentation choices local. Session revisions reject stale/out-of-order snapshots, but do not prevent attachment gaps: snapshot/subscription coordination is still needed. Exact revision scope and restart baseline identity/reset remain open. Terminal screens are separate from structural `SessionState`; no patch/replay pipeline is adopted.

SSH port forwarding is the intended later remote connection option, not part of the application protocol. Direct remote HTTP access would require a separate authentication/encryption/access-control design. Do not build remote infrastructure for the first slice.

Local authentication is explicitly deferred. Loopback does not restrict access to the current user: other local processes or users may reach the control endpoint. Do not describe this initial endpoint as authenticated or enable non-loopback exposure without revisiting security.

Use a Cargo workspace with `ship` (binary, CLI and startup), `ship-client`, `ship-server`, and `ship-core` (genuinely shared types, traits and helpers). Client and server depend on core rather than each other's internals. A metadata roundtrip before PTYs is the recommended next checkpoint, not yet approved scope. One interactive terminal across the real client/server path remains a later checkpoint.

The server owns sessions, not workspace entities. Each recursive tab owns a pane layout and ordered child tabs; empty tabs are valid and can be selected independently of panes. Client selection names the most specific session/tab/pane and derives ancestry. Typed prefixed IDs identify targets without requiring whole paths. Details, removal/move semantics and unresolved interfaces live in the [session structure spec](../openspec/specs/session-structure/spec.md); domain terms are in the [glossary](../GLOSSARY.md).

## Initial daily-driver scope

The first daily-driver release comprises milestones 1 through 4: a local terminal workspace, persistent sessions, agent awareness and integrations, and automation. Each milestone should leave a usable increment. The order expresses capability goals, not a requirement to postpone foundational API boundaries until milestone 4.

### 1. A terminal workspace to work in

Reach this capability milestone through smaller runnable checkpoints, with a proposed metadata roundtrip before one real terminal across the client/server path. The metadata scope is not approved yet. The list below is not a single implementation batch.

Include:

- Sessions, recursive tabs and split panes.
- Focus, resize, zoom, move, rename and close operations.
- Configurable keyboard bindings.
- Mouse focus, text selection and pane resizing. Pane-border resizing is implemented by the [mouse change](../openspec/changes/archive/2026-10-14-mouse/tasks.md), alongside sidebar resizing, tab drops and context menus. macOS PTY verification and Cyan's live border-drag check are recorded there; two-terminal hand coverage and Linux remain unverified.
- Scrollback, copy/paste and terminal text search.
- Terminal emulation and PTY handling through suitable libraries.

Completion evidence:

- Pi, Neovim, ordinary shells and running tests work side by side.
- Repeated splitting and resizing preserve usable rendering and input.
- Alternate-screen applications enter and exit correctly.
- Selection, copying, pasting and search work in representative sessions. Until built-in search lands, `ship pane read --lines N` piped into `grep` or an editor stands in for the search evidence, as approved in the scrollback change's 2026-10-11 amendment. Edit-scrollback in milestone 5 replaces built-in search and copy mode.
- These behaviors are exercised on Linux and macOS, with known limitations recorded.

### 2. Leave and come back

Include:

- A background server that owns terminals and processes independently of the UI.
- Detach and reattach without interrupting running work.
- Save and restore tab and pane identities, layout and working directories.
- Log panes whose saved directory was unavailable, and start them in home.
- Recover the previous saved tabs by hand when needed.

Agent conversation resume moves to a later change. Restored panes start fresh shells; they do not resume agents or rerun their earlier programs.

Keep the promises distinct:

| Situation | Expected behavior |
| --- | --- |
| The UI closes while the server remains alive | Existing processes continue; the client can reconnect. |
| The server or machine restarts | Restore saved tabs and start fresh shells in their saved directories. Original processes do not survive. |
| An agent supports conversation resume | Deferred to a later change; Ship does not resume the conversation. |

Persist reconstructible tab and pane descriptions, not process handles, PTYs or connections. Safe saves and a policy for saved-format changes belong here. Exact screen-history replay is a follow-on capability, not a prerequisite for basic persistence.

Completion evidence:

- Closing and reopening the client preserves running work.
- Restarting the server reconstructs the saved tabs with fresh shells and logs panes whose directory was unavailable.
- An interrupted save leaves either the previous saved tabs or the new ones.
- A missing directory falls back to home with a warning in the server log; an unreadable file starts empty without being deleted.

### 3. Know which agent needs attention

Include:

- Working, blocked, idle and unknown agent states.
- An unseen-completion indicator distinct from an agent being actively busy.
- An agent list and jump-to-agent action.
- Desktop and audio notifications with simple configuration.
- OSC 7501, the [Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status), as the reporting interface. Claude Code, Codex, Pi and crush already emit it, so Ship keeps no per-agent knowledge and an agent that adopts it needs no Ship change.

Hooks and screen-based detection move to later, needed only for an agent Cyan uses that never adopts the protocol. Even then, an integration should be a small adapter that reports through the same records, not a plugin framework or a broad catalog of screen heuristics. The program-status change amended this milestone.

Completion evidence:

- Several concurrent agents can be distinguished by state and location.
- An agent requesting input or finishing unseen work is discoverable without visiting every pane.
- Unknown state is shown honestly rather than treated as completion.
- Notifications respect configuration, and unavailable optional notification/audio facilities do not break terminal work.
- Agents' own reports are shown against real sessions.

### 4. Let agents operate the workspace

Include a CLI and machine-readable control interface for:

- Listing and inspecting tabs, panes and their program status.
- Creating, splitting, focusing and closing panes.
- Running commands and sending text or logical keys.
- Reading visible output and recent scrollback.
- Sending a prompt to a pane and waiting for it to be handled.
- Waiting for a program-status transition or exit.
- Receiving state updates, through `ship pane wait` and the existing listing commands.

Use explicit target identities and useful errors. Automation must not accidentally act on whichever pane the user happens to focus. Share operation definitions with the application rather than reimplementing behavior in CLI handlers. The pane-automation change amended this milestone.

Completion evidence:

- An agent can start a sibling task, submit work, wait for a result and read it through the control interface.
- The sequence uses explicit identities rather than guessed targets; `ship pane create` prints the new pane, which settles the creation-response contract.
- User focus changes do not redirect targeted automation.
- Invalid targets, closed panes, timeouts and unavailable output are reported clearly.

## Later roadmap

Milestones 5 to 7 came out of a 2026-10-26 review of open issues, Herdr and Zellij. Each item is an issue; its Details hold the research and the open decisions. Milestone 5 is the minimum for the first release ([#42](https://github.com/shipmux/ship/issues/42)). Milestone 6 only adds behavior, so it can join that release if it is ready. Estimates are implementation Rust lines, against about 11,700 today.

### 5. Ready to share

Finish what a first-time user would notice missing. About 320 to 680 lines.

- Keyboard tab work in tabs mode: tree-aware `h`/`l` and reordering with `client.tab.move` ([#83](https://github.com/shipmux/ship/issues/83)).
- Rename a tab or pane from the UI ([#84](https://github.com/shipmux/ship/issues/84)).
- Edit a pane's history in the user's editor, in place of built-in search and copy mode ([#85](https://github.com/shipmux/ship/issues/85)).
- Fix keys reaching the server after a later binding ([#20](https://github.com/shipmux/ship/issues/20)). Done in [#90](https://github.com/shipmux/ship/pull/90).

Completion evidence: tabs can be navigated, reordered, nested and renamed without a mouse, and a pane's history can be searched and copied from in an editor.

### 6. Scripts as extensions

Let scripts outside Ship do what plugins would, through `ship` commands, panes and labels. About 900 to 1,700 lines.

- `server.run` from a key binding ([#22](https://github.com/shipmux/ship/issues/22)).
- Floating panes, one layer per tab, for prompts, pickers and quick shells ([#86](https://github.com/shipmux/ship/issues/86)).
- Metadata tokens: short labels scripts put on panes and tabs, display-only ([#87](https://github.com/shipmux/ship/issues/87)). Git branches and PR status come from scripts reporting tokens, not from built-in Git support.
- Plugin packaging later, as a thin layer over these once their surface has settled. A GitHub topic, as Herdr uses, is enough of a marketplace.

Completion evidence: a script can show a branch or PR status in the sidebar, and a key binding can open a picker in a floating pane, with no Ship change.

### 7. Remote

Show several servers in one client. This merges the earlier remote attachment and multiple-machines milestones. Most of the work is client-side: connection state held once becomes one entry per server. About 800 to 1,660 lines including restart and update.

- Saved servers reached over SSH, shown together in the sidebar with per-server status ([#88](https://github.com/shipmux/ship/issues/88)). SSH settings come from `~/.ssh/config`; no remote install or named remote sessions.
- Restart the server from a client ([#89](https://github.com/shipmux/ship/issues/89)), and self-update ([#36](https://github.com/shipmux/ship/issues/36)), so a remote server can be updated without a shell on it.

Completion evidence: a dropped connection does not stop remote work or make local work unusable, and reconnection restores a coherent view without silently duplicating commands. Exact delivery/retry guarantees remain a design question.

## Deferred supporting features

| Capability | Initial position |
| --- | --- |
| Configuration system | Defer past the first terminal slice; fixed defaults and minimal operational bindings first. Storage, key matching, reload and shared/per-client scope remain open for a customization slice. |
| Plugin framework and marketplace | Packaging follows milestone 6, once `server.run`, floating panes and metadata tokens define what a plugin can use. No plugin runtime or embedded language. |
| Built-in worktree management | Use Git or Worktrunk in ordinary panes. |
| Self-update and release channels | Milestone 7 ([#36](https://github.com/shipmux/ship/issues/36)); normal installation/distribution tooling until then. |
| Browser client | After remote. Try `ttyd ship`, the native client in a web terminal, first, as Zellij's web client does with xterm.js. A wasm client only if that feels slow: crossterm does not build for wasm, so it needs Ship-owned key types, a Ratatui web backend such as Ratzilla, and an authenticated `--host` mode. |
| Agent conversation resume | Last, if ever. OS-specific shutdown behavior and per-agent session hooks that would need ongoing upkeep. |
| Agent hooks and screen-based detection | Only for an agent Cyan uses that never adopts OSC 7501, reporting into the same status records. |
| Advanced terminal graphics and inline images | Separate compatibility work; not an initial requirement. |
| Exact saved-screen replay | Follow basic persistence if useful. |
| Live server upgrades and process handoff | Defer; distinguish ordinary restart/restore from process survival. |
| Elaborate themes and notification rules | Start with a small configuration surface. |
| Full Windows/platform parity | Best effort outside Linux and macOS. |

Not planned: built-in Git status and worktrees (scripts report them as metadata tokens instead), screen-based agent detection beyond the row above, swap layouts, and themes beyond the small configuration surface.

## Decisions still open

Resolve these during slice design and dependency experiments, not by silently treating this brief as a specification. Current choices are recorded in the [openspec specs](../openspec/specs/) and the [transport handoff](research/terminal-transport-handoff.md).

- Delivery of the demonstrated ratatui-ghostty compatibility patch (pinned fork versus source/subtree import); no method has been selected or installed in Ship. Supplied session plus portable-pty passed macOS feasibility checks; actual application integration, coherent publication and Linux coverage remain.
- Exact HTTP operations, SSE subscription/snapshot lifecycle, message shapes, and restart identity behavior. Axum/Tower/Reqwest passed temporary synthetic loopback transport probes, not Ship integration. Automatic Reqwest zstd decoding remains unverified; zstd leads the codec candidates and gzip fallback is a recommendation.
- Final screen/cell schema and detailed rendering coverage. Full snapshots replace the earlier whole-vector structdiff pipeline direction; performance optimization remains evidence-driven.
- Proposed metadata checkpoint scope, actor ownership topology, creation responses and attachment lifetime.
- Detailed shared-state fields and per-client view state. Focus is client-local but reportable as presence; concurrent terminal input is allowed. Tab geometry follows the minimum available width and height among clients currently viewing that tab; behavior with no viewers remains to be specified.
- Saved-state format evolution, recovery policy and agent resume descriptions.
- Minimal fixed interaction/exit bindings for the first slice. Configuration format, storage, configurable key matching/reload and plugin ownership are deferred to later slices; see [ownership research](research/config-plugin-ownership.md).
- Remote authentication, command delivery/retry behavior and transport, before remote implementation. Milestone 7 is SSH-only. Direct HTTP would need a `--host` mode that drops the loopback and `Origin` checks and requires a token header (not a cookie) on every request, over an encrypted network such as Tailscale.
- Milestone 6 semantics: whether programs can set metadata tokens with an escape sequence, and whether a tab's floating layer is shown per client or for everyone ([#86](https://github.com/shipmux/ship/issues/86), [#87](https://github.com/shipmux/ship/issues/87)).
- Repository location, project license and package/distribution identifiers.

## Reference and scope of evidence

Herdr is the behavioral reference, not the structural template. The initial inventory examined checkout `d6b40d4edd550ccea081f089605a64314f8c8b27` (version 0.9.3). Useful references in this notebook:

- `repos/herdr/repo/README.md`: product behavior and persistence distinctions.
- `repos/herdr/repo/docs/next/website/src/content/docs/cli-reference.mdx`: user-facing command families.
- `repos/herdr/repo/src/cli/spec.rs`: declared control surface.
- `repos/herdr/repo/src/sound.rs`: an inspected example of implementation responsibility Ship intends to rent from a library.

The inventory used docs, declarations and limited code checks, not a complete behavioral audit. No implementation-size estimate has been demonstrated by a prototype. The roadmap reflects Cyan's starting priorities. The first-slice ownership and transport direction is recorded, and macOS dependency feasibility has been demonstrated. The initial client/server scaffolding and health exchange are implemented. Session replication, terminal integration, Linux terminal validation and full behavior coverage remain future work.
