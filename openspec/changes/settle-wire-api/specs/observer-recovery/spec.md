# Spec Delta

## ADDED Requirements

### Requirement: Server on another protocol
Before each reconnect attempt, a running client SHALL read the server's `/health`. If its `protocol_version` differs from the client's, the client SHALL exit with the protocol mismatch error `ship` gives at startup, without attaching. The delay between attempts SHALL reset only once a new attach has delivered an event after `attached`, so a server whose events don't decode is retried with increasing delays.

#### Scenario: Server replaced by another protocol
- **WHEN** a client is attached, the server stops, and a server speaking another protocol starts under the same name
- **THEN** the client's next reconnect attempt ends it with the mismatch error naming both protocols and the restart command, and the new server receives no attach

#### Scenario: Events that don't decode
- **WHEN** a server on the client's protocol sends an event of a known type the client can't decode after every attach
- **THEN** the client keeps reconnecting with increasing delays of at most five seconds
