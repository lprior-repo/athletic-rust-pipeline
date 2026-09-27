# ADR-008 — Rust-only pipeline and tooling logic

Status: accepted.

## Context and decision

A second language implementation of acquisition, parsing, review or output creates competing rules
and delivery paths. Implement product and developer-tool logic in Rust. `census-service` owns the
application; `xtask` provides the common developer command surface.

Historical research scripts were removed without deleting their unique captured evidence. Native
Restate replaces shell orchestration of the census; Python and shell scripts are not pipeline steps.
Operational wrappers for gates, deployment and isolated fault injection may invoke real binaries;
they must not become a second business-logic implementation.

## Consequences

Extend the existing Rust entry points instead of reviving deleted scripts. Research reports retain
provenance, not executable authority. The current verb catalog lives in [xtask](../../xtask/README.md),
and native lifecycle procedures live in [deployment-lifecycle.md](../deployment-lifecycle.md).
