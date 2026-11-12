# Spec Delta

## MODIFIED Requirements

### Requirement: Setting tags over HTTP
`PATCH /api/tabs/{id}` and `PATCH /api/panes/{id}` SHALL accept `tags` as a JSON Merge Patch (RFC 7396): a name left out SHALL stay as it is, a tag object SHALL set that name, `null` SHALL remove it, and `"tags": null` SHALL remove every tag.

#### Scenario: Patch one tag among several
- **WHEN** a pane has `a` and `b`, and a caller patches it with `{"tags": {"b": null, "c": {"value": "3"}}}`
- **THEN** the pane has `a` unchanged and `c`, and no `b`

#### Scenario: Remove all over HTTP
- **WHEN** a caller patches a tab with `{"tags": null}`
- **THEN** the tab has no tags
