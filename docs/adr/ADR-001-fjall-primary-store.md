# ADR-001 — Fjall is the system of record

Status: accepted.

## Context

The workload retains append-heavy source observations, reversible identity decisions and durable
receipts over multi-day runs. Recruiters consume exports, not an interactive SQL service. A second
database would add operational state and competing ownership without a demonstrated query need.

## Decision

Use embedded, version-pinned Fjall as the authoritative evidence store. Do not add PostgreSQL or a
parallel database. Analytical read models belong to `census-report`; they are derived, never a
replacement authority for source evidence.

Preserve observations and source provenance. Changes to persisted representation require an
explicit schema revision and migration for affected existing stores, not silent reinterpretation.
The fresh-run boundary in [ADR-013](ADR-013-fresh-national-source-census.md) forbids importing the old
corpus into the new census; it does not waive historical-data preservation.

## Consequences

The application owns key design, transactional receipts, durability, bounded reads and recovery.
A live read snapshot is not a restartable backup. Single-process database ownership does not mean
all in-process readers must stop while writes occur.

[The table registry](../../crates/census-store/src/table.rs) owns implemented tables and keyspaces;
[backup procedures](../FJALL_BACKUP.md) own backup/restore operations. Keyspace counts, source-line
inventories and benchmark results are deliberately not duplicated in this decision.
