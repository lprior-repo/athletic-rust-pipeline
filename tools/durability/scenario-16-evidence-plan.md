# Scenario 16 evidence plan — golden census determinism

Wrapper: `tools/durability/scenario-16-golden-census-determinism.sh`.
Owns the harness side of catalog row 16 (`docs/NATIONAL-CENSUS-FAULTS.md`):

> Replay frozen real captures and retained advice with fixed run semantics; exact IDs and semantic
> artifacts match. Separate nondeterministic operational measurements rather than fabricate them.

plus the two catalog subcases naming scenario 16: quarantine one malformed source object while
unrelated work continues (retaining its bytes and rejection locator, never an empty success), and
interrupt export before promotion (previous accepted generation stays published, a newer partial
file is never selected by filename or timestamp).

Invocation:

```sh
SCRATCH_STORE=$PWD/var/durability-scratch-<tag> \
  env -u CI tools/moon-local run pipeline:durability -- scenario-16-golden-census-determinism
```

`EVIDENCE: <dir>` prints first; every lane, log and manifest lives under it. Nothing is deleted.

## Lanes, commands and where each assertion lives

| Lane (log name) | Command run by the wrapper | Fixed expectation |
|---|---|---|
| `replay-pa-piaa` | `tools/moon-cargo run -p xtask --bin xtask --locked --offline -- replay pa_piaa` | xtask's own golden-record equality: the parsed school-id set of each `directory_alpha_{a,b,z}.html` must equal the `association_id` set recorded in the sibling `golden_directory_alpha_*.json`; `PROVENANCE.json` must match every listed capture in bytes and sha256; `robots.txt` must still block `/officials/directory/`, permit `/schools/` and not block `/`; the details page must parse a non-empty school name. The wrapper re-derives each count from the golden file and requires the printed line to carry that exact count, plus `A J McMullen School`/`12048`, `contacts=1`, and all ten per-capture summaries |
| `parity-pipeline` | `tools/moon-cargo test -p census-service --test parity_pipeline --locked --offline -- --nocapture` | the single test embeds the authentic owned capture (`crates/census-crawl/src/milesplit/owned/fixtures/troy_725218.json`) and seeds `crates/census-crawl/tests/fixtures/milesplit` raw captures into a fresh tempdir store with `with_offline(true)`; it asserts `requests == 0`, `from_cache == 3`, owner rows/errors, exact athlete ids `14222592`/`11357806`, result ids `201782263`/`201782277`/`201782806`, retained archive digest equal to the capture sha256, and identical journal payloads on the repeat; the same test then rebuilds the fixture store from scratch and requires equal consolidate counts, equal result-adapter projection, equal workbook rows/mapped athletes, and a *different* publication digest |
| `review-advice-replay` | `tools/moon-cargo test -p census-review --lib --locked --offline -- consensus::tests::replay --nocapture` | retained dual advice replays without re-asking (`requested == 0`, `failed == 0`), the stored row and `Resolved` state are unchanged, and exactly one receipt exists; historical single-model acceptance is re-asked and cannot survive disagreement; a case superseded during advice is not resurrected by the checkpoint; a missing subject stays a durable unresolved review |
| `crawl-quarantine` | `tools/moon-cargo test -p census-crawl --lib --locked --offline -- quarantine --nocapture` | every durable quarantine boundary allows fresh publication without evidence loss; a crash before reason publication blocks neither recovery nor orphan removal; an occupied destination is an error and is never replaced; an interrupted metadata rename does not hide immutable-archive corruption; repeated damage keeps distinct evidence; a quarantine IO failure preserves the damaged body and refuses publication; an unreadable result page is quarantined and the walk continues |
| `crawl-roster-quarantine` | `tools/moon-cargo test -p census-crawl --lib --locked --offline -- milesplit::tests --nocapture` | an empty known roster is a named gap, not negative identity evidence; a missing name does not erase a readable provider identity; rejected rows keep provider identity and exact UTF-8 span; malformed HTML fails loudly; lookalike hosts cannot supply roster identity |
| `service-generation-crash` | `tools/moon-cargo test -p census-service --lib --locked --offline -- --exact school_address::generation::tests::crash_boundaries_never_publish_a_mixed_generation --nocapture` | the test re-executes its own binary at each commit point and requires exit 77; after every injected point the previous accepted generation stays verifiable and no mixed generation is published |
| `exporter-kill-restart` | `tools/moon-cargo test -p census-service --test exporter_kill_restart --locked --offline -- --nocapture` | a real SIGKILL of the workbook exporter mid-staging leaves no `current`; the restart republishes and verifies; an uninterrupted export exits cleanly |
| `school-address-publication` | `tools/moon-cargo test -p census-service --test school_address_publication --locked --offline -- --nocapture` | a blocking destination preserves the previous publication; an unmanifested external baseline is not consumed or republished; a missing manifest artifact refuses readback and preserves `current` |
| `store-derived-generations` | `tools/moon-cargo test -p census-store --test derived_generations --locked --offline -- --nocapture` | one staged row per id with last-write-wins; a published generation is invisible until publish and durable across reopen; a same-operation/digest repeat is a repeat and a different digest is refused; a stage against moved evidence is refused; publication reclaims the superseded generation; reclaim honours its budget and finishes next call; carry-forward; a generation larger than one staging batch publishes whole; non-generation-partitioned tables are refused |
| `reconcile-stage-gate` | `tools/moon-cargo test -p census-reconcile --lib --locked --offline -- index::stage_gate_tests --nocapture` | a changed input publishes a new generation holding the whole row set; a repeated pass publishes nothing and reclaims nothing (the abandoned stage is gone); located unsupported-cohort review rows survive without a canonical subject |
| `service-worker-export` | `tools/moon-cargo test -p census-service --test recovery --locked --offline -- --exact worker_export::exporter_restart_republishes_identical_snapshots_and_totals --nocapture` | a restarted exporter republishes identical snapshot bytes and identical totals for the same frozen store |

