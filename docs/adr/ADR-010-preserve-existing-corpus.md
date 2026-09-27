# ADR-010 — Preserve and explicitly migrate historical evidence

Status: partially superseded by [ADR-013](ADR-013-fresh-national-source-census.md).

## Original decision

Preserve the accumulated source corpus rather than silently deleting it or treating a parser/schema
refactor as permission to reacquire everything. Changes to existing persisted representations need
an explicit, verified migration with backup and rollback; observations and provenance survive.

## Supersession

ADR-013 removes old-corpus migration/import as a prerequisite or population source for the **fresh
census**. Use a fresh store and unused durable namespace, acquire that run's evidence anew, and reuse
it within the run. Historical counts, receipts and seals cannot establish fresh-run completion.

The preservation obligation still applies to existing stores and artifacts. This decision does not
assert that a current legacy-import CLI exists or authorize automatic conversion on startup.
[FJALL_SCHEMA.md](../../FJALL_SCHEMA.md) owns actual schema behavior, including explicit historical
shapes; [backup procedures](../FJALL_BACKUP.md) own safe restore and validation.
