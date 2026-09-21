# ADR-001: Fjall remains the primary durable store

## Status

Accepted. Implemented: `crates/midwest-census/src/store/` (Fjall keyspaces `entities`,
`journal`, `meta`), `crates/midwest-census/src/bin/midwest-serve.rs`.

## Context

The census retains every observation, source identity, piece of evidence and reconciliation
decision for a nationwide athlete population, and it must survive restarts, crashes and
multi-hour runs without an operator babysitting a server. Two candidate stores were considered:

1. an embedded LSM key-value store (Fjall), owned by the process; or
2. PostgreSQL, with the pipeline as a client.

The primary downstream consumer of the finished census is an Excel workbook produced once per
run, not an interactive analyst issuing ad-hoc SQL. The census workload is a small number of
writers, append-shaped writes over immutable observations, and prefix scans over deterministic
binary keys.

## Decision

Fjall is the system of record for the census:

* canonical entities (athletes, schools, coaches, meets, performances);
* source identities and raw observations;
* evidence and observation digests;
* result relationships and coverage state;
* review cases and reconciliation outcomes.

PostgreSQL is **not** introduced. If an interactive query surface is ever required, it is built
as a projection over Fjall, not by moving the source of truth.

## Consequences

* No database server, no connection pool, no network hop, no schema migration tool on the
  critical path; a run either takes the store directory lock or fails with the reason.
* Re-running after a crash is a `Store::open` away. There is no separate "is the database up?"
  failure mode.
* Schema evolution must be handled explicitly: new entity fields are additive in the stored
  JSON payloads, and key encoding changes require a revision marker in `meta` — Fjall will not
  enforce constraints for us.
* Analytics over the raw corpus (ad-hoc joins, window functions) are deliberately out of scope;
  the workbook and the store's own stats are the supported query surfaces.
* Backup and restore are file-level operations (`tools/`, `deploy/` units describe the layout),
  so a restore drill is part of the verification program rather than a DBA procedure.
