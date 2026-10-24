# Result-context capacity, 2026-10-24

Bead: `athletic-rust-pipeline-5301`. Base:
`51307f0fc193c190e5bdf06d2913a3818f1b489a`.

The original unchanged `workbook:co2027-pr-1141` recovery exited 1 after
7,844,822 ms. Its 1,000,001st verified context membership exceeded the old
1,000,000 limit. It produced no publication. The original Store, requests,
keys, failure receipts, staged proofs and Downloads workbook remain protected.

## Change

The existing result-context accumulator now admits at most 16,000,000
memberships and 1 GiB of logical membership bytes. The distinct source-capture
body accumulator has its own 2 GiB limit. Its previous shared constant still
limits the separate group index and replay archive to 512 MiB. The retained
journal remains limited to 262,144 entries and 512 MiB.

Membership identity, deduplication, per-entity limit 65,536, capture inventory
limit 40,000, charge-before-insertion behavior, selection, eligibility, schemas
and the semantic-kernel source list are unchanged. This adds no export path.

Read-only diagnostics checked all 1,220,513 rows of a pending physical
performance inventory. Conservative retained-content bounds were 10,984,617
memberships, 607,815,474 logical bytes, 14,992 capture representations and
1,443,644,626 declared capture-body bytes. The largest conservative per-entity
count was 2,584. These are conditional physical-content bounds: original-job
binding, exact typed demand, actual capture reads, closure fit and journal/archive
fit are unproved. The scans do not establish accepted athletes or eligible marks.

## Executed checks

Commands ran from the isolated worktree with CI unset, build jobs 1, pinned
Moon on PATH and owned SSD scratch `qualification-1257`:

```text
tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
MOON_RUST_TEST_THREADS=1 tools/moon-local run pipeline:tests-scoped -- -p census-report --lib -E 'test(export::qualification::participation)' --no-tests fail
```

Final gates exited 0: three tasks, 9.851 seconds, session 60281 reaped.
Configured forced argument-count advisories remain visible. Focused tests
exited 0: 65 passed, 1,475 outside-filter tests skipped, session 50812 reaped;
Nextest run `f6b8f236-fe3e-4321-86b1-fe6b29def603`, 83.815 seconds of tests.
The initial combined gate exited 1 solely for test-expression formatting;
canonical `pipeline:fmt-write` exited 0 and the final gates above passed.

Tests insert 1,048,576 distinct memberships across 128 entities, 65,536 proofs
for one entity and 40,000 distinct capture keys. Literal count/byte boundaries,
exact one-over typed refusals, duplicates, identity separation and unchanged
state on refusal are checked. Large-byte boundaries seed accounting scalars;
they do not allocate 1 or 2 GiB or prove whole-job RAM fit.

## Evidence identities

Retained under `/tmp/athletic-work-20261021/`:

| Artifact | SHA256 |
|---|---|
| `worksheet-context-capacity-contract-1311.json` | `732ef22316a24103c02472d317b59d9a5c26fdff5cf5dab472a44aa6d4f4cf3b` |
| `worksheet-context-capacity-final-source-pin-1311.json` | `8df8e2bfd408fe3db6e2972326dcae5bb7bcc7fe77eda3f399a0e3d55aaa4177` |
| `worksheet-context-capacity-gates-final-1311.log` | `f0fdfa5a19e95160f4020cfff62093910d113232a27b8d3c39c1d869ef4d168c` |
| `worksheet-context-capacity-tests-1311.log` | `07aea5b72ee44481df420cad1c38f05a0ad07f71f86fd02481d13a4221c506f2` |
| `worksheet-context-capacity-code-review-1311.txt` | `ef3adefd148ce3146f764fc6901284d098eaf788e6509366864293fff15bfb21` |
| `worksheet-context-capacity-test-review-1311.txt` | `99c8b56816b3f51dd71a1cd809b3d6a5ee4fd7ef484ab9fe821909344801204a` |
| `worksheet-pending-performance-bound-1308-result.json` | `3862f5a0b778c0d82390d6ae137f161a3b10a5b5d560d79990dad6ee25aabfcd` |
| `worksheet-pending-entity-bound-1310-result.json` | `1d5b7139be1dd49651bb0b35189a52314519558c9905c47d22dac671477b2309` |

The three final source hashes are recorded in the source pin. Independent
CODE and TEST reviews bind those exact bytes and the executed scoped checks.

## Open acceptance

Full release gate, original-job retry, journal/archive capacity, current-school
and calendar authority, eligible population, Michigan Upper Peninsula membership,
five-state frozen publication, normalized-output agreement, original-request
replay and fresh-process cold read remain open. This repair does not complete
the recruiting workbook or authorize an automatic retry of the original job.
