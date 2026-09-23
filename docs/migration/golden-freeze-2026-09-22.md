# Golden corpus freeze — 2026-09-22

Basis: the working tree as of this date (HEAD `ea81c56` plus the uncommitted wave work in
progress). Purpose: give the identity migration (Wave A) a fixed reference, so "the corpus was
migrated, not rebuilt" is a comparison against numbers rather than a claim. See `module-map.md`
for the crate survey this freeze accompanies.

## Method

For every directory below, `find <dir> -type f -print0 | sort -z -T /home/lewis/spool | xargs -0
sha256sum | sha256sum`, i.e. the digest of the sorted per-file digests. The sort spool must be on
`/home` — `/tmp` is a 62 GB tmpfs and a sort that spools there fails (`EDQUOT`), which silently
produces the digest of an **empty** stream. Both the combined digest and the file count are
recorded because either alone can be made to match by accident.

## Frozen values

| area | path | files | combined digest (first 32) |
|---|---|---|---|
| legacy entity journals | `var/midwest-census/entities` | 53 | `b9a7901d9419194d222ff27a499034fc` |
| published snapshots | `var/midwest-census/out` | 30 | `2fd6dae582a7b9ad296e0904192dcdd9` |
| journal writes | `var/midwest-census/journal` | — | `349b072ba943a20569a174d87cfcbb14` |
| parser goldens | `crates/midwest-census/tests/golden` | 112 | `db95f4b16f485a28a80b7b7f09aafdb1` → `bce0ffdf9fc36375951cf21009fc54ba` (2026-09-23) |

**Update 2026-09-23 — one golden, one row.** `pipeline__workbook-shape.json` moved with the
`source_observations` carrier: the Run Metrics sheet prints one row per table's append counters
(`workbook/meta/metrics.rs::store_counters`), so the new table adds one label/value pair and the
sheet's cell count goes 126 → 128 (workbook total 13,941 → 13,943). No sheet lost a row, no other
golden changed. The directory digest above is recomputed from the tree at the time of that edit;
the file count is unchanged because nothing was added or removed.

Sizes for scale: `entities` 2.7 GB · `fjall` 2.0 GB (52 files, the live database — not hashed
here, it is the system of record and is written by every derived pass) · `http` 6.6 GB (80,460
cached bodies, immutable) · `out` 2.3 GB · `journal` 4.4 MB.

## Legacy journal row counts (the seven derived tables)

Measured on a copy of the live root (`/home/lewis/census-live-copy`) before any migration pass, so
these are the *pre-migration* numbers the strengthened identity model has to be judged against:

| derived table | rows in `entities/<table>.jsonl` |
|---|---|
| `source_identities` | 2,504,148 |
| `review_cases` | 27,967 |
| `conflicts` | 3,695 |
| `coverage` | 210 |
| `identity_verdicts` | 18 |
| `snapshots` | 1 |
| `source_access` | 0 |

These are exactly the tables `store/legacy.rs` now *skips* on open (they are read models; the
derivation is their author), which is why the row counts above are recorded here as a fact about
the retained corpus rather than as something the import is expected to reproduce. If they were
imported by an open that predates the gate, `Store::walk_table` shows them as rows keyed under a
foreign sequence — item 1 under "Open checks" below.

## Open checks against this freeze

1. **Derived-table import leak (pre-run gate).** `var/midwest-census/entities` holds journals for
   all seven derived tables (`source_identities` 524 MB, `review_cases` 7.1 MB, `conflicts` 934 KB,
   `coverage` 40 KB, `snapshots`, `source_access`, `identity_verdicts`). If this root was opened
   before the derived-import gate landed, those rows sit at `seq > 0`, where `stage_derived` (which
   writes only the `DERIVED_SEQUENCE` key) and `drop_unnamed` (which drops only unnamed ids) can
   never displace them. Required before the run: `foreign_sequences == 0` on those seven tables,
   measured with `Store::walk_table`. The repair path is a self-heal in the derived write.
   The instrument is `crates/midwest-census/tests/corpus_walk.rs` under
   `WALK_ROOT=<root> cargo test -p midwest-census --test corpus_walk -- --ignored
   --nocapture`; it is `#[ignore]`d because an unset `WALK_ROOT` is not a claim about the
   code.

   **Measured 2026-09-22** against `var/midwest-census`, once the run above released the store
   lock (the single-open rule refuses a second handle, so the reading has to wait for the
   writer): **every one of the seven derived tables walks clean** — `source_identities` 2,507,541
   rows, `review_cases` 27,967, `conflicts` 3,695, `coverage` 215, `snapshots` 2, `source_access`
   0, `identity_verdicts` 18, each with `foreign_sequences == 0` and `highest_sequence == 0`, and
   every ledger count equal to the walk's row count. Nothing was imported under a foreign
   sequence, so the repair path is not needed on this root. The observation log's own tables
   report `foreign` > 0 by construction (each append carries its own sequence) — that is the log's
   shape, not the leak this check is about. Note that these counts are past the freeze's digests
   (`source_identities` was 2,504,148 when it was written), because the run between them appended
   to the corpus.
2. **Row counts per table** — the file-level digests above pin the corpus bytes but say nothing
   about per-table row counts; those come from `cargo xtask census-status` once the tree builds and
   are the numbers the seal's reconciliation is judged on.

## What this freeze does not constrain

`fjall` and `http` are deliberately outside the digest: the first is written by every derivation,
the second is an immutable fetch cache. A migration that changes either is not a freeze violation —
it is exactly the work being measured.
