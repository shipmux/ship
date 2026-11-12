# Design

The design for this change lives in three papers under `design/`. They are the source of truth, and this file only points at them.

- [`design/product.md`](design/product.md): additions an older client can ignore need no protocol bump; `process` replaces `status`; `/api/v0` becomes `/api`; request bodies keep ignoring unknown fields. Locked, amended through A-1.
- [`design/architecture.md`](design/architecture.md): shape A, raw lines under one skipping decoder, with `ship watch` as a pipe; `/health` before each reattach; request bodies unchanged, with the strictness options compared in decision 3. Locked, amended through A-2.
- [`design/program.md`](design/program.md): two slices, first the shapes, then the tolerant client. Locked, amended through A-1.
