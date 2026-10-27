# request-ordering Specification

## Purpose
Let one caller send several requests at once and have the server apply them in the order the caller numbered them, so a client never has to wait for one response before sending the next.

## Requirements

### Requirement: Order header
A request MAY carry `x-ship-order: {attachment}/{n}`, where `attachment` is an active attachment's ID and `n` is a whole number from 1. Each attachment's numbers SHALL form one sequence. A request without the header SHALL be handled exactly as it would be without this capability, whatever else is in flight. The header SHALL be independent of `x-ship-attachment-id`, and either can be sent without the other.

#### Scenario: Unnumbered request
- **WHEN** a script sends `ship tab create` while a full-screen client's numbered requests are waiting their turn
- **THEN** the create is handled at once, without waiting for them

#### Scenario: Malformed header
- **WHEN** a caller sends `x-ship-order: nonsense` or `x-ship-order: attachment:3f2a/0`
- **THEN** the response is 400 `validation` and the request is not handled

#### Scenario: Unknown attachment
- **WHEN** a caller numbers a request with an attachment that isn't active
- **THEN** the response is 404 `not_found` and the request is not handled

### Requirement: Numbered requests run in order
The server SHALL start an attachment's request `n` only after its request `n - 1` has finished, whatever order they arrived in. Request 1 SHALL start at once. A numbered request that has started SHALL finish even if its caller disconnects, and the next number SHALL then start.

#### Scenario: Arrived out of order
- **WHEN** a caller sends request 2, a key frame for a pane, and then request 1, removing that pane
- **THEN** request 2 runs only after the removal and its key is dropped as input for a removed pane

#### Scenario: Caller gone mid-request
- **WHEN** a caller sends request 1, creating a tab, and disconnects before the response
- **THEN** the tab is created, and the attachment's request 2 runs once it arrives

### Requirement: Repeated numbers
The server SHALL refuse a numbered request with 409 `out_of_order`, without handling it, when its number has already started. The message SHALL say the number already ran. A copy of a number that is still waiting SHALL get its refusal when that number's turn comes.

#### Scenario: Number sent twice
- **WHEN** a caller sends request 1 twice
- **THEN** the second gets 409 `out_of_order` saying the number already ran, and nothing is done twice

### Requirement: Missing numbers
The server SHALL refuse a waiting request with 409 `out_of_order`, without handling it, once it has waited 1 second with none of its attachment's requests running or finishing. A slow request that is running SHALL NOT count toward that second. The message SHALL say request `n - 1` never arrived, and the server SHALL log a warning naming the attachment and number. The server SHALL NOT skip a missing number, store responses for replay or run a refused number later.

#### Scenario: Gap in the sequence
- **WHEN** a caller sends requests 1 and 3 but never 2
- **THEN** request 3 gets 409 `out_of_order` about 1 second after request 1 finished, saying request 2 never arrived, and the server logs a warning

#### Scenario: Slow predecessor
- **WHEN** an attachment's request 1 takes 3 seconds and request 2 arrives while it runs
- **THEN** request 2 runs after request 1 finishes and is not refused

#### Scenario: Slow but moving
- **WHEN** an attachment's requests 1 to 20 are in flight at once and each takes a few milliseconds
- **THEN** all 20 run in order and none is refused, though the last waits for all the others

### Requirement: Ordering state follows attachments
The server SHALL keep ordering state only for attachments that have sent a numbered request, and SHALL drop an attachment's state once the attachment has ended. A new attachment's sequence SHALL start at 1, including after the server restarts.

#### Scenario: Reattach
- **WHEN** a client's attachment ends and it attaches again
- **THEN** the new attachment's request 1 runs at once, whatever the old attachment had reached