Every `tools/moon-cargo test` invocation runs under `timeout --signal=TERM 2700s` with
`--locked --offline --nocapture`. Each log records its own `COMMAND:` line, the full compiler and
harness output, and at least one `test result:` line. The wrapper fails on a nonzero exit, on any
`SKIPPED:` marker inside a lane log, on any non-green or skip-affected `test result:` line, when the
outermost pass count is below the lane's required number, and when any required test name is absent
from the log. Nested child test binaries (the crash-boundary lane) are allowed: every result line
must still be green with zero ignored and zero failed.

## Frozen inputs and integrity

Replayed bytes come from two places, both read-only:

* `crates/census-crawl/tests/fixtures/` — committed byte-exact captures (`pa_piaa`, `milesplit`,
  `wiaa`, `wiaa_results`, `ohsaa`, `athleticlive`, `athleticlive_athletes`).
* `crates/census-crawl/src/milesplit/owned/fixtures/troy_725218.json` — compiled into the test binary
  with `include_bytes!`.

Before the first lane and after the last one the wrapper hashes every file under both trees
(`sha256sum`, sorted by path) into `frozen-captures-before.sha256` /
`frozen-captures-after.sha256` and fails if they differ. A re-crawl cannot have produced or changed
any replay result, and no lane may mutate a capture.

## Deterministic assertions versus operational measurements

Asserted (deterministic): capture digests and byte counts, golden id sets, school/athlete/result ids,
row and error counts, cache answers (`from_cache == 3`), `requests == 0`, receipt counts, review
states, journal payload equality, generation visibility/durability/reclaim outcomes, published
workbook rows and mapped athletes.

Recorded but deliberately not asserted (nondeterministic): wall-clock and link times, test-run
durations (`finished in …`), cache byte counters, store paths, and the rebuilt publication digest —
the determinism lane asserts that the rebuilt store's digest *differs* while its semantics match, so
publication identity is separated from semantic equality instead of being fabricated. The
`qualification_*` examples under `crates/census-crawl/examples/` apply the same separation inside
their `qualification.json` (`fetch_stats` next to identities); they need retained captures outside
`var/` and are therefore not driven by this wrapper.

## Read-only audit of a green run

```sh
EVIDENCE=<printed EVIDENCE: path>
grep -E '^(COMMAND|SECTION|PROOF|NOTE|LIMIT|PASS):' output-of-runner-or-<EVIDENCE>/../*.txt
grep -n '^test result: ' "$EVIDENCE"/*.log
grep -n '^SKIPPED:' "$EVIDENCE"/*.log
sha256sum -c "$EVIDENCE/frozen-captures-after.sha256"   # from the repository root
```

`sha256sum -c` re-verifies every replayed capture without writing, and `grep '^test result: '`
re-verifies the per-lane pass counts printed by the harness. The per-lane logs are the primary
evidence: each contains the exact command line, the compiled test names and the assertions above.

## Limits

* No live national store, production snapshot or fresh crawl is replayed; the frozen corpus is the
  committed fixture captures and one compile-time embedded owned capture.
* Retained-advice replay runs through `census-review`'s in-process loopback lanes; no local model
  server or GPU lane is contacted, and model-output failure modes stay scenario 13's.
* The derived-generation lanes exercise interruption as an abandoned unpublished stage and as
  process-level export interruption; an OS signal delivered inside index materialization remains
  scenario 17's subcase.
* Missing prerequisites produce `SKIPPED:` and exit 0 only for genuinely absent tools
  (`timeout`, coreutils, the moon runner, `tools/moon-cargo`); a missing committed capture tree or a
  failed assertion is `FAIL:` and exit 1.
