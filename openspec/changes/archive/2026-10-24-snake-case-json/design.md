# Design

## Context

Wire names come from serde attributes on the `ship-core` types. 53 types use `rename_all = "camelCase"` and 24 use `snake_case`. Since most keys are single words, only six keys and four enum values change on the wire (proposal.md, What Changes). The same types serialize the HTTP API, attach streams, CLI stdout, error lines and the saved tabs file. utoipa derives the OpenAPI schema from them. `PROTOCOL_VERSION` is 15 in code, while the health-exchange spec still says 14.

## Goals / Non-Goals

**Goals:** one spelling per name across JSON, config and key-binding actions, with the protocol bump that tells old clients apart.

**Non-goals:** any change to CLI flags, config keys or what any field means.

## Decisions

1. **Flip `rename_all` on every camelCase type to `snake_case`, and make no per-field renames.** The attribute is the only source of wire names, so the rename can't miss a field the way a hand list would. Rejected: renaming only the ten names that change, which would leave camelCase attributes that bite the next multi-word field.
2. **No `serde(alias)` for old keys.** Ship has no releases, so the only callers are builds from main. An alias only affects reading, so output would change anyway, and every alias is a second spelling to keep. Rejected: accepting `protocolVersion` in health so a new client can say "server is protocol 15" instead of "malformed health". Both errors exit 1 before attaching, and the message names the server.
3. **Old saved files rely on the existing serde defaults.** `right_click_passthrough`, `exit_behavior` and `scrollback_bytes` already default when absent, and `Pane` doesn't deny unknown fields, so a protocol-15 file parses. Its panes take the built-in defaults for those three. Rejected: an alias on those fields just for the saved file.
4. **Protocol 16.** The protocol is what decides whether a client and server can work together, and these wire names have changed.

## Risks / Trade-offs

- [A protocol-15 saved file loses three pane settings once] → Tabs, names, structure and zoom survive. Only builds from main are affected.
- [Scripts written against main that read camelCase keys break] → It's a breaking change, called out in the pull request.
- [A camelCase key is missed in docs or tests] → The rename is in the type attributes, so a grep for the old names across the repo, excluding the archive, must come back empty.

## Migration Plan

Update client and server together, as for every protocol bump. An old client or server on either side is refused at the health check. Scripts rename the six keys and four values.
