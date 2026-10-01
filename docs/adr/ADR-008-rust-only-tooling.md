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

## Amendment — 2026-09-29: admitted research readers, pending Rust port

Status: accepted, owner-authorized.

The address pipeline under `hs-address-pipeline/` and the TSSAA school reader
`parsers/tn_tssaa_school.py` are admitted to the tree as research material: they carry the discovery
strategy for school postal addresses and the Tennessee association reader the Rust adapters still
owe. They are excluded from the workspace Python scan by explicit skip paths in
`xtask/src/contract/tree.rs`, and this exclusion is temporary. The next phase ports them, one reader
per Rust adapter, through `cargo xtask new-source`; the port removes the skip entries rather than
widening them. Until then they are reference material with no execution authority: no pipeline verb,
handler or fixture may invoke them, and no output of theirs is admitted as census evidence.
