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
| parser goldens | `crates/midwest-census/tests/golden` | 112 | `db95f4b16f485a28a80b7b7f09aafdb1` |

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
   measured with `Store::walk_table`. The repair path is a self-heal in the derived write; the
   measurement needs a built binary, so it is pending, not passed.
2. **Row counts per table** — the file-level digests above pin the corpus bytes but say nothing
   about per-table row counts; those come from `cargo xtask census-status` once the tree builds and
   are the numbers the seal's reconciliation is judged on.

## What this freeze does not constrain

`fjall` and `http` are deliberately outside the digest: the first is written by every derivation,
the second is an immutable fetch cache. A migration that changes either is not a freeze violation —
it is exactly the work being measured.
