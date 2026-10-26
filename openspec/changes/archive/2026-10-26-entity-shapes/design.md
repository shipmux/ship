# Design

The design for this change lives in three papers under `design/`. They are the source of truth, and this file only points at them.

- [`design/product.md`](design/product.md): what changes and why. Every entity has one shape, inputs are subsets of it, PATCH follows JSON Merge Patch, zoom is a tab field, and every body and query reports problems in one 400 `validation` error. Locked.
- [`design/architecture.md`](design/architecture.md): shape A. serde decides whether a request is valid, and the server's own OpenAPI schema explains a refusal. Locked.
- [`design/program.md`](design/program.md): the file map and four build slices: shapes, the error path, zoom, then docs and specs. Locked.
