# Research notes

These notes explain experiments, comparisons and options considered while building Ship. They are historical evidence, not the current guide or permission to implement a design. A note can call something "open" even when a later change settled it.

For today's behavior, use the [guide](../site/README.md). For exact requirements, use [OpenSpec](../../openspec/specs/).

## Features that have since shipped

| Research | What to read now |
| --- | --- |
| [Agent state detection](agent-state-detection.md), an early proposal to read agent screens | [Agents](../site/agents.md). Ship uses agents' own Program Status Protocol reports, not those screen rules. |
| [Configuration and plugin ownership](config-plugin-ownership.md), a comparison of Herdr and Zellij | [Configuration](../site/configuration.md). Config files, reload, keys and menus are implemented; a plugin runtime is not. |
| [Prompts and plugins](prompts-and-plugins.md), options for rename prompts and external programs | [Tabs](../site/tabs.md#renaming), [Panes](../site/panes.md#renaming) and [run actions](../site/configuration.md#running-programs-from-keys). The old action names and sample config are not current instructions. |
| [Edit scrollback](edit-scrollback.md), a comparison of editor placement and cleanup | [Edit scrollback](../site/panes.md#edit-scrollback). The editor now opens in the pane's stack, not the proposed split-and-zoom design. |
| [Typed relay](typed-relay.md), the original internal publication design | [`relay.rs`](../../crates/ship-core/src/relay.rs) and [session observation requirements](../../openspec/specs/session-observation/spec.md). The note predates the implementation. |

## Terminal and transport experiments

These preserve measurements and reproduction details. Their protocols and source paths may predate the current code. The reported benchmark runs were on macOS. Ship has since worked in Cyan's Linux use too; the platform limits in these notes apply to the old experiments, not to today's application.

- [Terminal transport handoff](terminal-transport-handoff.md) explains why compressed full snapshots replaced screen diffs. The [preserved experiment](../../experiments/terminal-transport/README.md) maps temporary paths to durable sources and results.
- [Pane terminals transport probe](pane-terminals-transport-probe.md) compares input streaming, compression and HTTP transport behavior.
- [Request ordering probe](request-ordering-probe.md) compares numbered requests with streamed input for typing, dragging, floods and paste.
- [Screen cost per workload](screen-cost.md) compares Ship, Herdr and Zellij CPU use and explores rendering optimizations.
- [Terminal feasibility](terminal-feasibility.md) records the earlier terminal wrapper and screen-diff experiments. Its structdiff pipeline was not adopted.
