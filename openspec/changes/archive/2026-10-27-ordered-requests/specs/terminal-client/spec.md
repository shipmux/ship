# Spec Delta

## MODIFIED Requirements

### Requirement: Ordered input
A client's bound actions, menu items, typed keys, pastes, `client.send`, clicks, fold clicks, wheel turns, drags and mouse input to programs SHALL take effect on the server in the order they happened. Anything that depends on the selection, the layout or the folds SHALL be resolved against the state after everything before it has taken effect. This SHALL hold for `server.` actions too: an action SHALL take effect only after keys typed before it.

#### Scenario: Type right after a new tab
- **WHEN** 500 ms of delay is added each way between client and server, and the user presses `alt-n` then types `echo hi` and Enter at once
- **THEN** `hi` is printed in the new tab's shell, and nothing reaches the pane that was selected before, in ten runs of ten

#### Scenario: Zoom what was just split
- **WHEN** 500 ms of delay is added each way and the user presses `alt-|` then `alt-f` at once
- **THEN** the new pane is the one zoomed, in ten runs of ten

#### Scenario: Type then close
- **WHEN** 500 ms of delay is added each way, and the user types `ls` into a shell pane and presses `alt-shift-x` at once
- **THEN** the server routes both keys to that pane before removing it, and its debug log shows no input dropped for a removed pane, in ten runs of ten

#### Scenario: Idle typing
- **WHEN** nothing is waiting and the user types into a shell on a local server
- **THEN** each key is sent at once and feels no slower than before

### Requirement: Input across a disconnect or detach
When the connection drops, queued input SHALL be dropped and an in-flight action SHALL NOT be retried after reconnecting. Already-sent input or requests are not undone. When the server refuses one of the client's requests as `out_of_order`, the client SHALL treat it as a dropped connection and reattach. Detaching SHALL wait for the client's queued and in-flight input requests to finish before ending the attachment, so earlier typing is not abandoned; a signal may interrupt this drain.

#### Scenario: Proxy cut mid-burst
- **WHEN** 500 ms of delay is added each way, the user presses `alt-n` and types, and the connection is cut before the tab appears
- **THEN** after reconnecting, none of the waiting keys reach any pane and the create is not sent again

#### Scenario: Detach after typing
- **WHEN** on a slow link the user types `touch done` and Enter, then presses `alt-q` at once
- **THEN** the file `done` exists after the client has detached

#### Scenario: Refused number
- **WHEN** the server refuses one of the client's requests as `out_of_order`
- **THEN** the client logs a warning, reattaches with its last view, and input typed after the reattach reaches the selected pane

## ADDED Requirements

### Requirement: Input bursts
The client SHALL keep at most 4 input requests in flight. Input that arrives while 4 are in flight SHALL wait and go out together in one request when one finishes, up to about 1 MiB per request. A single frame larger than that SHALL go alone. When the server refuses an input request for any reason other than `out_of_order`, the client SHALL show the error on the status line and keep sending later input.

#### Scenario: Flood of keys
- **WHEN** a script writes 2000 keys into the client's terminal at once, without bracketed paste, while a shell pane is selected
- **THEN** all 2000 reach the shell in order, the client never has more than 4 input requests in flight, and it sends fewer than 2000 requests

#### Scenario: Oversized paste
- **WHEN** a user pastes 3 MB of text into a shell pane
- **THEN** the status line shows the server's refusal, nothing from that paste reaches the pane, and typing afterwards still reaches it
