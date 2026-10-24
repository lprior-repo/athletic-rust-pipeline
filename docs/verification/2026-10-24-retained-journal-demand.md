# Retained journal demand diagnostic, 2026-10-24

Bead: `athletic-rust-pipeline-yts0`. Base:
`0894d878823be5aa4603858eeec4ec5ce17277e9`.

The original `workbook:co2027-pr-1141` recovery failed before journal completion
and publication. PR139 repaired its result-context capacity, but retained-journal
and replay capacity remain unqualified. Declared capture lengths cannot bound
the actual values recorded before interpretation rejects a mismatch.

## Change and boundary

`capture-protocol journal-demand` opens an explicitly named, stopped current
qualification Store and measures all present roots in the existing owned-capture
and owned-meet phases through one held snapshot. It reuses the Store scanner,
complete-chunk validation, bounded payload reader and existing retained-bundle
serializers. It streams canonical byte counting and SHA256 without collecting a
second journal or writing a publication.

The separate diagnostic admits at most 4,000,000 physical rows and 16 GiB of
physical encoded bytes per phase, the existing 1 MiB physical-record ceiling,
4 MiB per logical payload, and 32 GiB of counted canonical JSON. It publishes a
bounded scalar report only after both phases exhaust successfully. The assumed
4 MiB recorded read grant is conservative, not a measured minimum grant.

Normal admission, journal/archive limits, schemas, eligibility and source
selection are unchanged. All 210 semantic-kernel input bytes retain digest
`3e847686e4c77a12d9920b838416ccf8a7b12b42a5430ec4db1536ae6462c174`.
Review caught and removed an initial diagnostic re-export from `milesplit.rs`;
the public associated method uses the existing exported bundle type instead.

The diagnostic body invokes read APIs only. The required existing normal opener
can seed absent internal counters/row marks and sweep immediate stale `.part`
files under `entities`/`out`; this is not a physically read-only Store opener.
The unchanged-state tests cover initialized fixtures. Actual original-Store
measurement and protected-artifact verification remain separate operations.

## Executed checks

CI was unset; pinned Moon, build jobs 1 and owned SSD scratch
`qualification-1257` were used. Test invocations additionally set
`MOON_RUST_TEST_THREADS=1`; other invocations unset that variable.

```text
tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
tools/moon-local run pipeline:tests-scoped -- -p census-store --lib -E 'test(read::view::membership)' --no-tests fail
tools/moon-local run pipeline:tests-scoped -- -p census-crawl --lib -E 'test(milesplit::retained_view) | test(semantic_inventory_v2::kernel)' --no-tests fail
tools/moon-local run pipeline:tests-scoped -- -p census-service --bin census-service -E 'test(cli::capture_protocol)' --no-tests fail
```

All four actual processes were reaped with exit 0. Final gates completed three
tasks in 51.390 seconds. Store tests passed 66/66, crawl/kernel tests 26/26 and
CLI tests 58/58: 150 focused cases, including 28 new diagnostic tests. Tests
outside the filters were skipped: 1,730 Store, 2,182 crawl and 250 CLI cases.
Configured argument-count advisories remain visible; the shared scan seam
retains its cohesive explicit parameters without suppressions.

The tests exercise 25,001 actual roots beyond normal admission, chunk closure,
same-snapshot behavior, exact existing canonical serialization, null/escaping,
payload/key refusal, explicit-root and historical-format refusal, and output
errors. Independent review found scalar counter tests would miss a bypass in
the actual writer. Four private seeded writer tests now prove exact-limit
acceptance, excess/overflow refusal and unchanged count/hash after refusal,
without allocating GiB. The initial E0432 test-import failure and subsequent
successful repair remain in separate logs.

The 5090 request returned no usable patch. The 3090 counter draft was adapted
to standard checked counting and SHA formatting before review. Both exact
requests terminated and their clients were reaped; no GPU lease is retained.

## Evidence and open acceptance

Evidence is retained under `/tmp/athletic-work-20261021/`. The final 12-file
source pin is `worksheet-journal-demand-source-pin-final-1315.json`, SHA256
`35d11d7b538c0836bcf7525b2c0f79b4b51342a1c820c14613578823ea99af0b`.
Original contract, API amendment, writer-test amendment, exact command logs and
independent production CODE and TEST reviews are bound by the final qualification
receipt. A passing scoped suite is not the full release gate.

Actual journal demand is still unknown at this source checkpoint. Present-phase
measurement does not prove original-job binding, absent/legacy reads, actual
minimum read grants, outer replay V4 bytes, attachment closure, descriptor and
cold-decoder fit, RAM fit or workbook correctness. Those obligations keep yts0
open. Full five-state accepted population, eligible marks, source-backed UP
membership, frozen publication, normalized outputs, replay and cold read remain
open under the recruiting-workbook goal. Existing Store, staged proofs, requests,
keys and user workbook remain protected; this change schedules no automatic
original-job retry.
