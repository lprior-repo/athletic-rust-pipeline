# ADR-036: SPEC-20261010 bounded-memory decision gates a-g

## Status

Accepted 2026-10-11 by owner approval (beads `zzq8`, `s0ly`, `ysru`).

## Context

SPEC-20261010 (bounded-memory export redesign, epic `cnng`) requires owner decisions on seven
gates before implementation of limits, admission model, oversized-component behavior, artifact
envelopes, sheet limits, archive migration and XLSX support may proceed. The gap map at
`var/spec-20261010/gap-map.md` records which requirements remain unmet in the current tree; no
bounded-memory prototype is treated as satisfying a current requirement.

## Decision

### Gate a: resource profile and fail-closed refusal

Approved as **versioned test starting values**, not measured production optima:

- visitor window 25,000 records and 64 MiB encoded, stopped before either bound and before the
  separately granted decoded budget;
- frozen row envelope 1 MiB including newline; frozen data/inventory page 4 MiB; metadata record
  64 KiB; inventory node 64 children, depth 8;
- identity reuse component 128 MiB decoded; larger requires gate c;
- sort worker 64 MiB charged allocations; merge fan-in at most 32; per-stream I/O buffer at most
  64 KiB;
- store batch 8 MiB encoded and 4,096 rows; journal physical chunk 512 KiB;
- per-codec JSON depth/node/string and XLSX cell/shared-string/ZIP limits are set per codec
  before that codec is enabled.

Fail-closed outcomes (`RowTooLarge`, `BudgetExceeded`, `NeedsPartition`, `UnsupportedUnitSize`)
are accepted even where previously representable inputs are rejected. Crossing a limit retains
evidence and owed work and never drops or truncates rows or declares a partial census complete.
Supported spill is preferred over refusal only with proven equivalence. Production values and the
acceptance procedure require measurement before they replace these test profiles.

### Gate b: accounting model and operational envelope

The operational target is a **16 GiB whole-process envelope**, not the proposed 2 GiB-per-verb
figure, accounted **incrementally and application-owned**:

```
runtime and SDK baseline + engine allowance + admitted stage bytes + queued descriptors
  + measured overhead + drain reserve <= 16 GiB
```

A capped arena/buffer gives a hard application-owned bound; estimates for variable Rust graphs
remain estimates with measured headroom. cgroup `memory.max`/`memory.high` are last-resort
containment (they may kill/throttle) and never substitute for graceful admission and durable
recovery. The two-minute workbook and five-minute seal figures are targets without supporting
measurement; they become acceptance criteria only after input shape, hardware, concurrency,
cold/warm state and durability scope are fixed and measured.

### Gate c: oversized identity components

Version one **refuses with a typed recoverable outcome** naming the component root and required
bytes (`NeedsPartition`/`RowTooLarge` class); the unit stays owed, prior publication is preserved
and no convergence is fabricated. A separately proven disk-backed implementation of the same
predicates is a later option; approving any memory target is not proof that all valid populations
fit the component strategy. Neither option may broaden or simplify which identity outcomes are
produced.

### Gate d: audit format

Adopt the ordered streamed audit record codec with checked counts. The legacy `audit.json` reader
is retained for existing archives and is not extended. Expected IDs are validated via external
sort/unique; no all-ID BTreeMap.

### Gate e: nonperformance sheet limits

A nonperformance sheet that would exceed its supported row/byte limit **fails closed with a typed
refusal** at a documented ceiling. No silent splitting; the publication contract is unchanged.

### Gate f: archive migration and reader policy

Old frozen-input readers are retained read-only. Converting an old archive is a **new immutable
derivative** recording old root, conversion version and new root; conversion refuses with a
precise incompatibility error when a required field, order or provenance rule cannot be
preserved. Unknown versions are rejected before allocation.

### Gate g: XLSX support envelope

The bounded XLSX reader is qualified as an **inline-string-only profile**: bounded XML tokens plus
compressed and decompressed byte budgets; sheets requiring shared strings are refused explicitly.
A disk-backed shared-string lookup is a later separate qualification. Byte-identical XLSX output
remains a separate reproducibility objective; semantic parity plus strict independent readback is
mandatory, and a file hash is never substituted for equality of the intended census data.

### Allocator/cache experiments (bead 4jdn)

Allocator/cache/memtable tuning is not a prerequisite of the bounded data path and stays outside
it until one identity-pinned variable at a time is measured against the working set under this
envelope, with rejected candidates and their evidence retained.

## Consequences

- The admission coordinator (`zqrp`) and limit-bearing capture/admission code proceed against
  these test profiles; production values replace them only through measured evidence.
- `4jdn` remains blocked until measurement is authorized by a separate recorded decision.
- Successor work is bounded by the gap map: durable records (`gn1j`), freeze v2 (`ygso`), one
  decoded generation (`9gm4`), backup promotion window and frontier pruning (`qplc`), v2 journal
  reads (`x997`), derived-manifest coverage and `bind_run` serialization (`8mso`), drain
  certificates (`j75n`), corruption verification (`cz91`), bounded grouping (`0jix`).

## Evidence

- Beads: `zzq8`, `s0ly`, `ysru` (this decision); `cnng` (epic); `4jdn` (deferred).
- Gap map: `var/spec-20261010/gap-map.md`.
- Owner approval recorded 2026-10-11 for all seven gates in one decision session.

## Resource-refusal classification and retained inputs (2026-10-13)

The gate-a limits are admission boundaries, not evidence that an artifact is corrupt. Report
boundaries preserve `RowTooLarge`, `BudgetExceeded`, and `NeedsPartition` as typed resource
refusals with the observed quantity and applicable ceiling. Their diagnostic codes are
`export-row-too-large`, `export-budget-exceeded`, and `export-needs-partition`. CLI context and
nested cleanup retain the primary diagnostic and every secondary cleanup error.

The report-to-`JobError` boundary maps a primary resource refusal to terminal handler status 413,
so a deterministic limit does not enter automatic transient retries. Existing success/request
wire formats and serialized team-attempt outcomes are unchanged. This does not add the same
status contract to seal or open-work wrappers, which retain their existing diagnostic behavior.

A resource-refused retained frozen input must keep its root pointer, immutable objects, input
record, lease and sequence unchanged during resume, cancellation and binding. This applies to
missing-record, Requested, Freezing, Frozen and Abandoned states. Missing record state does not
authorize replacing a root that could not be verified within the supported envelope. Primary
resource errors nested inside Cleanup receive the same protection. Actual format/integrity
errors continue through their existing refusal or retirement paths; unknown versions keep
existing read-only preservation semantics.

This error and preservation correction changes no store, frozen codec, token, lineage or
semantic-policy version. It does not establish bounded decoded allocations or whole-process
memory, and terminal reporting does not itself implement durable resumption after an operator
changes resource limits. Those acceptance requirements remain open.
