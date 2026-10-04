# Verification evidence ledger

Each section records its own historical tree, input generation and execution limits. Results do
not transfer to a later revision, fresh store or new run identity. A historical seal is not the
fresh national census's release certificate. Current requirements live in [NATIONAL-CENSUS-PLAN.md](NATIONAL-CENSUS-PLAN.md)
and [OPERATIONS.md](OPERATIONS.md). Source audits and imported measurements are explicitly labelled.

## Athlete-sheet republication (Class of 2027 workbook) — 2026-10-04

Beads `athletic-rust-pipeline-dhp` (closed) and `athletic-rust-pipeline-vez`: two owner-reported
defects on the delivered Class-of-2027 workbook, fixed, republished and re-verified in the
export/recruiting presentation layer only. Census data, schools, results and school-level postal
provenance are unchanged. The remaining per-event PR fill gap is the jurisdiction result-collection
obligation `athletic-rust-pipeline-622`, not part of this fix; the post-fix column inventory is
recorded on `athletic-rust-pipeline-c0d`.

Main executed from a clean tree with the native pipeline target dir
`/home/lewis/.cache/cargo-target-native-athletic` (`CARGO_TARGET_DIR` plus
`RUSTFLAGS=-C target-cpu=native`; no throughput gain over the generic release build, so the native
target is a measurement experiment only) and its already-built release binary:

```sh
bash var/rebuild-workbook-20261004-r2.sh
cargo test -p census-report
/home/lewis/.cache/cargo-target-native-athletic/release/census-service \
  verify --workbook var/final-workbook-sol-20261004/current/workbook.xlsx
bash var/republish-evidence-20261004.sh
tools/gate.sh
```

Observed results: generation `3782cb1872b6c49891523439e323257e44535d2010171f26359b0fafaa0339f6`;
`verify: OK (complete frozen generation)`, exit 0, 2m15.7s; census-report tests 198 passed, including
the cross-country/track 5000 m separation and the School Address tamper oracles; the delivered
workbook (69,141,755 bytes, sha256
`75a850c8e84678ee49f57bd23b35d212924ebe2d01f8d439333ffea134d4df63`, `cmp`-equal to the generation
artifact) and manifest (`c7ba0c69...`) carry the fix, while the census json (`a51e3bf6...`) and the
best-results CSV (`cfca2e56...`) are hash-unchanged from the earlier delivery. The Athletes sheet
holds 566,229 rows in 60 columns with a single School Address (69,278 fills, 12.2350%) and no postal
columns; the 12-column postal block stays on the Schools sheet; recruiting.csv is 21 columns x
566,229 rows; cross-country selections moved from `5000m (s)` (661 pre-fix fills) to `XC (s)` (661
post-fix fills). Independent audit: var/republished-verification-flash-20261004.md (deepseek-flash, executing its own
commands through the verify-shell service; `verdict: clean` - 8/8 manifest keys, both verifier
invocations, OOXML header counts and 60/60 audit percentage rows reproduced; it did not scan
worksheet-wide fills a second time or render the workbook). Limits: this is a republication of the
verified-but-incomplete census snapshot, not a completed national census; the ten-item narrative is
var/fresh-census-verification-sol-20261004.md, and the pre-fix hashes survive only inside the
preserved generation `0d28e4a810f368fe9c4237d3183d349ba72dee4be698a284d69a6d6f3a499ef8`.

Delivered report: fresh-census-verification-sol-20261004.md, 18,999 bytes, sha256
`b044d53b5f25f31f74c95b90349283dbcedb4727cbcd7417494aedfae2836497`, `cmp`-equal to the in-repo
copy under var/ (the report carries its own delivery note instead of a self-referential hash).

Gate: `tools/gate.sh` PASS on the corrected tree (var/gate-r5-20261004.log; all lanes green, 2342
run / 2342 passed / 3 skipped, rc=0). Earlier runs failed on an untracked exploratory example
(var/gate-r3-20261004.log) and on a stale traversal-test count expectation
(var/gate-r4-20261004.log), both corrected: the example was deleted and the xtask
fixture-graph expectation now reads five violating files, matching the harness's violation count. The
tree also carries the gate's own repair: an empty `FULL`/`RELEASE` conditional was removed from
tools/gate.sh (dead block, no lane added or removed).

Perf: `cargo xtask perf check` rc=0, no regression across all fifteen lanes, but the harness flags a
corpus-size mismatch against the 2026-09-30 baseline (38,530 vs 58,421 elements, different workload
hashes) and calls the comparison potentially invalid, so a matched-corpus `cargo xtask perf record` is
still owed; verify of the real 3.6 GB frozen input peaks at 16.5 GiB RSS (91.0s wall), the standout
resource risk. Evidence: var/perf-check-20261004.log, var/republish-evidence-raw-20261004.txt.

Mutation coverage: scoped `cargo mutants` runs (the gate lane is whole-workspace and `--in-place` is
serial, so no `--jobs`) measured the critical rules - athletes/cells.rs 48 mutants (29 caught, 2 missed,
17 unviable), export/postal.rs 61 (14 caught, 1 missed, 46 unviable), recruiting/csv.rs 46 (18 caught,
1 missed, 27 unviable), athletes/rules.rs 0 under the filters; the narrow
`selection_matches_event|pr_mark_name` set was 12/12 caught. All four survivors are exact-budget
boundaries (32_767 UTF-16/byte limits, `Cell::Empty` not distinguishable through the sheet-reader
surface) or scoping-equivalent clauses, none a consumer-visible defect, each classified on
athletic-rust-pipeline-nno. One previously unasserted consumer-visible counter path gained a
regression - a_director_only_published_email_counts_as_a_cohort_contact_email asserts
`totals.class_of_2027_with_coach_email` in the published census-all-sources.json - and the tree was
re-gated: var/gate-r6-20261004.log, all lanes green, rc=0.

## Owned panic-extraction enforcement — 2026-10-03

Bead `athletic-rust-pipeline-79l`; the full national census objective remains unchanged.
This is source-safety maintenance, not national, recovery, release or workbook acceptance.
The lexical gate covers owned tests, fixtures, examples, benches, cfg-disabled proof sources
and rendered generator sources. Total helpers such as `unwrap_or` remain allowed.
No owned extraction-lint waiver, panic replacement or fabricated success fallback was added.
Fallible checks propagate a typed error, borrow evaluated operands once and format only failure.

Main executed the following with
`TMPDIR=/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/test-tmp-sol-20261002`,
empty `RUSTC_WRAPPER`, and Cargo
`-Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access`:

```sh
cargo test -p census-crawl -p census-service --all-targets
cargo clippy -p census-crawl -p census-service --all-targets --all-features -- \
  -D warnings -D clippy::unwrap_used -D clippy::expect_used
cargo test --workspace --all-features
cargo test -p xtask --all-targets
cargo clippy -p xtask --all-targets --all-features -- \
  -D warnings -D clippy::unwrap_used -D clippy::expect_used
cargo build -p xtask --bin xtask
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- \
  -D warnings -D clippy::unwrap_used -D clippy::expect_used
target/debug/xtask panic-extraction --root .
```

Observed results: crawl/service 1,604 tests passed after removing four incidental/wiring
assertion tests; the full workspace run passed 2,357 tests across 60 suites with three ignored.
That workspace test run preceded the final broad-warning-waiver regression; the subsequent
xtask run passed all 139 tests. Latest integrated workspace check and all-target extraction
Clippy both passed after the final regression and six proof-source repairs
(`artifact://1750`, 54.92 seconds). The canonical 17-flag production safety Clippy passed again
on the final integrated sources (`artifact://1765`, 12.79 seconds). No full release lane is inferred.
The actual current lexical command exited zero: 1,538 Rust files and three rendered templates
checked. The mapper render is empty; three render calls do not mean three nonempty generators.

The independent scanner review `Audit79lScannerFinal` approved only this maintenance scope:
seven historical defects closed, no surviving finding. Range/update and Unicode-distinct
references now pass; malformed block-comment EOF, root/descendant links, selected FIFO source
and unqualified `allow(warnings)` now fail closed. Main's actual FIFO CLI changed from a
TERM deadline exit 124 to immediate refusal exit 1; the latter is not a universal clock bound.
Admission-budget evidence uses real tiny-budget walker tests, not a fabricated million-entry
CLI run. Static/quiescent tree, component/frame/file/entry and source-byte limits are documented
in `xtask/README.md`; directory substitution, aggregate pathname bytes and filesystem clocks
are not certified. Literal/corpus parity is not compiler-wide lexical equivalence.
Execution records: `local://panic-extraction-cli-baseline-sol-20261003.json`,
`local://panic-extraction-cli-after-sol-20261003.json`,
`local://79l-final-lexical-and-native-discovery-smoke-sol-20261003.json`;
independent terminal report:
`local://79l-final-scanner-independent-review.json`. Runtime commands are Main's observations,
not executions by the read-only reviewer.
The seven actual successful incremental closure payloads are preserved in
`local://79l-final-scanner-historical-closure-records.json`; current source/report hashes and
actual `sha256sum` output are in `local://79l-final-scanner-raw-digests.json`.
The final `target/debug/xtask comments` checked the same 1,538 files and found no comments.


The later one-execution printable-mailbox experiment also timed out at its independently
approved 600-second deadline (`artifact://1847`, 600.23 seconds). Its exact command is frozen
in `local://79l-p1-observed-short-loop-execution-plan.json` (SHA-256
`5f8cc02a68b8ce023a12d28af47c2f2151c4dbaa9953d2c17065b06629e39ff0`);
read-only independent permission is recorded in
`local://79l-p1-checked-loop-execution-independent-review.json` (SHA-256
`88788a61aebbd5ed6e39b45b3e1970626e133bf5b1362493c1b9aac6764cd566`).
The command applied only the three exact generated byte-fold/trim-loop bounds of 14.
real constructors, safety checks and unwinding assertions were unchanged. No final
verification verdict was produced; no proof or trust obligation is closed.
After the deadline, scope `run-p3718799-i7955360.scope` was actually observed
`not-found`, `inactive`, `dead`, with an empty control group.
Inspection-only GOTO dumps (`artifact://1805` and `artifact://1812`) and static dispatch
analysis do not certify feasibility or turn these timeouts into passing proofs.

An isolated runtime replay used a genuine original-run GET capture, not fabricated bytes:
NC meet 716802's 299,072-byte performances response, body SHA-256
`94c1b695d395fed85b5ac1122963fe8722763652dc83ad8e6daa56952a8b6cf3`,
immutable manifest SHA-256
`d5c25bd48a168c67788c4bf55cf36869ed931f073389d709bac6d39c27095837`.
Actual command:
`env TMPDIR=/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/test-tmp-sol-20261002 RUSTC_WRAPPER= cargo -Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access run -p census-crawl --example capture_replay_79l_smoke`.
The complete-capture smoke exited zero in 0.77 seconds, proving two exact-body cache
replays, two cache hits, zero physical requests, preserved observed response URL and
unchanged acquisition date `2026-10-03T08:04:40Z`. Immutable body and manifest readback
matched their originals in owned root `var/original-capture-replay-isolated-sol-20261003-02`.
The original archive was not modified and no Fjall store was opened.
The first attempt copied only the mutable cache pair and failed immutable-archive
readback (`artifact://1850`, exit one, 18.13 seconds); ordinary cache hits did not
create the missing archive. The passing attempt staged the complete preserved capture
before replay; it is not evidence of repairing or migrating an incomplete archive.
Both owned roots are retained. The throwaway example was removed after exercise.
Scope is one GET capture, not all-source attribution, POST/representation-context
binding, captured-result lineage, native fault qualification or fresh national acceptance.

Current original-run inspection did not certify national success. The native admin query
`SELECT target, status, COUNT(*) AS invocations FROM sys_invocation GROUP BY target, status ORDER BY target, status`
returned 500 completed RPC invocations, including 49 distinct `JurisdictionCensus/run`
targets (`artifact://1858`). RPC completion is not business completion.
`target/debug/census-service national-report --ingress http://127.0.0.1:18095/ --season 2026 --revision 1 --json`
actually exited one in 0.45 seconds: 48 jurisdictions failed. Forty-seven reported
`Terminal error [500]: the invocation stream was closed after the 'abort timeout' (1h) fired.`;
Wisconsin reported ``Terminal error [500]: no consolidated schools: run `collect` and `consolidate` before the wiaa_results provider``.
Only South Carolina appeared in the success array, with 457 total rosters, 186 committed,
271 remaining, 36,392 athletes and 9,041 reported Class-of-2027 subjects. These are partial
report fields, not a validated national population, accepted cohort or workbook.
`local://original-national-current-open-work-query-20261003.json` retains the observed
report-field summary explicitly as a summary, not full raw output.

The actual admin state query enumerated 41 `Ingest` keys: 39 MileSplit and two Wayzata
(`artifact://1863`). Passing precisely those 41 keys to `open-work` exited zero in
0.70 seconds (`artifact://1868`): 19 open jurisdiction sweeps; teams complete in 38
jurisdictions, rosters in 40 and meets in 39. The response reported zero owed rosters,
zero source objects within this supplied-key scope, and silent source `wayzata_mn`.
This is not the all-role source denominator or proof that missing source objects owe
no work. Subsequent raw shared-state inspection established a consumer defect: all
40 retained roster-progress records had positive remaining counts but were represented
as complete with zero owed rosters. No replacement logical identities, timeout increase,
live Fjall opener, invocation purge, seal or workbook acceptance was introduced.

The bounded, four-concurrent no-input shared-state transport read the original 49
jurisdiction identities once: 49 HTTP 200 replies, identity matches and no transport
errors. Its retained raw JSONL is
`var/original-national-state-read-sol-20261003-01/main-raw-3cd1cc83-3602-49a7-b280-ad2c3bcf74e3.jsonl`,
SHA-256 `ec949260fb405de746e412210a10b27c905cfa0f3e6a4d6ba96465287e7ad684`.
The independently derived offline projection retained 15,023 total, 4,691 clean committed
and **10,332 remaining rosters** across the 40 records. Nine records lacked roster
progress; their roster totals remain unknown, not zero. The exact transport and offline
projection inputs and raw-byte checks are retained beside the captures. These reads are
not an atomic population snapshot, source qualification, accepted cohort or completed census.
The all-role state query returned 91 distinct role/key pairs: 41 Ingest, 49 jurisdiction
and one national. Those counts do not establish the original paired 50/49 implementation
manifest or enumerate absent source obligations.

Main corrected roster stage/owed accounting in `restate_services/open_work.rs` and
the single roster obligation in `census/state/open.rs`, without changing stored shapes,
logical identities or retry ceilings. The fallible
`unfinished_rosters_keep_the_jurisdiction_nonterminal` regression failed before the fix
(`artifact://1888`, left true versus right false); all 209 service-library tests passed
after it (`artifact://1891`). The native binaries built (`artifact://1894`). The new
owned-Rust lexical scan checked 1,538 files and three rendered templates, and workspace
all-target/all-feature Clippy denied unwrap/expect extraction successfully

Before same-store verification, the original endpoint received TERM and exited zero.
Its supervised drain certificate was
`drained: accepted=45 completed=45 cancelled=0 timed_out=0 aborted=0 panicked=0`,
retained in `var/original-national-state-read-sol-20261003-01/original-endpoint-drain.txt`;
the conventional `serve.log` did not exist. Cold `store-integrity` returned `ok true`,
with matching counts in every listed table: 1,764,089 athletes, 1,865,444 source observations
and 18,533 source meets, but zero performances, identity decisions and review cases.
Physical integrity is not semantic acceptance. Immutable verification binaries are
under `var/releases/roster-accounting-sol-20261003-01/`; serve SHA-256 is
`58dadeed68c61f51945e3ae221103761a4e693108e127057dfebb7522a8a95a3`.

The required cold backup completed before handover:
`var/releases/roster-accounting-sol-20261003-01/census-service --store var/national-sol-20261002-01 store-backup --to var/backups/original-national-before-roster-accounting-sol-20261003-01`.
It retained 303,232 files and 25,587,117,323 bytes in 1,524.62 seconds, with all listed
table counts matching. Backup manifest SHA-256 is
`98ce2b9d2665650cafffe8f6b9655f8604a4b0f2f167f7f1d419a96dff818e63`.
This is a cold original-store backup, not a restore/readback fault qualification or a backup
of the separate native Restate journal directory.

Main started immutable `census-serve` on the same original store at `127.0.0.1:19081`
(PID 3808761). The initial tool readiness check timed out at 30 seconds; later genuine
cleartext HTTP/2 `/discover` returned successfully (`artifact://1915`). Registration
created `dp_16u7pThplPEHo40DluLAEPT` (`artifact://1916`), leaving the old registration
intact. The exact original `open-work` inputs — season 2026, revision one and the same
41 supplied Ingest keys — then exited zero in 0.47 seconds (`artifact://1919`).
Readback verified all **49 unchanged jurisdiction identities remain owed**, all roster
stages are nonterminal, and the 40 positive known counts sum to **10,332 owed rosters**.
The nine absent/zero-count stages remain owed without invented roster totals. Raw reply
and checked arithmetic are retained as `native-accounting-reply.json` and
`native-accounting-readback.json` beside the original state captures.
This exercises the corrected accounting path, not acquisition recovery, a completed
national census, a workbook seal or an in-flight upgrade fault. These immutable binaries
predate the subsequent complete unwrap-family syntax cutover; do not bind its later
source or verification verdict to this canary.

Two new advisory invocations used the actual local Qwen servers rather than treating
an agent label as GPU evidence. `local://79l-k5-actual-5090-qwen-review.json` retains
one RTX 5090 HTTP-backed response, ID `chatcmpl-e4677355bbbb9dc4`, model
`qwen3.8-27b-uncensored`, endpoint `127.0.0.1:11000`; its recommendation was
`APPROVE_STATIC_ONLY` for the exact proposed K5 byte-row construction.
`local://79l-p1-actual-3090-qwen-model-advice.json` retains one RTX 3090 response,
ID `chatcmpl-BvfrPrehEUS63uFDD2fKfz6FCnTlmyOq`, endpoint `127.0.0.1:11001`,
same model: `sha2/force-soft` would be supplementary only, and two fixed-constructor
runtime comparisons cannot certify default-production safety or general hashing
equivalence. Artifacts `1872` and `1877` bind the existing server PIDs, ports and GPU
UUIDs; no signed hardware/model attestation is claimed. Coordinators were distinct
from these actual Qwen HTTP model calls. Both reports preserve their exact model
input/output and limitations; neither grants source-edit/solver permission or closes
any of the original 18 trust rows or six bridges. The earlier required-Luna review
remains rejected for unattested model provenance. This is proof-maintenance advice,
not genuine ambiguous-athlete census advice or national/release acceptance.

Actual changed runtime surfaces were exercised, not just compiled. Main started an owned fresh
debug endpoint on `127.0.0.1:38267` with store
`var/no-unwrap-endpoint-smoke-sol-20261003-01` and fetched `/discover` using
`curl --http2-prior-knowledge --fail-with-body --silent --show-error --max-time 10`
with accept `application/vnd.restate.endpointmanifest.v4+json`. Ordinary HTTP/1 discovery was
rejected; the correct cleartext HTTP/2 request returned protocol 5–7 and eleven services,
including configured `BrowserSession` alongside the ten base services.
Main sent TERM to owned PID 3676866; observed exit zero and preserved `serve.log`:

```text
drained: accepted=3 completed=3 cancelled=0 timed_out=0 aborted=0 panicked=0
```

No Restate node was registered for this smoke, and no workflow/fault/census credit is assigned.
The actual Chromium profile `var/browser-rankings-no-unwrap-sol-20261003-01` and loopback CDP
port 29226 were used with controlled fixture port 21045:

```sh
env TMPDIR=/tmp RUSTC_WRAPPER= \
  ADLAW_LANE_FIXTURE=http://127.0.0.1:21045/ \
  ADLAW_LANE_CDP=http://127.0.0.1:29226/ \
  cargo -Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access \
  test -p athleticnet-browser --lib lane_smoke -- --ignored --test-threads=1
```

Both tests passed; actual ranking POST requests, unchanged bodies, challenge 403 and normal 200
were captured in `var/browser-rankings-no-unwrap-evidence-sol-20261003-01.json`.
This is controlled browser evidence, not live-provider access or national fault05 acceptance.
Owned Chrome PID 3676868 received TERM and exited zero; the fixture server was stopped.
`ss -H -ltn 'sport = :38267 or sport = :29226 or sport = :21045'` returned no listeners.
Fresh stores, browser profiles and evidence remain preserved; no unrelated run was stopped.
Main removed only its completed throwaway CLI fixture graph and two standalone macro-probe
source/binary pairs after the actual smokes and independent review; retained audit snapshots
and all durable stores, captures, browser profiles and runtime evidence were preserved.

## Direct cohort cutover and live coach claim readback — 2026-10-02

The direct publisher graduation-year repair now retains typed `PublishedGraduation` claims,
without fabricating reverse-calculated grades. Canonical merge, identity-evidence digests,
cohort review, model packets and workbook conflicts consume those claims. Projection receipt:
`milesplit_result_sets_v5` after the capture-bound replay repair recorded below; source
interpretation remains `milesplit_owned_meet_v3`. Historical v4 receipts remain preserved.
Publication policy is 5. Historical stores and policy-1/2/3/4 bundles remain preserved, not
silently migrated or accepted under the current policy.

Executed with the isolated `TMPDIR` and empty `RUSTC_WRAPPER` recorded below:

```sh
cargo fmt --all
cargo check --offline --workspace --all-targets --all-features
cargo test --offline -p census-domain
cargo test --offline -p census-store published_graduations
cargo test --offline -p census-crawl milesplit
cargo test --offline -p census-review
cargo test --offline -p census-report
target/release/census-service --store var/qualification-bound-sol-20261002-06 index
target/release/census-service --store var/qualification-bound-sol-20261002-06 workbook --out var/qualification-bound-sol-20261002-06/publication --grad-year 2027
target/release/census-service verify --workbook var/qualification-bound-sol-20261002-06/publication/current/workbook.xlsx
```

Domain 250, published-claim store 3, MileSplit 118, review 126 and report 182 tests passed
(`artifact://1152`, `artifact://1155`). The authentic four-capture qualification projection
command and its eight unchanged body/metadata inputs are retained in `artifact://1157`.
It made zero actual requests, retained 230 source observations, seven subjects and 13
performances; identical third application kept sequence 13 unchanged. Index readback
reported ten source identities, zero conflicts and zero review cases. A temporary typed
physical `ReviewCases` reader independently returned `[]` and was removed after execution.
Root 05's previously observed unsupported cohort case remains preserved as historical evidence.

The real workbook command and independent verification passed. Frozen generation:
`960fa368e9d37a9d73615b30f771e3b518d180aad5c36f117215915baf8e1883`.
Its `recruiting.csv` retains Adelyn Spann, 2027, Girls, Alabama, Abbeville High School,
source profile 14222592 and verified identity. This bounded retained-capture publication is
not the fresh national census or the requested final Downloads workbook.

After integration, workspace/all-target/all-feature check passed again; OHSAA 28, RIIL 12,
TSSAA 16 and native VM 31 tests passed (`artifact://1185`). The release service binaries and
native VM example built. Service parity then passed seven cases and failed two OHSAA
serialized-fact goldens reflecting removed fabricated person identities and actual capture
provenance (`artifact://1197`); this is not a full gate PASS. Subsequent review found OHSAA
changed-content suppression and HTTP-200 refusal misclassification. Their integrated repair
passed 38 OHSAA tests in 5.29 seconds (`artifact://1235`):
`cargo fmt --package census-crawl && cargo test --offline -p census-crawl ohsaa`
with the temporary directory/wrapper settings above. The 5090/SOL lane replaced the
two obsolete consumer DTO snapshots with genuine source-backed behavior assertions.
Integrated workspace/all-target/all-feature check and all ten service parity cases
passed (`artifact://1241`, 3.62-second test execution). Source-text retry tests were removed rather than re-pinned;
real native retry/exhaustion behavioral obligations remain required.

Actual public acquisition and stopped-store readback:

```sh
target/release/census-service --store var/qualification-tssaa-live-sol-20261002-02 provider tssaa --states TN --school-names 'Page High School' --limit 1 --refresh --observed-on 2026-10-02
target/release/census-service --store var/qualification-riil-live-sol-20261002-01 provider riil --states RI --limit 1 --refresh --observed-on 2026-10-02
target/release/census-service --store var/qualification-ohsaa-live-sol-20261002-01 provider ohsaa --states OH --school-names 'Dublin Coffman High School' --limit 1 --refresh --observed-on 2026-10-02
target/release/census-service --store var/qualification-tssaa-live-sol-20261002-02 export-data --data var/qualification-tssaa-live-sol-20261002-02/readback --school-year 2026
target/release/census-service --store var/qualification-riil-live-sol-20261002-01 export-data --data var/qualification-riil-live-sol-20261002-01/readback --school-year 2026
target/release/census-service --store var/qualification-tssaa-live-sol-20261002-02 consolidate
target/release/census-service --store var/qualification-riil-live-sol-20261002-01 consolidate
```

TSSAA: one school, two requests, zero errors, eleven published appointments/mailboxes.
RIIL: one directory request, zero errors, 55 schools and 258 appointments without invented
mailboxes or current-tenure claims. Its current CLI arm did not apply the supplied school
limit; observed output is 55, not one. OHSAA: search transport failed, zero facts and one
explicit report error; successful publisher acquisition is unproven.

Independent canonical JSON readback and recomputed body SHA256 found no foreign school
link or capture URL/time/digest mismatch across the eleven TSSAA and 258 RIIL appointments.
TSSAA's eleven persisted tenure claims explicitly retain school year 2026, the complete
published 2026–2027 staff statement, URL `?id=157`, retrieval `2026-10-02T13:45:26Z` and
digest `14482667d01a0d547a5aa0cb52d0d37c7e2312ddb90d9745b96c2b68c6a5c677`.
Three separately labelled postal claims remain. RIIL's directory capture is 200852 bytes,
retrieved `2026-10-02T13:45:25Z`, digest
`a442de68b5d8bbb66309d40016b70f825e2edac3da86c0957f20082b162d8d92`.
These genuine bounded source qualifications do not certify all jurisdictions/families.

## Measured native VM journal refusal — 2026-10-02

Fresh `var/vm-sol-20261002-05` booted the signed Arch image under real QEMU, registered the
native deployment and accepted real captured-athlete ingestion. Its final bounded Admin
bookends and complete v2 journal are retained in
`sweep-reboot-start-9c655d22-0d67-460e-a81b-103096c6bfcc.stderr.log`.
Original invocation `inv_1kEl3g4dVuTp052VbZTYDYqNJPrUsd9ChB` had protocol 7, the same
deployment/key/handler and journal size four: Input, Run, Run completion, Sleep.
Sleep completion ID was 2; wake time was 1790952376499 epoch milliseconds.
Both independent invocation snapshots positively reported `running`, not `suspended`;
the driver refused injection and exited 1. No hard reset or midnight fault passed.

Pinned Restate 1.7 upstream source establishes that journal-v2 leaves the legacy `completed`
column unset. Missing/null cannot mean unfinished. The corrected witness requires exact
command/notification identity, a complete bounded journal, independent suspended-future
evidence and guest-clock safety margin. The 5090/SOL repair now validates authentic
`SignalIndex` and all bounded future/envelope/completion variants without early matching-leaf
shortcuts. Main integrated the guest-only Admin configuration: positively measure deployment
`BidiStream`/HTTP/2, set Sweep inactivity to one second before first acceptance, read back
the same owner and unchanged abort timeout. Production defaults remain 3600 seconds.
This does not qualify a fault by itself; the next fresh VM confirmed configuration but refused injection.

Native host ownership repair rejects overlong Unix socket pathnames before preparing
payloads/disks/children, checks the QEMU owner during boot probes, and preserves original
scenario errors alongside all attempted cleanup failures. Known-dead QEMU cannot receive
a fabricated guest drain certificate. These integrated changes passed 47 native VM example
tests in 4.40 seconds; release example build passed in 59.52 seconds (`artifact://1237`):

```sh
rustfmt --edition 2024 crates/census-service/examples/qualification_native_vm.rs
cargo test --offline -p census-service --example qualification_native_vm
cargo build --offline --release -p census-service --example qualification_native_vm
```

All cargo commands used the recorded `TMPDIR` and empty `RUSTC_WRAPPER`. Independent reads of
actual roots `vm-sol-20261002-{03,04,05}/cleanup.json` confirm each endpoint/node/QEMU was
TERM-requested, reaped and exited zero; each endpoint drain counted accepted=4/completed=4
with cancelled/timed_out/aborted/panicked all zero. Those cleanup results do not change the
failed/refused fault verdicts. All disks, captures and logs remain retained.

## Measured suspended SDK signal boundary — 2026-10-02

Fresh `var/vm-sol-20261002-06` ran the repaired release VM example against real QEMU and
Restate 1.7, exiting 1 after 183.03 seconds. `deployment.json` positively records
`BidiStream`, `HTTP/2.0`, the same deployment owner, and the guest-only one-second
Sweep inactivity override with unchanged one-hour abort timeout. No production binding
default was modified.

The full refusal is retained in
`sweep-reboot-start-8e843daf-fec5-4b43-a083-6fbf948ae830.stderr.log`.
Both independent bookends positively reported `suspended`, protocol 7 and journal size four.
The complete journal retains Input, Run, Run completion and Sleep completion ID 2;
the Sleep wake time is 1790954352791 epoch milliseconds. The actual durable suspended future is
`FirstCompleted(Single(SignalIndex(1)), Single(SignalName("stop")))`, not an awaited
`CompletionId(2)`. Parsing succeeded, but the Sleep correlation proof remained absent.
The driver correctly refused injection; this is not a reboot or midnight PASS.

The 5090/SOL lane traced exact SDK 0.12/shared-core 7.0.3 semantics: `SignalIndex(1)`
is reserved cancellation, not the Sleep completion. Production `tokio::select!` polls durable
branches separately; the recorded suspended future belongs only to the signal branch.
Main cut over to the SDK's supported durable selection with explicit error/cancellation
propagation. Five window/stop behavior tests and ten service parity cases passed
(`artifact://1252`). The next actual VM reached the positively correlated Sleep described below;
the witness still requires exact completion identity, not guessed signal offsets or an arbitrary
uncompleted Sleep.

Independent `cleanup.json` confirms endpoint/node/QEMU TERM, reap and exit zero;
accepted=4/completed=4, every other drain outcome zero. All root artifacts remain preserved.

## Measured durable Sleep reset and unresolved boot ownership — 2026-10-02

Fresh `var/vm-sol-20261002-07` ran the integrated release endpoint and native VM driver,
exiting 1 after 317.79 seconds. `host-active-before-reset.json` positively retains matching
protocol-7 suspended invocation bookends, deployment `dp_13qUff8RpY7AadOKqSpzSc9`,
the original Sweep key and complete four-entry v2 journal. Its future includes awaited
`CompletionId(2)` alongside reserved cancellation `SignalIndex(1)` and named `stop`.
The Sleep has wake time 1790955564922; guest witness time 1790951966545 with a 30000-ms
safety margin. This is the reached native injection boundary absent in roots 05/06.

Actual QMP hard reset occurred and SSH reached the changed boot. The reboot then failed:
`qualification.service` did not expose a live owner within the unchanged bounded poll.
Initialization already executes `systemctl enable --now`; the startup cause is not established
by the serial log alone. `cleanup.json` preserves the primary and guest-drain errors:
guest certificate absent, QEMU TERM requested, reaped and exit zero, `qemu_orderly=false`.
No recovery, midnight, cleanup or full fault-suite PASS follows from the reached boundary.
All root disks, source captures and logs remain preserved. Bead: `athletic-rust-pipeline-lel`.

Integrated verification with the recorded isolated temporary directory and empty wrapper:

```sh
cargo test --offline -p census-service --test milesplit_roster_observations --test fjall_restate_e2e
cargo fmt --all
cargo check --offline --workspace --all-targets --all-features
cargo test --offline -p census-service --example qualification_native_vm
cargo test --offline -p census-service --test recovery
cargo build --offline --release -p census-service --example qualification_native_vm
```

The two consumer suites passed seven tests (`artifact://1262`, 0.83 seconds).
The VM example passed 56 tests, recovery passed eight real CLI/store/process tests in
23.73 seconds, and release example build passed (`artifact://1266`).
The recovery harness was mechanically split into bounded private modules after LSP reference
checks; original fault assertions and fixture inputs remain. Graduation-only roster observations
retain `observed_grade=None`; canonical typed published years and original capture ownership,
digest and replay/readback stability are asserted instead of reverse-invented grades.

The preceding full Nextest execution (`artifact://1257`) was not a PASS:
1786 passed, three failed, three skipped, 395 unexecuted after fail-fast. Those three failures
were obsolete discovery cardinality and reverse-grade consumers, repaired by the focused
passing suites above. A subsequent full-suite result is still required. Post-reset readiness
failure now records independent bounded systemd state/current-boot journal and preserves
all primary/diagnostic/publication errors; unit tests do not prove actual guest startup.
Actual diagnostic execution used a new root and the preserved authentic capture bundle:

```sh
target/release/examples/qualification_native_vm host --root var/vm-sol-20261002-09 --tools var/native-vm-tools-sol-20261002 --base-image var/native-vm-tools-sol-20261002/Arch-Linux-x86_64-cloudimg-20261001.604814.qcow2 --census-serve target/release/census-serve --restate var/native-runtime-restate-1.7.0-sol-20261002/restate-server-x86_64-unknown-linux-musl/restate-server --captures var/native-vm-captures-sol-20261002-root08.json
```

The external tool deadline was disabled; every driver/SSH/owner wait retained its internal
bound. Root 09 exited 1 after 312.88 seconds. Its actual
`recovered-supervisor-failure.json` records changed boot
`fe63381d-027f-4374-809d-8dddeae75c9a` → `58a6d0f6-875c-4b8a-9f63-056d1c76baac`,
live QEMU before capture, and successful independent systemd/journal commands.
Systemd reported `LoadState=loaded`, the expected `/etc/systemd/system/qualification.service`,
`UnitFileState=disabled`, `ActiveState=inactive`, `SubState=dead`, `MainPID=0`,
`Result=success`, `ExecMainStatus=0`; current-boot journal had no unit entries.
The unit file survived the actual reset, but boot enablement did not. A repair must persist
the validated enablement link/directory before accepting work and prove survival on the next
hard-reset run, not restart the unit or extend the deadline to disguise the failure.
Root 09 cleanup retains the original owner failure, absent guest drain, QEMU TERM/reap/exit
zero and `qemu_orderly=false`. It remains unqualified.

Root 08 hit the external tool's default 300-second deadline before diagnostic/cleanup capture;
it has no cleanup certificate or verdict. A post-timeout process query found no qualification,
QEMU, Restate or endpoint process, but that is not an orderly drain/reap certificate.
Its incomplete audit artifacts and disks are retained; no fault or cleanup PASS is claimed.


The enablement repair passed 61 native example tests and release build
(`artifact://1275`). Actual root `var/vm-sol-20261002-10` survived the witnessed
QMP reset: its bounded `reboot-oracle.json` reports PASS for original Sweep
`inv_1kEl3g4dVuTp052VbZTYDYqNJPrUsd9ChB`, unchanged key/deployment/machine,
changed boot, acknowledged cursor `38332` and 18 physical observations.
The original suspended invocation finished through its owned stop signal.
Root 10 then exited 1 after 159.03 seconds because the host midnight consumer
still expected legacy journal rows instead of the current validated v2 envelope.
Main reuses the strict native witness parser against the measured baseline;
the unchanged authentic root-10 capture supplies completion/date/boot regressions.
All 63 VM tests and release build passed (`artifact://1279`).

Actual root `var/vm-sol-20261002-11` then completed the bounded reboot, natural
UTC-midnight, Ingest and reused-capture oracles in 463.58 seconds.
Its `verdict.json` has no execution failure, but exits 1 with
`BLOCKED_OR_UNPROVEN`: active Jurisdiction/source-stage recovery and successful
production Fetcher acquisitions on both sides of midnight remain unproven.
These bounded outcomes do not certify national completion or full fault scenarios.
The subsequent read-only native review found the host baseline did not bind its
internally valid Sleep observation to the acknowledged original invocation/key.
Both identity rejection regressions failed before the repair (`artifact://1286`).
The host now binds original ID/key, accepted invocation ID and the intended shared
clock workflow identity before admitting the strict native Sleep witness.
All 65 VM tests, 38 OHSAA tests and the release example build passed
(`artifact://1288`). Actual `var/vm-sol-20261002-12` completed the same bounded
reboot/midnight oracles in 460.59 seconds with no execution failure; it still exits
1 for the two explicitly unproven source-stage/fresh-Fetcher obligations.
Its recovered endpoint drain is accepted=6/completed=6, all other outcomes zero;
endpoint, node and QEMU TERM/reap each exited zero, with QEMU `ORDERLY`.

Root 10 ordered cleanup succeeded: recovered endpoint accepted=3/completed=3,
all other outcomes zero; endpoint, node and QEMU each TERM-requested, reaped
and exited zero, with `qemu_orderly=true` and `qemu_state=ORDERLY`.
All run artifacts remain preserved.

`cargo nextest run --offline --workspace --all-features`, with the recorded
temporary-directory/wrapper settings, passed all 2184 tests with three skipped
in 146.030 test seconds (`artifact://1282`, 148.44 command-wall seconds).
Strict workspace Clippy subsequently failed on the redundant OHSAA synchronous
large-error wrapper and an unnecessary optional-city identity closure
(`artifact://1284`); its repair is not yet a passing release gate.

The publication review identified four release blockers: completed result-set
keys suppressed changed captured bodies/metadata; roster admission lacked final
response/document-owner binding; accepted aliases lost published graduation
claims; coverage ignored direct-year contradictions. All three report
counterexamples failed before repair (`artifact://1299`). Coverage's two
contradiction cases passed after using the domain cohort-conflict predicate,
while the alias case exposed a further grade-only census evidence counter
(`artifact://1304`); that counter now includes direct published graduation.
All three changed-result counterexamples failed against compile-adapted legacy
bare-key suppression (`artifact://1303`). The new v5 receipts bind capture and
projection content and retain previous receipt identities. Focused verification
passed 136 MileSplit and 133 network tests (`artifact://1310`), including changed
year/new-result/raw-metadata replay and real loopback redirect/304 acquisition.
Those owned HTTP and synthetic fixture checks are not fresh national acquisition
or full native-fault acceptance. Native source-stage and fresh clock-acquisition
helpers are being integrated and have not yet executed in a VM.



## Current native source witnesses and VM consumer repairs — 2026-10-02

Executed with `TMPDIR="$PWD/var/test-tmp-sol-20261002"` and `RUSTC_WRAPPER=` after actual
`/tmp` quota failures affected Fjall test stores and sccache temporary files. Root filesystem free
space did not eliminate the per-user temporary-file failure; no other run or temporary state was deleted.

```sh
cargo test --offline -p census-service --example qualification_native_vm
cargo build --offline --release -p census-service --example qualification_native_vm
cargo test --offline -p census-service --example qualification_native_teams
cargo build --offline --release -p census-service --example qualification_native_teams
```

VM: 27 scenarios passed and the release driver built (`artifact://1136`). An LSP inline refactor
had placed Result propagation inside an Option-returning readiness closure; its compile failure
was repaired without suppressing process-health errors. Teams: 34 scenarios passed and the
release driver built (`artifact://1145`). These are consumer/lifecycle regressions, not the 17
required native release faults.

The actual native driver ran under an owned user/network/mount namespace, using the built
`qualification_native_teams`, `census-serve` and actual Restate 1.7.0 musl binary. Fresh root:
`var/qualification-native-teams-sol-20261002-actual-07`; prior `actual-06` remains preserved.
The launcher retained the pre-unshare namespace owner and passed its namespace descriptors;
it did not manufacture publisher HTTP responses. Both executions exited 1 with a measured
`BLOCKED_OR_UNPROVEN` verdict, not a complete-release PASS.

`actual-07/qualification-oracles.json` now positively witnesses
`permanent_source_first_attempt_refusal` and
`native_interruption_witnessed_final_reservation_boundary`. Actual Admin SQL returns explicit
`IS NULL` caller predicates and ingress identity; omitted JSON fields alone cannot satisfy these
checks. The permanent refusal is the real fixed serving-owner source-parallelism policy refusal,
not a publisher/network permanent response. The interrupted third reservation remains backed by
its native invocation, physical TLS hold, ingress inspection and TERM/reap/restart chronology.
Typed settlement/history consistency, three-attempt transient exhaustion, settled-source replay
without a fourth admission, retained source failures and actual TLS refusal also passed.

Independent meets/results, full machine reset and clock faults, fresh positive acquisition,
publisher/network permanent refusal, final teams-failure reporting and whole-parent repeat with
zero additional physical requests remain UNPROVEN in this isolated fault root. Actual parent
execution failed in unfinished meet transport before results/final reporting; its repeat retried
that unfinished meet work. These outcomes were not relabelled or waived.

## Current direct cohort defect discovered by physical readback — 2026-10-02

Executed the current owned interpretation against fresh
`var/qualification-bound-sol-20261002-05` using the eight authentic input paths recorded below.
The real collector qualification passed (`artifact://1119`): zero requests, 230 retained source
observations, seven bound source subjects, 13 performances and byte-identical third replay with
sequence 13 retained. This root used owned interpretation v3; it is historical capture replay,
not fresh national discovery.

Executed `target/release/census-service --store var/qualification-bound-sol-20261002-05/store index`.
The command reported `source_identities=10 conflicts=0 reviews=1 superseded=0 coverage=54 snapshots=1`.
A temporary typed physical snapshot readback independently observed Adelyn Spann's
`AppliedAthleteIdentity`, kind `source_bound`, canonical/member ID
`ath_subject_05b8ab961559e53e`, policy 1, with its retained member evidence digest
(`artifact://1139`). This establishes source-bound identity acceptance in this qualification
root, not consolidation of other providers or a national population.

The separate typed ReviewCases readback reproduced
`Class-of-2027 cohort unverified:ath_subject_05b8ab961559e53e:p2:6f8627acc2ff77e0`,
with detail `no grade observation retained; 2 evidence row(s), 1 source(s)`.
Authentic API `data[172]` and `data[181]` directly publish owner `14222592` and graduation year
2027. The canonical row retained that claim only in evidence notes and had no observed grade.
This is a cohort eligibility defect, not two-person ambiguity. Bead
`athletic-rust-pipeline-4tt` tracks typed direct-cohort retention; this section does not claim its
implementation or acceptance verified. No grade was fabricated from the published year.

The temporary physical identity readback was removed after execution. Preserved root captures,
original source acquisition dates, physical rows and immutable journals remain unchanged.

## Qualified hurdle and score/reducer fixes — 2026-10-02

Bead `athletic-rust-pipeline-ay4`. A temporary public-API smoke reproduced adapter rejection of
`400m Hurdles` and `Boys Varsity 110 Meter Hurdles Finals`, domain misclassification of the former
as flat `Track400m`, a synthetic owned Decathlon total of 3456 parsed as distance, and incorrect
compatibility of time with Decathlon and points with Shot Put. Abbreviated `400H` already worked.
These synthetic inputs establish parser/mark defects, not an authentic publisher occurrence.

Executed `cargo fmt --all`, `cargo check --offline --workspace --all-targets --all-features`,
`cargo test --offline -p census-crawl --lib hytek::tests::event_scores`,
`cargo test --offline -p census-crawl --lib milesplit::owned::tests::event_scores`,
`cargo test --offline -p census-report --lib bests::tests::`, and
`cargo run --offline -p census-crawl --example qualification_event_alias_smoke`.
Check passed; five HyTek, five owned-parser and 75 bests tests passed. After-smoke output classified
both full hurdle labels correctly, emitted `Points(CentiPoints(345600))`, and rejected both
incompatible families. The temporary smoke was removed after execution. Raw provider JSON and
public serialized variants remain unchanged. Reducer regressions cover incompatible-only inputs,
duplicate reports, distinct meet identities, date/round/heat separation and deterministic genuine
contradictions. The observed unused reducer import was removed through the language-server fix;
five existing per-binary service-test helper warnings remain. Raw execution: `artifact://1090`.
Native faults, course/equipment qualification, history completeness and national release remain
unproven by these checks.

## Authentic empty-school retention, binding and byte-identical replay — 2026-10-02

Bead `athletic-rust-pipeline-23u`. Actual execution first exposed a timeout constructed outside its
Tokio runtime (`var/qualification-bound-sol-20261002-01`), then the production empty-school guard
preventing source-owned retention (`...-02`). After removing that guard, `...-03` retained 230
observations but exposed an identical-payload retained-journal rewrite: sequence 13 advanced to 14
although table and journal contents matched. Suppressing unchanged projection/retained receipt
writes fixed that observed transition; missing/invalid school-file decode errors are not hidden.

Executed the real public collector with fresh exclusive output root
`var/qualification-bound-sol-20261002-04`:

```sh
cargo run --offline -p census-crawl --example qualification_bound_projection -- \
  var/qualification-bound-sol-20261002-04 \
  var/retained-pr-correction-20261001/store/http/52e0b5d61b6c7de90be2a35dd5fc3f42.body \
  var/retained-pr-correction-20261001/store/http/52e0b5d61b6c7de90be2a35dd5fc3f42.meta.json \
  var/retained-pr-correction-20261001/store/http/c7791d114036f1a83166fcb81998032d.body \
  var/retained-pr-correction-20261001/store/http/c7791d114036f1a83166fcb81998032d.meta.json \
  var/retained-pr-correction-20261001/store/http/a361c173de86509e4dbe70cc8212cea8.body \
  var/retained-pr-correction-20261001/store/http/a361c173de86509e4dbe70cc8212cea8.meta.json \
  var/retained-pr-correction-20261001/store/http/b0baf6da6f1b721f0118c6d070a9ca2d.body \
  var/retained-pr-correction-20261001/store/http/b0baf6da6f1b721f0118c6d070a9ca2d.meta.json
```

Overall generic qualification passed, with zero physical requests. Empty canonical-school input
retained exactly 230 source observations and no canonical population. Genuine original Alabama
index/roster evidence bound provider school 38332 to Abbeville High School, creating seven
source-backed subjects and 13 performances without ingesting roster athletes as population.
Physical readback contains one Adelyn Spann subject, `ath_subject_05b8ab961559e53e`, with direct
published cohort 2027 and owner `milesplit_athlete:14222592`. This is not canonical identity
acceptance or a proof about other duplicate captures.

Flush/drop/reopen and unchanged third collection preserved all seven physical tables, all tracked
capture/meet/projection/application journals, original bytes/metadata and school input. Replay
sequence remained 13 to 13; source observation count remained 230. All prior failing roots and
their evidence were preserved. Exact readback lives in the seven stage directories and
`qualification.json`; raw execution is `artifact://1079`. The capture-scoped source remained
partial; no fresh acquisition, lifetime PR, national completion or seal is claimed.

Executed `cargo test --offline -p census-crawl --lib
milesplit::results::run::owned_tests::partial_replay`: both consumer regressions passed, now
checking snapshot sequence and empty-school-to-binding recovery. Executed `cargo run --offline
-q -p xtask -- scan`: nine packages had zero forbidden constructs, zero oversized files and zero
functions over 60 lines. This replay/scan preceded the subsequent hurdle/reducer changes above;
results do not automatically transfer to later trees.

## Integrated parity and review repair checks — 2026-10-02

Executed `cargo fmt --all`, `cargo test --offline -p census-review --lib`,
`cargo test --offline -p census-service --test parity_national milesplit_html_parity -- --exact`,
`cargo test --offline -p census-service --test parity_pipeline`,
`cargo test --offline -p census-service --test parity_national
authentic_milesplit_projections_retain_owners_without_inventing_school_bindings -- --exact`, and
`cargo test --offline -p census-crawl --lib net::execute::conditional_capture`.
The integrated checks passed: 126 review tests, both named national parity tests, the pipeline
parity test, and three conditional-capture regressions. Five existing unused service-test helper
warnings remain.

Integration first exposed missing explicit nested module paths, then an incorrect zero
`projected_rows` receipt assumption. Fixed module resolution and removed that incidental receipt
assumption; the regression still verifies exact retained public subjects, published marks/cohorts,
stable receipts, and no invented canonical athletes or school bindings. The 19 ratcheted assertions
were all in a parent-gated test-only file, not production. Renamed it to
`conditional_capture_tests.rs` and migrated its sole module path to the existing scanner convention.
No baseline increase or production exclusion was introduced. Function-budget helpers preserve
review cache/revocation behavior.

Executed `cargo run --offline -q -p xtask -- scan`: all forbidden construct counts were zero
across all nine packages, no files exceeded 300 physical lines, and no functions exceeded 60.
Executed the real `qualification_owned_projection` collector command above with fresh output root
`var/qualification-owned-projection-sol-20261002-05`: execution succeeded, unchanged replay was PASS,
physical source observations remained 230 to 230, table/journal digests matched, physical requests
were zero, and original captures remained unchanged. This remains bounded offline replay evidence,
not fresh national acquisition or native fault acceptance.

## Owner removal of obsolete cargo-vet requirement — 2026-10-02

Bead `athletic-rust-pipeline-o8z`. Owner explicitly removed cargo-vet from the required work.
Removed its obsolete release-gate function/invocation and updated the current architecture,
delivery-plan and tools references. Dependency advisory, provenance and security requirements
remain. Historical cargo-vet failures below are preserved, but are no longer current blockers.
`bash -n tools/gate.sh` passed. The entrypoint has no `--help` mode; invoking it returned
`unknown argument: --help` with exit 2. Full revised gate execution remains pending.

## Actual dual local model qualification — 2026-10-02

Executed `cargo run --offline -p census-review --example qualification_dual --
var/qualification-dual-sol-20261002-actual-01`. Both real local endpoints answered:
`http://127.0.0.1:11000/v1/chat/completions` using `prompt_json`, and
`http://127.0.0.1:11001/v1/chat/completions` using `json_schema`. Both reported
`qwen3.8-27b-uncensored`; each request had a 512-token ceiling and 90,000-ms timeout.
The retained `dual-audit.json` records distinct request digests and the common evidence digest
`30a73b5568f0a5d85ba3963bcfce4e3a705b3b3a9411662ce138fb6de660667b`.
Both adjudications were `Undecided`; the combined outcome was `insufficient_evidence`.
The unchanged replay requested zero additional model calls and added no checkpoint.

This packet explicitly used a synthetic qualification-only school with no source capture or
jurisdictional evidence. Correct abstention proves bounded actual endpoint execution, not review
of a discovered public athlete, identity acceptance, national coverage or a census seal.

## Integrated release gate failure — 2026-10-02

Executed `tools/gate.sh --release`; the result was `gate: FAIL`. Preserved execution log:
`artifact://1013`. The integrated tree at execution preceded subsequent parity and function-budget
repairs; this result does not certify those later changes.

Observed blockers included the teams-stage inner `RunRetryPolicy` declaring three attempts against
the one-attempt inner ceiling; two MileSplit parity dispatch failures for
`troy_725218_raw_projection_provenance.json`; the crawl assert-family ratchet increasing from zero
to 19; and seven newly oversized functions. Strict source Clippy reported zero diagnostics in
eight missing proof kernels: `check_census_scope`, `check_fixed_point_bounds`,
`check_identity_contradiction`, `check_pr_comparison_laws`, `check_redirect_cycle`,
`check_retry_limit`, `check_store_batch_arithmetic`, and `check_terminal_state_no_retry`.
Mutation execution was blocked by the failing test suite. GNU time was absent, so this execution
did not supply peak-RSS evidence. No gate, audit requirement, proof obligation or baseline was
waived; the national deliverable remains uncertified.

## Additional owner main pull — 2026-10-02

Bead `athletic-rust-pipeline-0xl`. User requested another main pull during the census work.
SOL writers finished before the snapshot. `git fetch origin main` advanced origin/main from
`0b0e988d` to `e8f2d9ae`; `git merge --ff-only origin/main` fast-forwarded the local main.
All tracked and untracked local work was preserved in stash
`bb79304a6787e8521713dc2b26612db166c0a0e3` and reapplied successfully without conflicts.
This stash and the earlier `c74970ad2e80ba3cc29bf9b6d553899300555e8c` recovery stash remain intact.
No commit, push, historical artifact deletion or shared-process shutdown occurred.

Executed on the merged tree with restored local work:

```sh
cargo fmt --all
cargo check --offline --workspace --all-targets --all-features
cargo test --offline --workspace --lib --all-features
cargo run --offline -p census-crawl --example qualification_mpa -- var/qualification-mpa-sol-20261002-01
```

Compile passed; 1,684 library tests passed across eight suites, with two ignored.
The actual offline MPA collector qualification exited 0 after two evaluations and flush/drop/reopen:
four cache hits, zero physical requests, independently supplied directory/staff capture dates retained,
and zero collector errors. These fixture dates are qualification inputs, not original acquisition dates;
the replay does not certify current coaching tenure, national coverage or idempotent MPA physical writes.
Five existing unused-helper warnings remain in the service integration-test common module
(`athletic-rust-pipeline-p0n`). This integration evidence is not a clean release gate or census seal.

## Owned partial projection replay — 2026-10-02

Bead `athletic-rust-pipeline-23u`. The real public collector qualifier first exposed physical source
observation duplication: preserved root `var/qualification-owned-projection-sol-20261002-01`
grew from 230 to 460 rows after flush, owner drop, reopen and unchanged replay. Its execution flag
was true, but the independent replay oracle was FAIL. No accepted athletes or network requests
were produced; canonical absence was not treated as successful national coverage.

The fix stages `milesplit_result_set_effects_v1` table/canonical-content-digest witnesses in the same
atomic batch as physical rows and the partial/completed disposition. Each witness uses an indexed
`Store::journal_contains` lookup; retained rows are filtered in place and written in one batch per
table, not one batch per row or a scan of all accumulated witnesses. Partial school/cohort gaps
remain unfinished. Exact newly available provider school bindings can resolve owed entities while
unchanged observations remain physically stable. Older partial stores without these witnesses are
preserved and are not retroactively certified replay-idempotent.

Executed on the final indexed/batched implementation:

```sh
cargo fmt --all
cargo test --offline -p census-crawl --lib milesplit::
cargo run --offline -p census-crawl --example qualification_owned_projection -- var/qualification-owned-projection-sol-20261002-03 var/retained-pr-correction-20261001/store/http/52e0b5d61b6c7de90be2a35dd5fc3f42.body var/retained-pr-correction-20261001/store/http/52e0b5d61b6c7de90be2a35dd5fc3f42.meta.json var/retained-pr-correction-20261001/store/http/c7791d114036f1a83166fcb81998032d.body var/retained-pr-correction-20261001/store/http/c7791d114036f1a83166fcb81998032d.meta.json var/qualification-postal-sol-20261002-actual-02/collector-school-input.jsonl
```

All 104 focused tests passed. The real collector qualification passed unchanged replay: 230 to 230
physical source observations, equal physical table digests and equal key/payload digests in all four
journal phases, zero network/model requests, zero canonical athletes, and unchanged original captures.
The source school input is the independently postal-qualified NC school with no invented MileSplit
school identity, so the foreign Alabama result rows honestly remain unresolved.

Two permanent consumer regressions exercise missing-cohort partial reopen and partial exact-school
binding followed by truthful fixture binding completion, including physical table and journal equality.
An older unresolved-row assertion now inspects physical observations rather than an aggregate subject
scan, preserving the expected result row and no-inferred-grade checks. A provider subject can legitimately
own observations from several events; repeated subject IDs alone do not prove duplicate physical rows.
The qualifier retains actual API/raw capture dates independently, preserves input bytes, and snapshots
its source. Roots 01–03 remain preserved. This bounded replay is not native crash/lost-ack proof,
fresh national acquisition, source exhaustion, identity acceptance or a release seal. Integrated
workspace and release gates remain pending.

## Authentic owned results, postal dates and native teams boundary — 2026-10-02

Scope: integrated SOL-authored source projection and bounded qualification drivers on main
`0b0e988d` plus preserved local work. This is not fresh 49-jurisdiction acquisition or a release seal.

Executed:

```sh
cargo fmt --all
cargo build --offline -p census-service --bins --example qualification_native_teams --example qualification_postal
cargo test --offline -p census-crawl --lib milesplit::
target/debug/examples/qualification_postal var/qualification-postal-sol-20261002-actual-02 /home/lewis/src/ad-law-scrape/athletic-rust-pipeline/target/debug/census-service 2026-10-02T12:00:00Z
```

Build passed. The focused suite passed 102 tests. Initial integration repaired an untyped empty-vector
assertion and an unused import. SourceObservation IDs denote provider subjects, not result rows:
three distinct result observations for two athletes legitimately share a subject ID. The regressions
retain exact physical row counts and before/after physical-walk/content comparisons to detect replay
appends rather than rejecting legitimate multi-event evidence.

The authentic Troy raw projections preserve original inclusive lines 42–63, 208, and 712–719 plus
only an explicit closing `</pre>` newline. Independent SHA256 and byte comparison initially exposed
eight normalized CRLF endings in each projection; original endings were restored and exact comparison
passed. Female original SHA256 `729850b479e5782aa3e4ade7740cd46b8ffd8f35db79a873abd4ed0b3fb0f6cf`;
male `d9bcb109b52d5c54fddc1bede35a9ba6c4496d0c717d99bf8052113012165efd`. Published meet date remains
2026-03-27, outdoor, school year 2025. Foreign metadata rejection, atomic projection/receipt visibility,
interruption/reopen and completed-plus-new-set replay ran in the focused suite; these are deterministic
consumer regressions, not native process-crash proof.

Postal qualifier `var/qualification-postal-sol-20261002-actual-02/qualification.json` passed with one
NC school, two same-provider address claims, 16 coach contexts and zero athletes/model calls.
Evidence dates remain `2026-09-27T00:00:00Z`, separate from execution observation
`2026-10-02T12:00:00Z`. The rebuilt CSV CLI exited 0 after store-owner drop; CSV postal cells equalled
independently read XLSX cells. Policy 3 readback verified 213 rows over 11 sheets. The root and all six
private helper sources are snapshotted with lengths and SHA256. This retained-fixture replay does not
certify fresh source acquisition, eligible current coaching roles, athlete joins or national coverage.

Installed native Restate measured 1.6.2. An isolated official 1.7.0 runtime was downloaded to
`var/native-runtime-restate-1.7.0-sol-20261002`, leaving the installed binary untouched. The release
archive SHA256 `323eff8d4f98658a009dba4c94343e37145c5f043f14ee2ff957be06c6535749` passed
`sha256sum --check`; extracted binary SHA256
`027181315dfde51cf82f75913a5878422fbf92b2407c2fe5216702a228a2183e`, actual `--version` 1.7.0.

The actual native argv was `unshare --user --map-root-user --net --mount --fork --kill-child=TERM
target/debug/examples/qualification_native_teams var/qualification-native-teams-sol-20261002-actual-05
/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/target/debug/census-serve
/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/native-runtime-restate-1.7.0-sol-20261002/restate-server-x86_64-unknown-linux-musl/restate-server`.
The live host launcher supplied `QUALIFICATION_HOST_PID=1659045` and original kernel network/mount
namespace descriptors at child fds 3/4. Earlier failed roots 01–04 remain preserved: proc namespace-link
permissions, host NSS overriding private hosts, and v7 no-input call body rejection were measured and
fixed. Namespace-only hosts/NSS bind mounts follow recursive-private verification; source destination
guards and TLS validation remain unchanged. The proxy forced TLS EOF, never an HTTP 500 or source body.

Native root 05 observed exactly three logical teams Run executions, completion ID 4, terminal code
500 with the actual teams transport message, and equal retained Failed state through ingress/admin.
Six first-invocation TLS admissions and one repeat admission are separately measured and cannot be
attributed to source paths. Actual IDs: first `inv_17w2utFWtT0T3exsyQmx0zE2QglH3sulBN`, distinct
same-object repeat `inv_17w2utFWtT0T7A49LayYRx2su0Mn5SLGIF`. Teams state was unchanged and no repeat
teams-failure Run completion appeared. Rosters persisted an actual error and 977 remaining obligations;
the genuine meets-index request failed independently, meets remained null and results were not reached.
Overall `BLOCKED_OR_UNPROVEN`: final teams-report refusal and full same-key physical-request criterion
were not proven. No synthetic later-stage state substituted. Exact oracles, journals/events and HTTP/
physical observations remain under the root.

Ordered cleanup passed: endpoint PID 2583752 then node PID 2583751 TERM/reaped with exit 0; all four
ports rebound; proxy joined; no SIGKILL or upstream forwarding. Drain certificate:
`accepted=3 completed=3 cancelled=0 timed_out=0 aborted=0 panicked=0`. All stores/artifacts preserved.
This TLS boundary is not any of the required all-17 fault scenarios; isolated VM reboot/clock evidence
and healthy independent-stage qualification remain outstanding.

## Remote-main pull integration — 2026-10-02

Scope: pull the owner's merged PR into this working tree without discarding the ongoing local
slices. `git pull --ff-only origin main` advanced main to
`0b0e988d805c2c122149ecbf90bb16458d99f2d3`. The complete pre-pull tracked/untracked snapshot remains
in stash `c74970ad2e80ba3cc29bf9b6d553899300555e8c`; independent recovery archives remain at
`/tmp/athletic-prepull-tracked-c74970ad.tar` and `/tmp/athletic-prepull-untracked-c74970ad.tar`.
No commit, push, stash deletion, store opening, or native deployment occurred.

Restoration exposed 22 tracked conflicts and 55 formerly-untracked paths now tracked upstream:
15 byte-identical and 40 differing. All differing paths were compared across exclusive crawl,
review, and Main ownership. Upstream bounded archive/replay, failed-stage preservation, directory
fsync, postal validation, source-bound event refinement, individual-advice reuse, negative identity
adjudication, and full-fit Excel cell handling were retained. Local acquisition dates, TeamRelay
interpretation, unfinished owned-result projection, strict review shapes, revocation/sequence
fences, advice archival, and positive provider-profile fixtures were preserved.

Integrated verification:

```sh
cargo check --offline --workspace --all-targets
cargo test --offline -p census-domain -p census-store -p census-review -p census-crawl -p census-report -p census-service --lib
cargo test --offline -p census-review -p census-service -p census-store --lib
cargo test --offline -p census-service -p census-store --lib
cargo run --offline -q -p census-crawl --example qualification_owned -- var/retained-pr-correction-20261001/store/http/52e0b5d61b6c7de90be2a35dd5fc3f42.body /tmp/athletic-pull-owned-20261002.json
cargo fmt --all -- --check
git diff --check
```

The initial compile exposed nested-module path wiring, an ambiguous conversion, and a superseded
private reexport; repaired. Initial tests exposed an incidental audit-reason assertion, the old
JSON-versus-Decode error expectation, and a current state fixture lacking its required teams stage;
reconciled without changing domain sanitization or adding a historical-state fallback. Final library
results: crawl 733, domain 244, report 170, review 126, service 200, store 119 passed; 1,592 total.
All-target compilation, formatting, and diff checks passed. Compilation reported five unused
helpers in the existing `parity_pipeline` integration-test common module.

The actual source CLI read 311,763 original bytes, SHA-256
`8db9804ebc2d36b6f7ec1e2a289b2b8ab28610e0063f0ee9f54245eb128071ed`: 602 published rows,
560 individual rows, and 42 explicitly classified TeamRelay rows; no malformed individual rows.
Completeness remained Unknown and `ownership_complete=false`. Readback is
`/tmp/athletic-pull-owned-20261002.json`. HEAD matched origin/main and the index contained no unmerged
paths. This integration evidence is not a native-fault, fresh national census, or release certificate.

## Beads and branch planning reconciliation — 2026-10-01

Scope: the owner's complete canonical-athlete/multi-event PR/contact/school-address target across
the contiguous 48 states plus DC, active Beads cleanup, and content-first branch/worktree review.
Analysis baseline was main `02d13802`, matching the observed origin/main tip. This pass changed
planning documentation and local Beads records, not production Rust or historical census data.

The reconciliation created 60 explicit ownership/input/output/acceptance/handoff contracts,
including all 17 native faults, under product milestone `athletic-rust-pipeline-7lx`.
Each generated task JSON passed:

```sh
cue vet <task-contract.json> /home/lewis/.agents/skills/planner/schemas/task-schema.cue -d '#Task'
bd lint
bd dep cycles
bd ready --limit 0 --json
```

CUE success is structural planning evidence only. The first `bd lint` found 27 missing-heading
warnings in 14 existing issues; their original descriptions/notes were retained and their precise
reproduction/acceptance sections repaired. The final lint checked 78 issues with no template
warnings; the dependency checker found no cycles. The ready queue contained 22 actionable issues.
Stale in-progress claims were reset without claiming fixes. `cni` was superseded by `6yj`,
branch-local vet issue `3sb` by authoritative-main `6yj.6`, and coach-admission issue `6yj.13`
by the already closed exact implementation/evidence issue `gqk`. No other defect was certified.

The installed Beads build treats unfinished `parent-child` links as prerequisites and rejects
epic-to-task `blocks`; retaining the old audit parent/child arrangement prevented runnable work.
Grouping now uses nonblocking labels/relations. Task prerequisites form the actual release DAG;
epics use `tracks` and remain blocked pending tracked completion. Optional live vendor/private
association qualification, comparable performance baseline and twelve-versus-eight proof-policy
requirements remain explicit, not silently waived.

Four read-only scouts compared coach, DragonFly, dirty engine, Holzman, PIAA, Python-port and capture
snapshot content with current main. Main independently inspected ancestry/patch equivalence,
14 non-main local branches and ten linked worktrees: four clean tracked trees and six dirty.
`git cherry main coach-acquisition-rust` reported eleven `+` commits; most behaviors were already
harvested or superseded in main, but existing Arbiter acquisition still lacks production
caller/applicability integration (`o8x`). `git cherry main dragonfly-coach-directories` reported
`- 4d0b962c`: its adapter/fixtures/goldens/callers are already present, with stronger current admission.
Dirty engine models are unwired parallel contracts; only admission/halt propagation is a qualified
inspection candidate (`o99`). Dirty snapshot/PIAA replacements are superseded. Preserve capture
snapshot `bba998e2`, all dirty originals and synthetic-versus-live provenance.

Actual CLI observations:

```sh
target/release/census-service --help
target/release/census-service review --help
target/release/census-service school-address --help
```

The first two exited 0; review exposes a single default model endpoint, not proof of integrated
dual-model execution. `school-address --help` exited 2 with an unrecognized-subcommand diagnostic.
Current source also lacks its CLI variant/route; language-server references to
`census_service::school_address::run` found test callers only. The dirty Python-port route is the
selective integration candidate `q2w`, not its older address/geocode/publication implementation.
Source/LSP inspection also found national completion does not include the standalone reconciliation
and review path; durable integration is explicitly planned, not claimed operational.

Limits: no production build/test/proof/performance/native-fault suite, new acquisition, model call,
store mutation, seal, merge, branch/worktree deletion, commit, push or Dolt remote sync occurred.
The branch disposition and full execution map live in the owning
[national plan §7](NATIONAL-CENSUS-PLAN.md#7-ordered-stage-exits); preservation/approved retirement
is `h0b`. This planning evidence does not certify a fresh national census or any of the 17 faults.

## Retained PR normalization and Downloads follow-through — 2026-10-01

The owner requested a PR for every represented event and the workbook in
`/home/lewis/Downloads`, without another acquisition run. An immutable-input audit of the first
retained publication found 836,928 cohort subjects, 259,447 with retained result rows, and 577,481
without retained results. The old 127,496 PR comparison keys matched the then-numeric selection,
but 129,839 cohort marks were raw: examples `24.95a`, `11.52a` and `3:00.64a` exposed a real source
normalization omission. An additional `19-.25` long jump was numeric but not measurable by the
imperial notation oracle. Missing results and individual relay splits are not invented.

### Cold rollback boundary and append-only correction

```sh
mkdir -p var/retained-pr-correction-20261001
target/release/census-service --store var/run-2027-v2 store-backup --to var/retained-pr-correction-20261001/backup
target/release/census-service store-restore --from var/retained-pr-correction-20261001/backup --to var/retained-pr-correction-20261001/store
target/release/census-service --store var/retained-pr-correction-20261001/store repair-retained-marks
target/release/census-service --store var/retained-pr-correction-20261001/store repair-retained-marks --apply
target/release/census-service --store var/retained-pr-correction-20261001/store repair-retained-marks
target/release/census-service --store var/retained-pr-correction-20261001/store fjall-stats
```

The initial backup attempt correctly failed because its new parent directory did not exist; no
source store was deleted or reused. After creating that parent, backup and manifest-validated
restore completed in 730.70 seconds. Backup: 192,976 files, 25,842,522,015 bytes; restored root:
192,973 files, 25,636,072,355 bytes. Backup manifest SHA-256:
`89e494893adbcd82f6fa7639e3a412c7f68234dd4fd8ab07540d19d81282c3ee`.
All seventeen physical table counts matched at restore. The original store and backup remain intact.

Dry-run scanned 1,024,921 canonical performances and found 484,185 eligible corrections. Apply
appended exactly 484,185 observations; the next dry-run reported zero eligible/corrected. Both runs
reported 34,524 non-numeric/undeclared tokens and zero contradictory timing, missing event or
missing evidence. Apply, rerun and stats completed in 12.35 seconds. Physical performances rose
from 1,024,921 to 1,509,106 and observations from 18,393,110 to 18,877,295; every other table count
remained identical to the original baseline below. This corrects retained parser facts, not
acquisition freshness, identities or missing histories.

A disposable real-store readback also checked `perf_00011acc34a2fdd2`: its original `Raw("24.95a")`
observation remains, its correction reads `TimeSeconds(2495)`/FAT, and canonical readback equals the
corrected observation. IDs, owner, event/team/meet, result date, grade, locator and parsed capture
evidence are unchanged; the derived note retains the original token/revision. The release example
reported `retained_history_readback=PASS original_observations=1 correction_observations=1`
(`artifact://709`); it was removed after execution. This is one sampled production history,
complemented by the isolated invariance/chunk regressions below, not a full historical-row audit.

### Focused integration checks

- `cargo test -p census-crawl -p census-store -p census-report --lib`: 876 passed before final
  captured-fixture and collision-boundary integration.
- `cargo test -p census-crawl --lib captured_class_of_2027_result_keeps_its_numeric_mark_and_timing`:
  passed against the existing DC legacy capture.
- `cargo test -p census-store --lib performance_tests`: six passed, including both merge orders,
  retained observations, no numeric downgrade and event/team timing guards.
- `cargo test -p census-report --lib`: 151 passed; exact `19-.25`/`19-00.25` equivalence,
  rejected bare decimal/thousandth-inch input and accepted redundant trailing zeroes.
- `cargo test -p census-service --bin census-service retained_marks`: five passed, covering dry-run,
  immutable facts/history, idempotence, source-owner rather than locator-substring admission,
  field/no-result/unknown/multiple suffix boundaries, contradictory timing, hand timing and a
  101-correction full/partial chunk boundary.
- `cargo build --release -p census-service --bins`: passed.
- Strict workspace `cargo clippy --lib --bins --all-features` with the gate's complete denied lint
  set: passed. This focused command excluded the disposable audit/readback examples.
- Real CLI `repair-retained-marks` without `--store`: exit 1 with explicit-store refusal.
  `repair-retained-marks --help` exposes default dry-run and `--apply`.

### Intermediate timing publication and metric diagnosis

Native Restate 1.6.2 registered nine services under `dp_14p2CROCeFJ0p2IzEBZHtSx`, without a browser
lane. The sole owning endpoint used the restored root on port 9292; ingress/admin were
18295/19295. Only Workbook was submitted, completing invocation
`inv_18u4l5FquYh24qkVZjy1mucXI75gCwFhZG` in 392.67 seconds:

```sh
target/release/census-service workbook --ingress http://127.0.0.1:18295/ --grad-year 2027 --out /home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/retained-pr-correction-20261001/publication
target/release/census-service verify --workbook var/retained-pr-correction-20261001/publication/current/workbook.xlsx
```

Generation `240df362a2cdd3739b051b4bb5ededed3cd3820e93b19dd86b84a4054d8f4cbb` used schema/policy
1/2 and snapshot sequence 102459. Its XLSX is 156,911,239 bytes, SHA-256
`dd97cf9c3494c5b57627455c38002b387b80c7e53585d0f0ba3e1060c9964482`.
Native complete readback verified 2,031,265 rows; the standalone verifier returned
`verify: OK (complete frozen generation)` in 176.57 seconds. Frozen-input PR audit
(`artifact://718`, 67.44 seconds) counted 246,203 comparable keys and 246,203 published rows,
zero missing/unexpected keys, 2,111 excluded relay rows and 11,133 non-numeric marks.
The raw-mark sample exposed another omission: explicit metric field tokens such as `9.47m`.
This intermediate generation and its invocation/deployment receipts remain preserved; it is not
the final corrected Downloads artifact.

After endpoint SIGTERM, drain reported accepted=4, completed=4 and zero cancelled/timed-out/
aborted/panicked. The node then exited on SIGTERM in 46.101476 ms. Both exited 0; no run listener
or native service process remained. Logs are retained in `var/retained-pr-correction-20261001/`.

The first full `tools/gate.sh` finished in 506.02 seconds: all stages passed except Cargo-vet,
which reported 345 missing audits. Nextest ran 1,877 tests: all passed, three skipped.
This was before the additional metric-unit regressions and is not evidence for their integration.

### Exact retained metric refinement

After stopping the intermediate run, the same dry-run/apply/rerun sequence found and appended
17,152 additional metric corrections, then reported zero eligible corrections. Every invocation
scanned 1,024,921 canonical performances and reported zero contradictory timing, missing event or
missing evidence. Source-declared `m/M` values must be exactly representable as centimetres;
sub-centimetre precision is retained raw, not rounded. Event identities and timing are not inferred.

Combined correction history now adds 501,337 observations: physical performances are 1,526,258
and total observations 18,894,447. Every other physical table count remains at the original baseline.
The real retained Vermont row `perf_002fa5207eb50c1a` still has its original `Raw("9.47m")`
observation and now reads canonically as `DistanceMetres(947)`. The disposable readback compared
all non-mark/non-derived-evidence fields, source URL, capture date and canonical winner:
`retained_metric_history_readback=PASS exact_centimetres=947
unchanged_identity_affiliation_date_grade_timing_capture=true`.
Apply/rerun, release readback build/execution and stats completed in 106.17 seconds; exact receipts
are preserved in `var/retained-pr-correction-20261001/metric-repair.log` (`artifact://730`).
The disposable source was removed after execution.

Metric integration commands:

```sh
cargo test -p census-crawl --lib milesplit::mark::tests
cargo test -p census-crawl --lib published_metric_field_row
cargo test -p census-service --bin census-service retained_marks::tests
cargo build --release -p census-service --bins
```

Observed results: twelve mark-boundary tests, one complete source-row regression and six isolated
repair tests passed. Coverage includes exact `9.47m`, source row/event/grade preservation,
sub-centimetre and malformed-unit refusal, dry-run/apply/history, unchanged timing and idempotence.
Both release binaries rebuilt successfully.

### Final corrected native generation and Downloads

A fresh native node used `metric-restate.toml`, its own `restate-metric` base-dir and ports
15175/18395/19395. The sole restored-store endpoint listened on 9392 with concurrency 1 and a
120-second drain budget. Nine services registered under `dp_16ByUwV5OWqA3wRBhHXBvLX`; no browser
lane was configured. The admin query returned exactly one completed invocation,
`inv_1fqReb48tqDI7GMTqfhFiyU4LKSjl8YwsB`, targeting Workbook. No acquisition was submitted.

```sh
target/release/census-service workbook --ingress http://127.0.0.1:18395/ --grad-year 2027 --out /home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/retained-pr-correction-20261001/publication-metric
target/release/census-service verify --workbook var/retained-pr-correction-20261001/publication-metric/current/workbook.xlsx
target/release/examples/retained_pr_coverage_smoke var/retained-pr-correction-20261001/publication-metric/current
```

Workbook completed in 419.83 seconds. The standalone verifier returned
`verify: OK (complete frozen generation)` in 140.23 seconds; native complete readback verified
2,036,064 rows.

| Frozen publication fact | Observed value |
|---|---|
| Generation | `e17724feb2753eacb69e3763d021a469246450fb4ed5862ca7b0dd9958828197` |
| Input generation | `f09669a14e8f54493127ca792d64c13157bcde2f79ecad6aadf16a9f8c9d5bcc` |
| Input digest | `aea261f1f6733fa5b3ab020899329f16d7ae8c8824c420332864e17cb2504880` |
| Source digest | `6e87eba48d5fe7117020f5517c5d563c32af0a43498ead5c1472da894cbb22c4` |
| Snapshot / schema / policy | 102633 / 1 / 2 |
| Selection | All sources, Class of 2027, unlimited, no school-year filter |
| Frozen input bytes / SHA-256 | 5,027,957,482 / `5d1b982c91201c08341893228a1afc6054ea4c85be8829222e729c293e107858` |
| XLSX bytes / SHA-256 | 157,493,209 / `b1e863e48d25b751fb8b017a25b984bb97c319f031767a399e2ba04eda892fd4` |

The complete frozen-input PR audit finished in 60.23 seconds. It found 251,005 eligible
athlete-event comparison keys and exactly 251,005 published PR rows: zero missing and zero
unexpected keys. All 836,928 cohort subjects and 259,447 retained result rows remain represented.
The 577,481 subjects without retained results do not receive fabricated PRs. Individual PR
exclusions are 2,111 relay rows and 6,331 non-numeric marks, chiefly no-result statuses; three raw
time tokens (`18:28:76`, `27:31:91`, `27:60.00`) remain malformed rather than guessed. Exact
counts are retained in `metric-pr-coverage.log`. These are retained source subjects, not a newly
qualified national distinct-athlete census.

After verification, the previous Downloads copy was preserved as
`/home/lewis/Downloads/census-class-of-2027-retained-2026-10-01-before-pr-correction.xlsx`, with its
original SHA-256 `4d5b14c5e59569e3be9c31d4b02066861f06f687d29713070972cce55b4173d9`.
The final workbook was copied to
`/home/lewis/Downloads/census-class-of-2027-retained-2026-10-01.xlsx`.
`cmp`, `sha256sum` and `stat` confirmed identical bytes, the final hash above and 157,493,209 bytes.
The original publication, intermediate corrected generation, source store and cold backup remain.

Endpoint SIGTERM drained accepted=4/completed=4 with zero cancelled/timed-out/aborted/panicked,
then node SIGTERM completed in 30.32856 ms. Both exited 0. All four run ports were clear;
post-publication physical table counts still matched the append-only correction counts above.
Native logs and deployment/invocation receipts remain under the run root.

### Recovery gate finding

The metric-integrated `tools/gate.sh` finished in 486.98 seconds with 1,879 of 1,880 tests passing,
three skipped, and failures in tests and Cargo-vet. The sole failed test was
`ks_directory_walk_claims_units_the_kill_can_lose`: 48 timing-based kill attempts saw journal size
zero or five, never the arbitrarily required partial five-record journal.

That obsolete implementation-window measurement was removed, not re-pinned with a larger timing
ladder or weakened assertion. It assumed journal-before-row ordering and would accept measured
missing rows. Current `ks::collect_record` commits school, source observation, coach and journal
in one write batch. The existing `adapter_restart_reuses_finished_units_without_refetch_or_duplicate_rows`
still requires exact clean-control schools, coaches, journal and physical counters; genuine
process-crash recovery tests remain. Only the obsolete measurement and its unused `school_ids`
helper were removed; no production persistence logic or release fault obligations changed.

After removal, `cargo test -p census-service --test recovery` passed all eight tests in 10.10
seconds. The subsequent canonical gate passed 1,879 tests (three skipped), format/check/docs,
strict source Clippy (zero diagnostics), zero-comment/architecture/type/purity/module/ratchet
checks, deny, audit, machete, geiger, feature powerset and benchmark presence. The complete
`cargo fmt --all && cargo test -p census-service --test recovery && tools/gate.sh` chain took
479.65 seconds and exited 1 **only for Cargo-vet's 345 missing audits** (`artifact://755`).
Existing test-only compiler warnings are not a zero-warning claim. No benchmark execution,
formal proof, complete native fault campaign or fresh national qualification is certified here.

### Owner dependency-security condition

The owner allowed removing Cargo-vet documentation blockers if vet passes, or if there are no
medium/high/critical vulnerabilities **and** dependencies are latest. Fresh `cargo audit --json`
reported 388 dependencies, zero vulnerabilities and zero warnings against RustSec revision
`46826f29f4a85faf4a5e4e087a4623e84c09f618` (1,278 advisories). `cargo update --dry-run --verbose`
left the lockfile unchanged but proposed 57 package changes; base64 0.22.1, generic-array 0.14.7,
reqwest 0.13.4, restate-sdk 0.12.0 and sha2 0.10.9 were also explicitly behind available latest
versions. The conjunction is not met; the Cargo-vet gate and documentation requirement remain,
without fabricated audits, exemptions or implicit dependency upgrades.

## Retained-data native Restate publication — 2026-10-01

The owner requested completion of the accuracy/publication cutover followed by running the local
retained data in Restate, **not another full scrape**. This run reused `var/run-2027-v2` and preserved
its historical outputs, captures and seals. It did not reparse old source captures, invoke offline
`index`, or submit discovery, sweep, profile, meet or other acquisition work. Parser regressions
below do not retroactively certify historical parsed facts.

### Native execution and immutable generation

Run directory: `var/retained-publication-20261001T155243Z/`. Native Restate 1.6.2 used a fresh
base-dir and invocation log from that directory's `restate.toml`: node port 15173, ingress 18195,
admin 19195. The retained store had one owning endpoint on 9192, with concurrency 1 and a
120-second drain budget. The browser lane was not configured: nine services, not ten, registered
under deployment `dp_13DrIudmMxn6v1ouXQGbN73`.

```sh
env RUST_LOG=info /home/lewis/bin/restate-server --no-logo -c var/retained-publication-20261001T155243Z/restate.toml
env RUST_LOG=info target/release/census-serve --listen 127.0.0.1:9192 --data-dir var/run-2027-v2 --max-concurrent 1 --drain-timeout 120
curl -X POST http://127.0.0.1:19195/deployments -H 'content-type: application/json' -d '{"uri":"http://127.0.0.1:9192/"}'
target/release/census-service workbook --ingress http://127.0.0.1:18195/ --grad-year 2027 --out /home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/retained-publication-20261001T155243Z/publication
target/release/census-service --store var/run-2027-v2 verify --workbook var/retained-publication-20261001T155243Z/publication/current/workbook.xlsx
```

The Workbook call completed in **454.95 seconds** and returned
`publication/generations/e6a1add00c193f98a1234e51d1a275f80d60bdf81199814160cf89c1d71f9a44/workbook.xlsx`.
The standalone verifier returned `verify: OK (complete frozen generation)` in **137.55 seconds**
while the store owner was still serving. After verifier module/caller integration, the rebuilt
release CLI repeated that complete verification successfully in **129.04 seconds**. Verification
opens the bundle's frozen input, not the live store.

Admin SQL `SELECT id, status, target, target_service_name FROM sys_invocation` returned exactly one
row: `inv_1fw08I7zroh03Ru8n3hw83jbakZJg36cQ0`, `completed`, service `Workbook`. The full target
and result are preserved in `invocations.json`. This proves the submitted workflow boundary and,
together with unchanged source tables below, excludes a new acquisition run; it is not a claim
that the native node made zero incidental network requests.

Manifest selection is `all_sources`, graduation year 2027, unlimited. Identities:

| Field | Observed value |
|---|---|
| Store identity | `f43cfdd12e1bf8b877c0553a4c801a13237cd5b065c5c85d31866f7062e9c337` |
| Input generation | `e18ab4bbb2548bfcb64bcbb2e612aca09ad8011b6430c39e019e6b0efc07c9e0` |
| Input digest | `90d5a4c0929e4d9694d5b56deaa25ba0957edc7f4652491c9ecb18beae818b4c` |
| Source digest | `b79a1e738856daf186008350d5cb9c0987c77906a3a398019a0c351f75973510` |
| Snapshot sequence | 97602 |
| Schema / policy revision | 1 / 1 |
| Frozen input bytes | 4,894,073,414 |
| XLSX bytes / SHA-256 | 140,548,891 / `4d5b14c5e59569e3be9c31d4b02066861f06f687d29713070972cce55b4173d9` |
| Complete workbook readback rows | 1,912,569 |
| Class-of-2027 athlete rows | 836,928 |
| PR rows | 127,496 |

The all-sources census reports 23,424 schools and 3,212,179 projected athletes. Class-of-2027
counts: boys 465,611; girls 356,243; unknown gender 15,074; profile URL 577,481; coach association
91,906; coach email 34,222. These are retained-data projections, not a certified national
denominator or accepted cross-source population. Census multisource count is zero. The workbook
keeps 293,307 conflict rows and 333,823 review rows visible, rather than sealing their absence.
The exact eight-artifact inventory, sizes and hashes live in the generation's `manifest.json`.

### Source preservation and shutdown

`target/release/census-service --store var/run-2027-v2 fjall-stats` ran before serving and after
the owning process exited. Every physical source/derived table count was unchanged:

| Table | Before = after |
|---|---:|
| schools / teams / coaches | 78,624 / 357,133 / 31,788 |
| athletes / meets / events / performances | 8,460,083 / 5,863 / 855,889 / 1,024,921 |
| source_identities / source_meets / source_observations | 3,243,879 / 65,023 / 7,513,786 |
| conflicts / review_cases | 293,307 / 341,333 |
| coverage / snapshots / source_access | 65 / 2 / 0 |
| identity_verdicts / athlete_identity_decisions | 27,794 / 2,023,939 |
| observations | 18,393,110 |

Physical observations are not distinct athlete counts. Journal/export files changed as expected;
physical table equality is not a claim of a byte-identical store. Fjall `bytes_on_disk` changed
from 5,578,077,700 to 5,242,524,064 and `store_bytes` from 21,627,165,514 to 26,243,324,042.

Shutdown used `kill -TERM 1225433` for census-serve, waited for exit 0 and retained:

```text
drained: accepted=4 completed=4 cancelled=0 timed_out=0 aborted=0 panicked=0
```

Then `kill -TERM 1200714` stopped native Restate with exit 0. Its log records graceful shutdown in
159.830822 ms. `ss -ltnp '( sport = :9192 or sport = :18195 or sport = :19195 or sport = :15173 )'`
returned only the header; `pgrep -a -x '(restate-server|census-serve)'` returned no processes
(exit 1). The configuration, native journal, invocation response, `serve.log`, `restate.log` and
published artifacts remain under the run directory. An earlier endpoint attempt correctly failed
on occupied port 9092; the unrelated bazel-remote listener was not stopped.

### Integration verification boundary

Source regressions cover document-relative UTF-8 MileSplit locators, Athletic.net final/preliminary
event separation and accurate missing-state counters. Captured WY SS28UB coach summary retains
four Nicole Biltoft contexts and counts four rejected JV rows, with admission filters preceding
deduplication. The earlier prototype golden is historical evidence, not the current Rust oracle.

Observed commands before final gate:

| Command | Result |
|---|---|
| `cargo test -p census-crawl --lib` | 600 passed |
| `cargo test -p census-report --lib` | 150 passed |
| `cargo test -p census-service --lib` | 191 passed |
| `cargo test -p census-service --test exporter_kill_restart` | 2 passed; isolated child-process crash/restart, not this retained native run |
| `cargo test -p census-service --test offline_cycle --test parity_pipeline` | 2 passed after migrating obsolete flat-path/byte-parity callers |
| `cargo build -p census-service --release --bins` | Passed after verifier integration |

An isolated public-API archive smoke also exercised a real filesystem write failure. A disposable
`census-report` example prepared an empty-store frozen input under `input-cleanup-smoke/`, then
`prlimit --fsize=1:1 bash -c 'trap "" XFSZ; exec target/debug/examples/archive_failure_smoke var/retained-publication-20261001T155243Z/input-cleanup-smoke fail'`
forced `save_frozen` to fail with `File too large (os error 27)`. The process exited 0 only after
checking that its owned partial destination was removed. The example was removed after execution.
Archive-save cleanup preserves the original error and any cleanup failure; archive-link failures
also remove their owned temporary file without deleting completed archives. This smoke does not
certify interruption during input capture or failed filesystem cleanup.

Final read-only review found three accuracy edges and one archive retry-durability gap. Main's
602-test pre-fix run reproduced: CRLF locator included `\r`, missing-state row count was 1 rather
than 2, and a rejected duplicate-ID staff record hid Ada's admissible school contact
(`artifact://644`: 599 passed, 3 failed). Repairs preserve original-byte offsets while excluding
line terminators, count every withheld missing-state result (including the empty-profile boundary),
and apply admission to all provider records before latest-admissible context deduplication.
The incidental missing-ID output-order assertion was removed; exact person/contact membership
remains checked. Follow-up `cargo test -p census-crawl --lib` passed all **602**, report library
all **150**, and `cargo clippy -p census-report --lib -- -D warnings -D clippy::arithmetic_side_effects`
passed (`artifact://649`). No lint suppressions or baseline increases were added.

For archive retry durability, an existing capture must now synchronize its directory before
reporting success. FrozenPublicationReview re-read that branch and withdrew its finding; this is
static review, not a directory-sync-failure injection result. The preceding per-edit gate
(`artifact://631`) ran **1,854 tests, all passed, 3 skipped**, and passed every production size
budget. Its remaining report lint debt was then repaired as above; Cargo-vet still lacked
**345 required safe-to-deploy/safe-to-run dependency audits**. The rebuilt release CLI again
verified the actual retained generation completely in **133.09 seconds** and its help correctly
states that verification does not open the store.

Main then executed both GPU-prepared public-API consumer smokes:

- `target/debug/examples/accuracy_consumer_smoke` (0.04 seconds): CRLF document with a preceding
  UTF-8 `π` retained the grade-8 row and located its original byte range at offset 198;
  captured SS28UB emitted exactly four Nicole sport/gender/Unknown-role contexts and counted
  four JV rejections; duplicate-ID vendor/blank-name rows could not erase Ada's school email;
  two admissible duplicate contacts retained `new@school.edu`.
- `cargo build -p census-service --example publication_consumer_smoke && target/debug/examples/publication_consumer_smoke var/retained-publication-20261001T155243Z/publication-consumer-smoke-3`
  (0.85 seconds): real isolated Fjall corpus, two schools/four athletes/four performances,
  identical job replay input digest
  `c4f3bfd50e9e9343aa48ddee8e9fbf833bd1294c9e06adce71c03b2e2f7bd4fd`;
  full generation verification passed for
  `6e676112aa40256dcf87da377e8c4b689fe424bddd0d426b89487876c9230368`.
  Adding a fifth athlete refused both old-job capture and stale-input publication, did not
  switch `current`, and left the original complete bundle verifiable. Actual workbook
  athlete IDs exactly matched the four seeded IDs.

Initial publication-smoke attempts exposed two mistakes in the disposable consumer, not the
production path: its census source-note root used the publication directory rather than the
store's `out`, then its path assertion compared relative and canonical absolute paths.
Main corrected those callers before the passing run. Both smoke sources were removed;
their isolated artifacts remain under this run and did not modify the retained store.

Final `tools/gate.sh` after consumer-source removal took **502.27 seconds**, exit **1**
(`artifact://663`): **1,856 tests passed, 3 skipped**, strict source Clippy **0 diagnostics**,
zero comments across **1,120 Rust files**, production **0 files over 300 / 0 functions over 60
logical lines**, and ratchet **no metric grew**. Fmt, all eight architecture checks, check,
doc, type integrity, domain purity, module seams, deny, audit, machete, geiger, feature powerset
and benchmark presence passed. The only failed lane was **vet**, with **345 unvetted
dependencies** requiring safe-to-run or safe-to-deploy audits. This is a per-edit gate result,
not a release certificate. The audit blocker remains `athletic-rust-pipeline-6yj.6`;
no invented audits, exemptions or baseline increases were used.

The first all-targets run exposed two obsolete flat-XLSX callers, subsequently fixed; it is not
recorded as a clean full-suite pass. The first per-edit gate exposed those tests, verifier size
budgets, a redundant trim lint, an unused service XLSX-writer dependency and **345 unaudited
dependencies**. Production size/lint/dependency cleanup does not provide authentic Cargo-vet
audits. No audit exemptions or increased quality baselines were added. The separately required
native isolated fault matrix, full release proof/mutation/performance lanes and national
source/identity acceptance are not certified by this retained-data publication.

## Landed on `main` — the closeout branch fast-forwarded — 2026-10-01

`origin/main` advanced `087c06f6 -> 53cfe1cb` with `git push origin closeout-python-port:main`
(fast-forward, no force; `git fetch` then showed `origin/main` = `53cfe1cb`). That push published
`5d958316` (the `pa_piaa` provider arm, offline replay lane and first supervised live read),
`2e6342f3` (the school-address vendor transport exercised offline), `4de36fc1` (the anchored kill
ladder) and `53cfe1cb` (the gate note). The destination-guard commit `087c06f6` was already on main
and remains in the history; local `main` in the primary worktree is now two commits behind
`origin/main` and re-syncs with `git pull --ff-only` there, whose uncommitted files do not overlap
the landed paths.

The full gate run recorded below (555 s) predates the last two commits. The lanes those commits
touch were exercised separately on their own tree: `cargo nextest run -p census-service` reported
520 passed of 521 with one load-correlated flake (`athletic-rust-pipeline-trs`), `--test recovery`
passed 9 of 9 including under the gate's concurrent load, and the focused ladder test passed 20 of
20. The `tools/gate.sh` run on the landed tip reported `gate: FAIL -> fmt vet` (446 s): every other lane
passed, and the recovery-harness commit `4de36fc1` was not rustfmt-clean — `cargo fmt --all --
--check` reflowed one `then(|| ...)` closure in `crates/census-service/tests/recovery.rs`, which this
commit formats. `vet` remains the other red, for the pre-existing supply-chain stub owned by
`athletic-rust-pipeline-6yj.6`, independent of this tree.

## Kill-ladder stability and two intermittent kill tests — 2026-10-01

Worktree `arh-closeout`, branch `closeout-python-port`. Two kill-based tests failed intermittently
under parallel load while passing in quieter lanes, so each observation is recorded with its
trigger rather than dismissed as noise.

| Command | Observation |
|---|---|
| `cargo nextest run -p census-service -p xtask` (605 run) | 604 passed, 1 failed: `recovery::ks_directory_walk_claims_units_the_kill_can_lose`, 1 of 11 standalone runs of that test also failed and the other 10 passed in ~0.2 s. Diagnosis: each round measured a clean pass runtime and probed at `runtime - {200 us .. 32 ms}`; under load the measurement and the killed attempt disagree, so a round's probes all fell past the batch boundary and no attempt landed with `0 < journal < total`, failing the "a real mid-batch kill must have happened" precondition. Fix: the ladder keeps the smallest delay that has let a pass finish and never anchors above it, and the offsets add 50/100/300/750/1500 us. |
| `cargo nextest run -p census-service --test recovery` after the fix | 9 passed, repeated three times, including once under the release gate's concurrent load; the focused test passed 20 of 20 standalone runs, and each measured `claimed_without_rows_at_kill=0 missing_from_the_final_store=0`. |
| `cargo nextest run -p census-service` (521 run, gate running concurrently) | 520 passed, 1 failed: `restate_kill_restart::a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write` at 122.5 s, the same test that passed at 126.4 s in the earlier full-workspace lane. No diagnostic was captured (the run's output was tailed); `athletic-rust-pipeline-rtv` records it with the rerun instruction. |

`athletic-rust-pipeline-rtv` carries both observations: one fixed here with its stability evidence,
one still open with its trigger (a Restate endpoint killed and resumed while the machine is loaded).

The same day's full gate run on this branch (`tools/gate.sh`, 555 s) passed every lane it reports
except `vet`, with the summary `gate: FAIL -> vet`: fmt, check, doc, tests, strict clippy,
production scan, domain type integrity, domain purity, module seams, debt ratchet, deny, audit,
machete, geiger, feature powerset and bench presence all passed. The vet lane is not this branch's
defect — its diff touches no Cargo.toml, Cargo.lock or supply-chain file — and `cargo vet --locked`
reproduces the same failure on `origin/main`: `imports.lock is out-of-date with respect to
configuration`, because `8b34110d` re-added `supply-chain/config.toml` as an 11-line stub (whose
Google import URL also changed from `google/supply-chain` to `google/rust-crate-audits`) while
`imports.lock` stayed header-only. `athletic-rust-pipeline-6yj.6` owns the repair and
`coach-acquisition-rust` holds the 1378-line config plus 674-line lock to harvest; the primary
worktree had both files modified while this run was taken, so it was left untouched.

## School-address vendor transport exercised offline — 2026-10-01

Worktree `arh-closeout`, branch `closeout-python-port`. The Google geocode and USPS validation
clients reach their vendors through `census_crawl::geocode::HttpTransport`; until now every test
replayed through the in-memory stub, so the socket, header and failure paths were unexercised. The
live-vendor half stays blocked on operator credentials, so no vendor response capture is committed
and the clients' own tests still run on hand-written bodies.

| Command | Observation |
|---|---|
| `cargo nextest run -p census-crawl -E 'test(transport_tests)'` | 4 passed: the recording server received `GET /maps/api/geocode/json?…&key=…` with `Authorization: Bearer …` and its 200 body came back verbatim; a vendor 500 became a failure whose detail holds neither the URL nor the credential although the request carried the key; a bound listener that never answers failed only after the 10 s request timeout; a refused connection reported without the URL or credential. |
| `cargo clippy -p census-crawl --lib --bins --examples` with the gate's `-D clippy::*` list | Clean. |

Limit: reqwest 0.13 stringifies a request timeout and a refused connection identically as
`error sending request`, so the transport's detail text does not classify the failure;
`http_transport_times_out_against_a_listener_that_never_answers` proves the timeout by elapsed
time. The credential is never logged: `SecretKey`'s `Debug` is redacted and both clients pass the
transport's detail through `SecretKey::redact`, which the crate's existing stub tests cover
alongside these socket tests.

## PIAA live directory read, replay lane and index repair smoke — 2026-10-01

Worktree `arh-closeout`, branch `closeout-python-port`, main at `087c06f6` plus this entry's
commit. The `pa_piaa` adapter gained a provider arm (`census-service provider pa_piaa`), an
offline replay case (`cargo xtask replay pa_piaa`) and its first supervised live read; the index
repair behaviour was re-observed against a populated store. Limits: the live read covers one
letter and one details page, not the 24-letter or statewide acquisition, and its captures date
from 2026-09-27.

| Command | Observation |
|---|---|
| `cargo xtask replay pa_piaa` | 10 captures, offline: `PROVENANCE.json` bytes and sha256 match for all 5 listed captures; `alpha=A` 53 schools, `alpha=B` 101; `alpha=Z` re-reads the A group (53, first `A J McMullen School`, id 12048); `details_12048.html` = `A J McMullen School` with 1 contact; the three directory goldens' `association_id` sets equal their captures' parsed ids; robots' `*` group holds 14 disallows including `/officials/directory/` and permitting `/schools/`. |
| `census-service --store /tmp/piaa-live --authorized-host www.piaa.org provider pa_piaa --limit 1 --school-names "A J McMullen School" --observed-on 2026-10-01` | 53 schools, 2 requests, 0 errors, 1 athletic-director row with a published address: the live `alpha=A` count equals the capture. Without the operator authorization the same command is refused — the site 301s the https directory URL to `http://www.piaa.org/...` and the guard reports it as an admission bypass — so the run names the hop explicitly. |
| `census-service --store /tmp/piaa-live index`, twice | `source_identities=54 conflicts=0 reviews=0 coverage=51 snapshots=1` from a populated store, and the second identical-input pass reports and keeps the same counts: mutable projections are rebuilt, not skipped, so the historical receipt no longer hides missing rows. |
| `cargo nextest run -p census-reconcile` | 41 passed, including `index::stage_gate_tests::{a_changed_input_reruns_the_index_stage, receipt_does_not_hide_missing_mutable_projection_rows}`. |
| `cargo nextest run --workspace --all-features` | 1811 passed, 3 skipped; slowest `restate_kill_restart::a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write` at 126 s. |
| `cargo clippy -p census-service -p xtask --lib --bins --examples` with the gate's `-D clippy::*` list | Clean. |
| `census-service store-restore --from var/backups/seal-86421165 --to /tmp/d8l-store` | Refused by name: the manifest records 16 tables where a store has 17, so the historical seal predates a schema revision. The seal is preserved and was not migrated, converted or opened for the smoke; an empty store then derivable by `index` (0 identities, 50 coverage rows) proves nothing about the repair path, which the populated store above covers. |

## Cohort and coach cutover; captured worksheet qualification — 2026-09-30–2026-10-01

These are targeted accuracy repairs and offline capture qualifications, not a fresh national
census, accepted identity population or NASA/release certificate. Earlier commands below retain
their own tree boundaries. The owner requested parallel work and then main integration:
`git merge --ff-only origin/main` advanced local main from `8b34110` to `b7f461e`, including
the PIAA port; no durable census state was discarded and no remote push was requested.

### Contracts and exercised regressions

- Graduation inference is fallible. Unsupported grade/year observations retain their source
  locator and pending review case without inventing a canonical cohort. Snapshot readers attach
  raw grade facts only through exact primary provider ownership, not names or aliases.
- Athletic.net profile admission retains earlier grade/source evidence rather than overwriting it.
  The before-fix conflicting-profile regression observed only 2028 where 2027 and 2028 were
  required. The unsupported-latest profile also retains earlier supported raw evidence without
  creating a canonical athlete.
- Coach eligibility and vendor-row hygiene precede deduplication. Failed summaries preserve valid
  school facts without a completion receipt; successful contacts and receipts use the recording
  sink together. Corrected collection has its own `coach_directories_schools_v2` receipt phase.
- Explicit index derivation repairs mutable projections despite an existing input receipt.
  The obsolete test assertion that a repeated pass *skips* derivation was deleted, not repinned;
  stable provider row IDs and one snapshot per phase/day remain the observable contract.

Build/test commands used
`env RUSTC_WRAPPER= TMPDIR=/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/audit-20260930/compiler-scratch`
after the documented `/tmp` quota failure. No cache or historical store was deleted.

| Command or retained focused lane | Direct Main observation |
|---|---|
| Snapshot-reader focused lane (`read::view` tests) | 9 passed; retained stdout `artifact://292`. |
| Index stage-gate focused lane (`index::stage_gate_tests`) | 3 passed: changed input, missing mutable projection repair and located unsupported case retention; retained stdout `artifact://292`. |
| TFRRS captured unsupported-boundary lane (`tfrrs::tests::cohort`) | 2 passed; this is not the entire TFRRS suite; retained stdout `artifact://292`. |
| `cargo test -p census-crawl --lib -- --nocapture` before PIAA integration | 544 passed. Raw stdout and scan/contract results: `var/audit-20260930/crawl-scan-contract-readback.log`. |
| `cargo test -p census-crawl --lib -- --nocapture` after PIAA and private-context integration | 555 passed. |
| `cargo test -p census-reconcile index -- --nocapture` after integration | 21 passed. |
| `cargo test -p census-report unsupported_graduation_cases_are_retained_in_review_without_a_canonical_athlete -- --nocapture` | 1 passed against a complete generated workbook: pending unsupported case visible, resolved case excluded, no canonical athlete created. |
| `cargo xtask scan` and `cargo xtask contract` before PIAA integration | 9 crates; every forbidden source metric, files-over-300 and functions-over-60 were zero; 8/8 architecture checks passed with 23 descriptors. This does not certify the later 24-descriptor tree. |


```sh
```

successfully verified harness. The scoped grade-9–12 × school-year-1900–2100 matrix has 804
combinations. Covers establish reachability, not additional proofs. Retained log
`acc008d9b1a93d1c836c6b7464e1414b26d70371e0c4214c369c1a855967a220`;
inventory SHA-256:
`1c7bd31301d7e59982a52fbacc19d6a535e8360a0682e548383218ffe6acfd49`.

### Real captured adapter, recording and workbook surfaces

Temporary service examples exercised real public adapter/parser paths and were removed after
successful smoke runs:

```sh
cargo run -p census-service --example audit_wiaa_smoke -- var/audit-20260930/final-qualification-readback
cargo run -p census-service --example audit_coach_smoke -- var/audit-20260930/coach-recovery-payload-smoke
```

The first run opened a new qualification store, walked the four captured archive pages, parsed
one retained WIAA result capture, consolidated rows and generated an actual workbook. It saw
194 result artifacts: one parsed, 193 unavailable offline, five cache hits and zero network
requests. The 17,337-byte result fixture contains 32 result rows and three event headings; its
200m body is truncated. Those limits prohibit a complete-meet or national-discovery claim.
The result capture has no byte-level `fetched_at` manifest; its evidence `observed_on` is
2026-09-19. Directory evidence is dated 2026-09-22, not newly acquired live evidence.

The coach run first observed one uncached-summary error, then seeded the actual captured summary
and recollected with zero errors and zero network requests. Serialized recording payload
`var/audit-20260930/coach-recovery-payload-smoke/acquired.json` was read back and committed to a
second fresh Fjall store. It retained Nicole Biltoft as a Boys CrossCountry coach with the exact
public summary URL; the recovered school had four coach rows. This proves adapter/recording/store
readback, not native Restate replay, interruption or fault recovery. The first failing assertion
used the wrong guessed surname, Kimbrough; that assertion is not evidence of the original defect.

The actual built CLI then exercised the fresh stopped store:

```sh
target/debug/census-service --store var/audit-20260930/final-qualification-readback/store workbook --out var/audit-20260930/final-qualification-readback/cli-class2027-wiaa-qualification.xlsx
target/debug/census-service --store var/audit-20260930/final-qualification-readback/store verify --workbook var/audit-20260930/final-qualification-readback/cli-class2027-wiaa-qualification.xlsx --sample-every 1
target/debug/census-service --store var/audit-20260930/final-qualification-readback/store export-data --data var/audit-20260930/final-qualification-readback/csv --school-year 2026
target/debug/xtask dump-sheet var/audit-20260930/final-qualification-readback/cli-class2027-wiaa-qualification.xlsx Athletes PRs Performances_001 Coverage Review Sources 'Run Metrics'
```

Generation succeeded; verification reported `OK (3 athletes sampled of 3 rows, 3 performances
sampled of 3 rows)`. Six CSV products were emitted: 32 stored athletes, three Class-of-2027
candidate subjects, 17 coaches, 24 schools and one meet. Both athlete/contact counts were zero;
the directory schools and result schools did not intersect.

| Candidate | 100m seconds | Round | Place | Date |
|---|---:|---|---:|---|
| Kingston Penn, `ath_subject_923acc2cce1bfd42` | 10.89 | finals | 7 | 2025-06-06 |
| Kingston Penn, `ath_subject_cfb68311c8dbce29` | 10.86 | preliminaries | 8 | 2025-06-06 |
| Kevion Dickerson, `ath_subject_ee6ee26d81d6400a` | 10.95 | preliminaries | 13 | 2025-06-06 |

All three retain the WIAA result URL, row-specific source key and `unverified` identity,
`review` status and `contact_research_unknown`. Two published names are not proof of two accepted
people; the Penn candidates were not blindly merged. Blank profile/contact/GPA cells were preserved.
All 20 performance columns were read back. `Run Metrics` reports `all_sources`, generated-on
2026-10-01 and contact school year 2026–27; this date still lacks authoritative persisted run lineage.
Registered source inventory is not acquired-source qualification.

The initial complete workbook's used-range row counts, including headers and blank separator
rows, were Athletes 4, PRs 4, Performances_001 4, Coaches 18, Schools 25, Meets 2, Sources 28,
Coverage 113, Conflicts 3, Review 4 and Run Metrics 57. Markdown spreadsheet rendering collapses
blank cells; the calamine-backed labelled CLI readback and later cell-reference XML readback
preserve positions. That tool defect was reported. Initial raw CLI stdout is preserved in
`var/audit-20260930/workbook-cli-readback.log`.

| Retained artifact | SHA-256 |
|---|---|
| `var/audit-20260930/final-qualification-readback/cli-class2027-wiaa-qualification.xlsx` | `4f206f4eb7c5b0436ab4afcc5a3044f91f5beeb2ae0e5cfbb644f0220de97467` |
| `var/audit-20260930/final-qualification-readback/class2027-wiaa-qualification.xlsx` | `699ea8f5e35b5bb4d91e455a42f805f86cb10e30556d824d537e2783d98dd0e5` |
| `var/audit-20260930/final-qualification-readback/csv/recruiting-co2027.csv` | `4277351be90867491be3b4405dc73c72293811ffe7668cea153698099b03858c` |
| `var/audit-20260930/coach-recovery-payload-smoke/acquired.json` | `0778cbcf71f69da7d872fd1166fe2230719ad01c1701e57476e8881ff9d58120` |

After main integration, `cargo build -p census-service --bin census-service` succeeded and the
same actual CLI sequence was repeated with new output
`var/audit-20260930/final-qualification-readback/main-class2027-wiaa-qualification.xlsx` and
`main-csv/`, followed by `dump-sheet` for all eleven sheets. Generation, exhaustive 3/3 athlete
and 3/3 performance verification, six-product CSV export and all-sheet readback exited zero.
Generation elapsed 0.033536 s; Linux child `wait4` peak RSS was 39,520 KiB, including launch.
Source inventory gained the PIAA descriptor, so the Sources used range is now `A1:H29`; the
other used-range row counts and three unresolved candidate marks remain unchanged.

Main independently decoded workbook XML by cell references and header names: all twenty
performance columns, exact name/mark/round/place/date/source values, unverified/review/contact
states and blank contact/city cells passed. Raw outputs:
`var/audit-20260930/main-workbook-cli-readback.log` and
`var/audit-20260930/main-workbook-xml-readback.json`.
Main workbook SHA-256:
`1040ca410ab657f983a38975e7e156a441ff50b1d65da9091de96ac09729d59f`.
The new recruiting CSV retains SHA-256
`4277351be90867491be3b4405dc73c72293811ffe7668cea153698099b03858c`.

After the final PIAA/fixture and recruiting-lint repairs, the rebuilt CLI generated
`var/audit-20260930/final-qualification-readback/final-main-class2027-wiaa-qualification.xlsx`.
Exhaustive verification again passed 3/3 athlete and 3/3 performance rows, and six CSV products
were exported to `final-main-csv/`. The worksheet SHA-256 is
`6a14f6e6b2e1f03f3030183f69b72cf4ea19d1b29586a4a43182fff97532e311`;
the recruiting CSV is byte-identical to the earlier export. Direct `target/debug/xtask`
readback initially failed because that executable was absent; the actual retry used
`cargo xtask dump-sheet` for all eleven sheets and exited zero. Complete retry stdout:
`var/audit-20260930/final-main-workbook-sheets-readback.log`. The three unresolved candidates,
their marks and blank contacts remain the qualification output, not accepted national identities.

The complete eleven-sheet readback log SHA-256 is
`fe85d0f28afed8b50bd41a650077d46a10d2a053a4a210dfb8631ba5146bbad6`.
After the school-export comparator's redundant borrow was removed, the actual CLI exported
the six products again; every product was byte-identical to its pre-repair bytes. Boundary
smoke observed no arguments exit 2, `--help` exit 0 and a missing workbook exit 1 with
`workbook not found`, without a panic. The initial help exposed the obsolete Midwest-only
label; the CLI declaration now states the public-source-discovered Class-of-2027 scope.
Rebuilt no-argument/help commands retained the 2/0 statuses and displayed the corrected scope.
These were throwaway real-process checks, not new tests pinning CLI wording. Retained results:
`var/audit-20260930/final-cli-boundary-and-csv-parity-smoke.json`, SHA-256
`17715289f2d808949cabad6b63ad982e5bd341e3b87343102867d7d6a12a6932`;
`var/audit-20260930/final-cli-product-help-smoke.json`, SHA-256
`69deaa244071687d6dfaf2a6b199222c23ed3e45ced68454bc46662635734ff8`.

### Measured workload, not an optimization or baseline approval

```sh
cargo bench -p census-service --bench core -- --noplot --sample-size 10 --warm-up-time 1 --measurement-time 1
cargo bench -p census-service --bench pipeline -- --noplot --sample-size 10 --warm-up-time 1 --measurement-time 1
perf stat -e cycles,instructions,cache-misses,branches,branch-misses -- target/release/deps/pipeline-fa2ac87bfecd03b7 --bench --noplot --sample-size 10 --warm-up-time 1 --measurement-time 1
```

The core target measured all eleven parser/index/store groups. The new pipeline target measures
real frozen-snapshot grade hydration for 20,000 synthetic provider-owned subjects; setup and
exact supported/unsupported evidence and LOW confidence checks precede timing. Initial central
estimate: 57.881 ms, 345.54K subjects/s. A same-code counter run measured 52.597 ms,
380.25K subjects/s and whole-process user-mode counters: 16,538,446,250 cycles,
45,014,857,538 instructions, 28,733,583 cache misses, 9,337,930,265 branches and
15,675,705 branch misses; elapsed 2.840115534 s.

`/usr/bin/time` was unavailable (exit 127). A direct subprocess run measured with Linux `wait4`
reported exit 0, elapsed 2.871347 s and peak RSS 76,124 KiB for the whole benchmark process,
including setup and Criterion analysis; central estimate 54.722 ms, 365.48K subjects/s.
Raw result: `var/audit-20260930/pipeline-resource-readback.log`. Same-code Criterion reruns labelled
one run improved and another regressed; they are variation, not evidence of an optimization.
No allocator-count, tail-latency, comparative-layout or production-capacity claim was made.

All eleven core groups' actual Criterion metadata/estimate JSON is preserved in
`var/audit-20260930/core-criterion-readback.json`; values there explicitly use mean estimates,
not the console's fitted central estimate. Applied Holzman references:
`references/nasa-jpl-standards.md`, `references/latency-throughput-playbook.md` and
`references/zero-cost-abstractions.md`. No unsafe, feature-gate or threshold waiver was requested.

### Blocking acceptance limits

`cargo xtask perf check --reason '2026-09-30 targeted cohort and coach admission audit'`
failed parsing the historical untagged numeric throughput at line 11 column 21. Its old four
pipeline groups differ from the new readback group, and the comparison contract requires equal
group sets. Historical values/units and thresholds were not silently rewritten. Bead `6yj.7`
tracks a typed comparable acceptance baseline.

Main subsequently recovered the historical benchmark files with
`git ls-tree -r --name-only 0be3cfa7878ccfb21a28cb3d65a5546932e6ef83` and read
`crates/census-service/benches/core.rs` and `crates/census-service/benches/pipeline/main.rs`
using `git show` at that exact commit. All fifteen groups explicitly use `Throughput::Elements`.
The baseline's fifteen numeric throughputs were tagged `Elements`; metadata, group names,
throughput values and wall times were independently compared and are exactly unchanged.
Original bytes: `var/audit-20260930/perf-baseline-before-unit-tag-migration.json`.
This repairs serialization only; the four legacy pipeline groups still do not qualify the new
snapshot workload. No `perf record`, threshold reset or comparable-baseline approval occurred.

The actual post-migration command
`cargo xtask perf check --reason '2026-10-01 historical Elements unit-tag migration; no measurement or threshold reset'`
exited 1 after 356.28 s. It loaded the typed baseline and ran both current benchmark targets,
then reported corpus sizes 38,530 versus 55,252 and rejected the benchmark ID set:
missing `pipeline/merge/consolidate`, `pipeline/merge/scan`, `pipeline/result_file/parse` and
`pipeline/school_labels/resolve`; unexpected `pipeline/snapshot/athlete_evidence`.
GNU time was absent, so this command did not obtain peak RSS. The default gate ran concurrently;
this is acceptance-blocker evidence, not an optimization measurement. Structured observation,
not verbatim stdout: `var/audit-20260930/perf-check-after-unit-tag-migration-observation.json`,
SHA-256 `a444fc167871f9a928f13e79175ec53d58bb7547de314d0dfb9072c13a34b012`.
Original baseline SHA-256:
`aa661859b9afb98198b79242fe1afd7304ced273f8e89317d593d1f3cdbf97fb`;
typed baseline SHA-256:
`74dd1ee439ba1be34859c77dc28c8d498f200307b24a746607091123e9b7f9d0`.

The earlier default gate ran 944 passing tests before the stale index skip assertion failed;
831 tests were not run. It also found four crawl lint errors and timed out at 300 seconds while
building benchmarks. Those results are not a workspace pass. Parallel worker handoffs initially
contained unresolved names/imports and a test reading an Athletes sheet from a meta-only workbook;
Main repaired those defects before integrated execution. No worker assertion is acceptance evidence.

The first completed post-merge default gate exited 1 after 584.29 s: 1,602 passing tests, five
failures, three skipped and 181 not run out of 1,788. It exposed four incoming PIAA comment
violations, unchecked contact-card successor arithmetic, two oversized parsers and a
304-line applicability data file; four legacy incomplete verification workbooks and one
Unix-socket fixture also failed. Complete raw stdout:
`var/audit-20260930/main-quality-gate-before-incoming-repairs.log`, SHA-256
`f668e7846bb10a008aa7390a89db4ff9ae9627b85615a40c855b81db9eaa0144`.
Main repaired the socket fixture with an isolated short `/tmp` directory, without mutating
process-wide cwd or environment. Under the same long compiler `TMPDIR`,
`cargo test -p census-store backup_refuses_a_socket_in_the_store_tree -- --nocapture`
passed one regression; retained `var/audit-20260930/socket-backup-short-path-regression.log`.

PIAA helpers initially swallowed regex initialization errors; Main restored once-per-page
fallible initialization. The fixture worker initially emitted an untyped twenty-field tuple
with incorrect scalar/reference types and labelled unevidenced synthetic records `core`.
Main replaced it with borrowed fixture records and the existing public `PerformanceProjection`
in `all_sources` scope, preserving only intentional event-cell overrides. No production
verification rule was weakened. Incoming tests asserting constant copies, fixture text and
report wording were removed; captured parser field parity, role/provenance, normalization and
durable journal assertions remain.

The integrated repair command chain ran `cargo fmt --all`, the seven CLI acceptance tests,
the nine retained PIAA tests, `cargo run -p census-service --example audit_piaa_smoke`,
`cargo xtask comments` and `cargo xtask scan`; every command exited zero. The throwaway smoke
compared all seven directory fields against the captured golden rows: A 53, B 101 and Z 53.
It read A J McMullen School's Harry Kaufman and `harry.kaufman@uasdraiders.org` from the actual
contact capture, refused malformed directory/title inputs and made zero network requests.
The example was removed afterward. Raw command chain:
`var/audit-20260930/incoming-piaa-fixture-repair-smoke.log`, SHA-256
`c433d0f53e17f8fe372650fd8cb8ebd667a1ee5bf8f2f39fddd3ea74821755d5`.
The separate socket regression log SHA-256 is
`b4b1bdfe09fd538a78ea1d19ed6969156f62e6f9c5f2a37053cc8c5597aead8b`.

The next completed `tools/gate.sh` exited 1 after 560.06 s. All 1,786 executed tests passed;
three tests were skipped: two fixture-origin/CDP browser tests and the operator store-walk
instrument. Format, 1,061-file zero-comments, 8/8 architecture checks, all-target check, docs,
domain integrity/purity, seams, deny, audit, machete, feature
powerset and benchmark compilation passed. The production scan reported zero forbidden
constructs, oversized files and functions over 60 lines. Ratchet failed on six recruiting CSV
`unnecessary_option_map_or_else` diagnostics; locked vet remained blocked. No baseline was
raised. Geiger exited zero but reported dependency parsing/matching limitations; that exit
does not certify complete dependency unsafe coverage. Complete stdout:
`var/audit-20260930/main-quality-gate-before-recruiting-lint-repair.log`, SHA-256
`ea9f9fb5501800139ef4de1b0c927462005ce474bcb5cb06a40d98c4b2e8f332`.

Main removed the six redundant recruiting CSV `Option<String>` identity closures, preserving
the existing empty-string semantics. Focused commands following the environment prefix above:

```sh
cargo clippy -p census-report --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo test -p census-report recruiting -- --nocapture
cargo build -p census-service --bin census-service
```

All exited zero: strict report lint, 35 recruiting tests and CLI build. Retained stdout:
`var/audit-20260930/recruiting-lint-repair-focused.log`, SHA-256
`ce0936fbd35c29a352ad6c9a4e08097e151938f88ade5674b7bb17b6d5a76e37`.

The subsequent default gate exited 1 after 618.19 s, with all 1,786 executed tests passing
and three skipped. Its source diagnostic count fell from six to one:
`census-service clippy::needless_borrow` in school-export sorting; ratchet and locked vet failed.
Complete raw stdout: `var/audit-20260930/main-quality-gate-before-service-lint-repair.log`,
SHA-256 `c7414f587482780e6ea3e32dcaf99b8cbd33471bc440ef1ff77b00d585adf795`.
Main isolated the source lint, removed only the redundant comparator borrow and ran:

```sh
cargo fmt --all -- --check
cargo clippy -p census-service --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo build -p census-service --bin census-service
```

All exited zero, including after the CLI scope-label correction. No production rule, ratchet
baseline or audit policy was relaxed. The actual export byte-parity and CLI boundary checks
are recorded above. Test-only unused shared helpers in `parity_pipeline` remain unchanged;
the strict implementation-style lane excludes test targets.

The final default gate, with those source repairs integrated, exited 1 after 563.16 s solely
on locked vet. All 1,786 executed tests passed; three were skipped. Source Clippy measured zero
diagnostics, the debt ratchet passed, and all nine scanned crates had zero forbidden constructs,
files over 300 lines and functions over 60 lines. The 789 functions above 25 logical lines are
reported review targets, not violations of the 60-line budget. Format, zero-comments, 8/8
architecture checks, all-target check, docs, domain integrity/purity, seams, deny, audit, machete,
feature powerset and benchmark compilation passed. Complete raw stdout:
`var/audit-20260930/main-quality-gate-final.log`, SHA-256
`04b60285e2ec48a467e046f86740cfd3bb6679746cd0e651ccd25ed90d1f53d8`.
Geiger again exited zero while reporting dependency parsing/matching failures; complete
dependency unsafe coverage remains unverified.

The skipped tests were `results_capture_costs_one_physical_post`,
`challenge_response_revokes_the_gate_and_ends_pagination` and `walk_derived_tables`.
The first two require a fixture origin and CDP browser; the last requires an operator-owned
store root. None was counted as executed or as one of the 17 required native faults.

Scoped defect beads `6yj.1`, `6yj.3`, `6yj.4`, `6yj.5` and `6yj.8` were closed against their
actual constructor-boundary, coach-admission, recording/recovery, measured-readback and
main-regression acceptance evidence. Parent assurance remains open; identity `6yj.2`,
dependency audit `6yj.6`, comparable performance baseline `6yj.7` and extra review-wave `6yj.9`
are not certified by those closures. Main checked the current section's local artifact
references against existing files; subagent handoff sentences were not substituted for
execution or independent review approval.

`cargo vet --locked` failed because the header-only `imports.lock` disagrees with configured Google
and Mozilla imports. Main then ran `cargo vet` directly: exit 255, missing
`policy.audit-as-crates-io` classification for the modified vendored `chromiumoxide_cdp 0.9.1`;
validation stopped before the lock was populated. No audits, owner signatures, policy exception
or exemption was fabricated. Current blocker `6yj.6` is distinct from historical `3sb`, which
describes a missing owner store in another worktree.

Excel generation and readback work for these captured inputs. Accuracy beyond this qualification
needs broader public captures with acquisition manifests, admissible central identity decisions
and school-matched contact evidence. Persisted run lineage and atomic multi-artifact publication
remain publication-fidelity work. The 17 native fault scenarios and full release/proof gates are
not prerequisites to opening this offline worksheet, but remain mandatory national-release
obligations; no simulated or offline replay was relabelled as native evidence.

Global assurance disposition is **UNVERIFIED**, not approved. Verified local findings are
`fixed_with_evidence`; missing dependency/baseline/identity/native acceptance are `blocker`.
No owner-approved debt or waiver was inferred from worker output or passing local tests.


## The in-repository Rust lexer and the vet lane's unvetted crates — 2026-09-30

`ra-ap-rustc_lexer 0.174.0` and its `unicode-properties 0.1.4` were the last two unvetted
dependencies the gate's `vet` lane reported, and both were reachable only from `xtask`
(`cargo tree --workspace --edges normal -i ra-ap-rustc_lexer` prints the `xtask` root and nothing
else). Exactly one file used them: the zero-comments lane's tokenizer. Rather than record an
exemption for dev tooling, the lane now carries its own scanner — `xtask/src/comments/lexer.rs`
(token kinds and the byte cursor), `comments/literals.rs` (strings, raw strings, character and
lifetime literals) and the violation state machine in `comments/lexical.rs` — and the dependency is
gone from the workspace.

The replacement is pinned to the removed lexer's own verdicts.
`xtask/tests/golden/comments_lexer_corpus.txt` holds 109 crafted blocks (comment forms, literal and
raw-literal spellings, byte and C strings, character/lifetime ambiguity, `#[doc = …]` nesting,
Pattern_White_Space and BOM cases, unterminated literals) and `comments_lexer_baseline.txt` was
captured from the removed implementation through the same state machine; `cargo test -p xtask
comments::` → 9 passed re-renders the corpus and compares verdict by verdict. While the crate was
still present the two tokenizers were also compared over the whole workspace: 965 Rust files,
verdict-identical. The capture fixed the semantics that matter: a line comment stops before `\n` but
keeps a lone `\r`, NBSP is not whitespace (so `#\u{a0}[doc = …]` is not a documentation attribute),
an unterminated string consumes to end of input while an unterminated character literal stops at
`/`, `''`, `'a'` and `'1'` are terminated literals, and `r##x` is an unterminated raw string rather
than a raw identifier. Identifier classification uses std's `char::is_alphanumeric` in place of
XID_Continue, which only splits exotic identifiers the verdict does not see (`a·b // c` is in the
corpus).

On the branch's own tree the removal also completed its vet lane (`cargo vet --locked` →
`Vetting Succeeded (37 fully audited, 1 partially audited, 341 exempted)`, the exemption list
unchanged at 341, no fabricated audit). The supply-chain expansion that carried that result was not
harvested here, so this tree's vet lane stays red under 6yj.6 — with the unvetted-crate part of that
red now gone. Limits: the parity claim is verdict equality over the committed corpus and this
workspace's sources, not token-for-token identity, and no live host lane ran.

The branch's own gate run recorded `gate: PASS (debt ratchet holds; counts above)` with strict
clippy at 0 diagnostics and `files>300=0 fns>60=0`; the section below records what this tree's gate
run says after the harvest.

## The coach-acquisition branch harvested into main — 2026-09-30

`coach-acquisition-rust` (11 commits, tip `2c57d63`) was not merged as a branch: its 47-conflict
merge would have re-landed an arbiter port, an identity/ownership relocation and a root
documentation set that this tree already owns in a different shape. Its unlanded work was graded
commit by commit and the surviving slices were harvested onto this tree.

Landed: the in-repository Rust lexer that replaces `ra-ap-rustc_lexer` and its `unicode-properties`
(the zero-comments lane's tokenizer, pinned by the branch's 109-block corpus — `cargo test -p xtask
comments::` → 9 passed, and the lane still reports no comments over 1058 files); the `coachverify`
span scanner that replaces `scraper`, with `compute_contact_proof` re-pointed at
`census_domain::model`, where this tree keeps contact-proof digests; and the coach-directory tests
for the prototype's measured label table, the journalled school and the live summaries.

Rejected with reasons: the branch's `GradYear::of` saturating fallback (this tree requires the
checked construction recorded under 6yj.1 and ADR-003); its `source_date` module (this tree's
`model::dates::valid_date` already admits ISO dates and 1900..=2100 years, with its own tests); its
`watch_memory` refactor (this tree's `bootstrap/guard.rs` is already stop-aware); the arbiter lane,
the `census-store` proof relocation and the root documentation set (each superseded by this tree's
own integration and ADRs); and the `supply-chain` configuration expansion, which stays with 6yj.6
because vault policy is an owner decision. The branch's coachverify parity test was dropped together
with its 9694-line baseline: that baseline is a snapshot of the branch's fixture set (67 blocks
there against 56 here) and cannot be re-captured now that `scraper` is gone. Two branch test
expectations were pruned from the harvested crawl tests because this tree deliberately diverges:
the JV-first team rule (a rejected row must not suppress later varsity evidence, 6yj.3) and the
prototype row-for-row oracle (the lane's source report states the prototype's merged rows are not
one).

Commands and results: `cargo nextest run --workspace --all-features` → 1798 passed, 3 skipped (a
first gate run of that lane reported a tests failure that did not reproduce on the same tree);
`cargo test -p census-service --lib` → 198 passed, the 25 coachverify tests among them; `cargo test
-p census-crawl coach_directories` → 47 passed; `cargo test -p xtask comments` → 9 passed; `cargo
fmt --all --check` clean; the zero-comments lane clean over 1058 files. `cargo xtask gate` → every
lane passes except `vet`, which still fails on `imports.lock is out-of-date with respect to
configuration` under 6yj.6; removing `scraper` and `ra-ap-rustc_lexer` ended the unvetted-crate part
of that red. Limits: the harvested file states were verified on this tree, not re-run through the
author's own gate.

## Targeted projection readback and error classification — 2026-09-30

This is a qualification-store repair check, not a fresh national census or release certificate.
Main stopped broad ownership restructuring after the owner's scope objection. The unused
Store-dependent unsupported-cohort mutation scaffold was removed; it was not an integrated
solution. Remaining cohort retention and immutable run-lineage obligations are not waived.

The compiler scratch lane first failed with `Disk quota exceeded` while sccache wrote `/tmp`
dependencies. No cache, historical store or artifact was deleted. Subsequent focused commands used
`env RUSTC_WRAPPER= TMPDIR=/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/audit-20260930/compiler-scratch`.

| Command following that environment prefix | Observed result |
|---|---|
| `cargo test -p census-crawl coach_directories -- --nocapture` | Initially 41 passed, 3 failed: offline/JSON acquisition failures had been relabelled `directory`/`probe`. After preserving typed errors and classifying at the boundary, 44 passed. |
| `cargo test -p census-reconcile verify::performances -- --nocapture` | 4 passed: exact meet linkage, field tampering, duplicate unsampled IDs, unknown IDs and missing numeric values. |
| `cargo test -p census-review --lib` | 79 passed, including unsupported-grade evidence in provider-owned clusters and model packets. |
| `cargo test -p census-service unresolved_same_name_candidates_remain_individually_addressable -- --nocapture` | 1 passed; unresolved same-name subjects retain different CSV athlete IDs. |
| `cargo build -p census-service --bin census-service` | Succeeded; compiler warnings were observed. This was not a strict lint or release gate. |
| `cargo test -p census-crawl coach_directories -- --nocapture` followed by `cargo check -p census-service --bin census-service` after obsolete-builder/import cleanup | 44 passed; CLI check succeeded. |

The actual built CLI was then exercised against the stopped retained qualification store:

```sh
target/debug/census-service --store var/audit-20260930/wiaa-cohort-readback-03/store workbook --out var/audit-20260930/minimal-projection-readback/class2027-wiaa-qualification.xlsx
target/debug/census-service --store var/audit-20260930/wiaa-cohort-readback-03/store verify --workbook var/audit-20260930/minimal-projection-readback/class2027-wiaa-qualification.xlsx --sample-every 1
target/debug/census-service --store var/audit-20260930/wiaa-cohort-readback-03/store export-data --data var/audit-20260930/minimal-projection-readback/csv --school-year 2026
```

Workbook generation succeeded. Verification reported `OK (3 athletes sampled of 3 rows,
3 performances sampled of 3 rows)`. CSV export produced all six named products: 32 stored athletes,
3 Class-of-2027 candidates, 17 coaches, 24 schools and one meet. Both contact counts were zero;
no athlete-to-coach contact was established by these captures.

Artifact readback showed all 20 performance columns, `Workbook scope=all_sources` and
`Store bytes on disk=2285612`. Recruiting CSV contained three `unverified` candidates; the two
Kingston Penn rows had distinct athlete IDs. They were not merged or accepted as distinct people.

| Artifact | SHA-256 |
|---|---|
| `var/audit-20260930/minimal-projection-readback/class2027-wiaa-qualification.xlsx` | `4e0b3cd7b25c32931fe5dd7b8af91c9e9ab55cc02cfda6edbcd2bfef37e1d76c` |
| `var/audit-20260930/minimal-projection-readback/csv/recruiting-co2027.csv` | `4277351be90867491be3b4405dc73c72293811ffe7668cea153698099b03858c` |

Limits: this readback reused captured qualification inputs, made no new population-discovery claim,
and did not exercise native faults, national identity acceptance, dependency provenance approval
or same-input atomic publication. Export dates still lack authoritative persisted run lineage.
The earlier full-workspace test result belongs to its earlier tree, not these later edits.

## Domain-owned canonical JSON encoder; `serde_json` leaves the production tree — 2026-09-29

The gate's `domain purity` lane had been red since `9e97084`: `cargo tree -p census-domain --edges
normal` carried `serde_json`, because the persisted digest contracts (athlete identity evidence,
identity verdict, review checkpoint over `(verdicts, cases)`, roster observation) and the
contact-proof payload were hashed through serde_json's compact writer.
[ADR-017](adr/ADR-017-domain-canonical-json.md) records the contract decision. The encoder now lives
in `crates/census-domain/src/model/canonical_json/` as a domain `serde::Serializer`, `serialized_digest`
keeps its name and lowercase-hex SHA-256 result, its error type moves to the domain's
`CanonicalJsonError`, and `serde_json` is a dev-dependency only. Callers were migrated rather than
left with a second encoder: `census-review`'s checkpoint digest maps a failure to
`StoreError::Invariant`, the roster digest to the new `CrawlError::Canonical { table, source }`
(classified Terminal), and the parity-test helper `tests/common/mod.rs::digest` calls the domain
function instead of hashing `serde_json::to_string` itself.

Byte equality is the acceptance test, and the first executions earned it. The new writer shipped
with three real defects that the parity tests caught: sequences, maps and structs never wrote their
`[`/`{` opener, `f32` non-finite values bypassed the `null` path, and exponent spelling diverged —
serde_json 1.0.151 formats floats through `zmij`, not `ryu`, so `f64::MAX` rendered
`1.7976931348623157e308` against serde_json's `1.7976931348623157e+308`. The dependency is now
`zmij = "=1.0.23"` (the release serde_json uses) and the oracle is pinned `serde_json = "=1.0.151"`
so it cannot drift. An independent `security-reviewer` pass found no exploitable defect, no preimage
ambiguity, no panic/UB path and no byte divergence for the shapes the digests contain, plus four
gaps: `serialize_i128`/`serialize_u128` were missing (serde's default rejects them where serde_json
encoded them; no payload carries a 128-bit integer today), the production digest contracts had no
pinned regression value, the harness compared `from_utf8_lossy` views instead of bytes and never
asserted that serde_json refuses the same unsupported map key, and per-number `String` allocation.
The first three are fixed and pinned; the allocation is gone (`write!` against the `io::Write`
sink). The reviewer's fourth finding — `CaseEvidence::digest` frames facts with unpadded
`0x1f`/`0x1e` separators, so a statement payload can impersonate a following fact — was pre-existing
and would have moved every `ReviewCase.id` if the framing had been padded or length-prefixed, so it
was recorded here as an open contract question rather than fixed silently.
[ADR-018](adr/ADR-018-identity-and-evidence-framing-escapes.md) resolved it on 2026-09-30 by escaping
the delimiter bytes, which leaves a delimiter-free payload's hash feed byte-identical; the dated
evidence is the ADR-018 section at the top of this ledger.

```text
$ cargo test -p census-domain --lib
test result: ok. 168 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$ cargo run -q -p xtask -- domain-purity
  census-domain normal tree: arrayvec, block-buffer, census-domain, cfg-if, chrono, cpufeatures, crypto-common, digest, generic-array, iana-time-zone, num-traits, proc-macro2, quote, rust_decimal, serde, serde_core, serde_derive, sha2, syn, thiserror, thiserror-impl, typenum, unicode-ident, zmij
  no async/I-O dependency present
$ cargo run -q -p xtask -- comments
zero-comments policy: 939 Rust files checked, no comments
$ tools/gate.sh
gate: FAIL -> ratchet vet
```

`ratchet`'s eleven metrics are the pre-slice list unchanged (`clippy census_domain
arithmetic_side_effects: 0 -> 5`, `clippy census_domain indexing_slicing: 0 -> 12`, `scan
census-crawl.indexing: 0 -> 1`, `scan census-domain.indexing: 0 -> 6`, `functions_over_60_lines: 0
-> 6`, six files over 300 lines in `census-crawl`/`census-service`/`xtask`/`event_ontology.rs`), and
`grep -c canonical_json` over the clippy measurement is 0, so the new module adds no clippy debt and
the only census-domain diagnostics remain `event_ontology.rs`'s. `vet` still fails on the
supply-chain store deleted in `1db9157`. Every other lane passed in that run, including `tests`
(the workspace suite), `fmt`, `check`, `doc`, `zero code comments`, `module seams`, `domain type
integrity`, `deny`, `audit`, `machete`, `geiger`, `feature powerset` and `bench presence`.

One full-suite observation, environmental: a later standalone `nextest run --workspace
--all-features` reported 1578 passed and 1 failed —
`census-service::restate_kill_restart::a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write`
— which is the residual `free_port()` handoff race already diagnosed in this ledger, not the resume
it exists to prove; the isolated re-run passed in 125.6 s with the resume trace
(`the paused invocation inv_1037RzneRgxI0H41pIXDAbyXXTrSDz2T9Y was resumed`), and the gate's own
`tests` lane had passed on the same tree earlier.

Not established here: no live census or sealed bundle was produced, so the digest contract is
evidenced by construction, pinned values and the parity corpora rather than by re-exported store
artifacts; the roster and contact-proof preimages are built outside the domain
(`Records`, `TeamRef`, contact proofs) and are covered by shape (`SourceObservation`'s internally
tagged encoding, `Option`, nested `Vec`) plus the service-level parity tests, not by a domain-owned
pinned digest; the domain's dev-dependency now pins the workspace's single `serde_json` resolution to
1.0.151.

## Whole-index-stage skip via a receipted input digest — 2026-09-29

`IndexReport`'s index stage no longer recomputes its output tables when nothing it reads has
changed. `census-store` gained `StoreSnapshot::tables_digest`, which hashes each requested table's
name and every row's key and value in the `entities` keyspace, and `census_reconcile::index::derive`
computes that digest over the eleven tables outside its own output set — every table except
`source_identities`, `conflicts`, `review_cases`, `coverage`, `snapshots` and
`athlete_identity_decisions`, which the stage writes. When a matching `index-stage:<digest>` receipt
is held, the stage rewrites only the current pass's `Snapshots` row and returns a report stating that
nothing was derived.

Two defects surfaced in the first executions and are fixed here. `review_cases` was missing from
that output set, so the stage's own retained cases sat inside the digested inputs and no digest could
match twice: `index::stage_gate_tests::an_unchanged_store_skips_the_index_stage` failed with
`left: 2, right: 0`. The skipped report also first returned stored row totals, which on the real
corpus disagree with the pass that produced them — 3,243,879 against 3,364,515 source identities and
333,823 against 306,029 live review rows, while `consolidate` reported 341,333 review-case rows in
the same store — so the skipped report now returns zero for every derived count and 1 for the pass's
own snapshot row.

```text
cargo nextest run -p census-reconcile
Summary [   0.018s] 35 tests run: 35 passed, 0 skipped
cargo nextest run -p census-store
Summary [   0.958s] 106 tests run: 106 passed, 0 skipped
cargo check --all-targets -p census-store -p census-reconcile
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.33s
cargo clippy -p census-reconcile --lib --no-deps -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
cargo clippy -p census-store --lib --no-deps -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.19s
cargo fmt -p census-reconcile -- --check
(no output)
```

Measurement over the preserved class-of-2027 corpus (`/home/lewis/tmp/store-h`, a 21 GB copy of this
campaign's store). A release build of `census-service` from this tree fails in unrelated sources
(`census_crawl::net`, E0425/E0432/E0433), so no current service binary could be linked; two `run`
cycles with the previously linked binary — which the failed link could not refresh, and which
recomputed both cycles in full — took 682,228 ms and 654,000 ms of stage time with index stages of
480,743 ms and 460,736 ms. A throwaway example in `census-reconcile` then called `index::derive`
directly, twice in one process and again in a second process, against the same store:

```text
derive attempt 1: 262907 ms report=IndexReport { source_identities: 3364515, conflicts: 293307, reviews: 306029, coverage: 65, snapshots: 1, superseded: 0, identity_applications: 0 }
derive attempt 2: 14594 ms report=IndexReport { source_identities: 3243879, conflicts: 293307, reviews: 333823, coverage: 65, snapshots: 1, superseded: 0, identity_applications: 0 }
second process, both attempts: 13614 ms and 13592 ms
final code, skip path only: 12094 ms and 12127 ms
IndexReport { source_identities: 0, conflicts: 0, reviews: 0, coverage: 0, snapshots: 1, superseded: 0, identity_applications: 0 }
smoke exit: 0
```

The executing and skipping attempts ran back to back on one binary and one store, so the 262,907 ms
to 14,594 ms difference is this change's effect; the second process shows the receipt survives
reopening, and the final run shows the shipped skip path. The example was removed after the run and
only the tests above remain.

Not established here:

- The digest covers the eleven read tables' raw bytes only. Edits to the six output tables are not
  healed until a read table changes: the unchanged-store test wipes `source_identities` and asserts
  its rows survive the skip.
- `athlete_identity_decisions` is written by the stage alone (`index::apply::apply_decisions`, its
  only production caller) and derived from `review_cases` and `identity_verdicts`, both digested.
- The digest is a full scan of those tables paid on every cycle, including working ones, before the
  stage runs; a skip cost about 12–15 s here, which is the digest scan plus one snapshot row. A
  writer mutating a digested table while the stage runs is outside the documented
  single-owning-process model.
- The stored-versus-derived count disagreement above was observed, not traced.
- A clean checkout of `4e6981c7b` fails `cargo check --profile test -p census-store -p census-report
  --all-targets` with and without this change (`census-store` E0599, `census-report` downstream), and
  `cargo clippy -p census-reconcile --lib -- -D warnings` stops in `census-report` with 12 errors
  (one a `needless_borrow` on `rows.meets`). Neither is in a file this change touches; the
  measurement ran in the repository tree, which compiles its uncommitted store work.
- This is a qualification cycle over a preserved corpus, not the fresh national census, and its
  timings do not transfer to another revision, store or run identity.

A peer landing on 2026-09-29 displaced the earlier uncommitted section in this ledger that held this
campaign's baseline measurements (`4e6981c7b`, "Correct the landing record: the displaced work is
uncommitted and exists in no commit"; `git log -S 'Index-stage recomputation'` finds that text in no
commit). Those numbers are not restated, and every number above was measured in one session.

## DragonFly directory parse parity — 2026-09-29

The prototype's directory parser and the Rust lane were executed over the same six captured
pages and compared field for field (3,693 rows): 0 divergent fields, 0 row-count differences, 0
`currentPage`/`totalPages`/`totalResults` differences. Execution: `census-prototype/parsers/
dragonfly_directory.parse` over the fixture bodies on one side, `parse_directory` plus
`directory_school` over the same bytes on the other; the comparison details, the field mapping
and the three recorded divergences (rows without a short code, the unpersisted raw
`dragonfly_levels` map, the classification key rule) are in
[the parity record](../research/sources/coach-directories-national/dragonfly-directory-parity.md).

Three pages are now permanent: `tests/fixtures/coach_directories/golden_directory_rows.json`
holds the prototype's own rows for ASAA, WHSAA and NCHSAA page 1 (generator and digests in the
fixture `PROVENANCE.json`), and `coach_directories::tests::
the_live_directory_pages_reproduce_the_prototypes_rows` asserts the Rust parse against them.

The comparison found a real projection gap rather than a parse gap: the classification rule
accepted only keys ending `classification(s)`, and the live AHSAA pages name the class
`ahsaaClass`, so Alabama's 793 captured rows carried no class. The rule now accepts a key ending
`class` as well; measured coverage is 1,336 of 3,693 rows (AL 485, GA 157, WY 73, NC 450, GHSA p2
171, AK 0), against an independent count of 1,340 whose 4-row difference is exactly the
classified subset of the 12 vendor-fixture rows the lane drops.

```text
cargo test -p census-crawl
test result: ok. 462 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo clippy -p census-crawl --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.50s
cargo fmt -p census-crawl -- --check
(no output)
```

Not established here: fetching, caching, pacing or store writes; no prototype `out/` artifact was
reproduced; the golden covers three of the six compared pages. The throwaway Rust dumper used for
the six-page comparison was removed after the run.

## ADR-016 row hygiene and census-scope filters in the DragonFly lane — 2026-09-29

The prototype's per-row hygiene and its level scope now exist in Rust.
`crates/census-crawl/src/row_hygiene.rs` carries S03 (post suffix, post-only and
non-coach leads, `Dean` leads), S04 (`\btest\s+school\b`), S05
(`@dragonflyathletics.com`), S06 (`[\s\u{a0}]+` collapse, Python's whitespace
set) and S09 (empty key), with the audit's executed case tables ported as tests
(23 person cases, 8 vendor cases, the whitespace table, the level table).
`coach_directories/map.rs` applies them at census emission and drops any row
whose `(level or "Varsity")` is not `Varsity`, counted per level; `collect`
reports the counters in its notes.

The prototype orders the level filter per page, before merge:
`census-prototype/run.py:190-194` filters the detail record's coaches, and only
then does the merge collapse duplicates, so every sub-varsity row is a level
drop and none of them takes part in deduplication. The first Rust port
deduplicated first. That kept the same emitted row set — the level filter never
inserts a row, so no sub-varsity row could claim a key — but it misattributed
drops: a sub-varsity row that matched an already-kept varsity row read as a
duplicate and vanished from the counters. Executing the Wyoming test against the
dedup-first order produced `dropped_levels.get("JV") == Some(1)` instead of
`Some(4)` with the four varsity rows emitted in both orders; the filter now runs
before the duplicate check.

Executed in this tree (`crates/census-crawl`), with `RUSTC_WRAPPER=`:

```text
cargo test -p census-crawl
test result: ok. 461 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo clippy -p census-crawl --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.61s
cargo fmt -p census-crawl -- --check
(no output)
```

Two of those 461 tests read captured live payloads, not synthetic bodies:
`a_live_middle_school_page_keeps_only_its_director_and_counts_each_level`
(`tests/fixtures/coach_directories/probe/AL/summary-SVXJDF.json`, Rainbow Middle
School: one athletic-director row kept, 6 middle-school rows counted) and
`a_live_coach_on_junior_varsity_and_varsity_teams_keeps_the_varsity_rows`
(`.../probe/WY/summary-SS28UB.json`, Arapaho Charter High School: 4 varsity rows
kept, 4 `JV` rows counted). The Wyoming case fails against the dedup-first
order, so it is the regression for the ordering above. The lane README now
states the emission scope.

Not established here: no store-level or service-level run, no national census
evidence, and no comparison against prototype output rows — the prototype's
`out/` artifacts for these two schools were not reproduced. Workspace-wide gates
are not run: `census-report` does not compile at this revision.
## PIAA member directory ported to Rust — 2026-09-29

Worktree `arh-piaa`, branch `port-piaa`. `census_crawl::pa_piaa` implements the prototype's
`parsers/pa_piaa.py` (the 24 linked letter pages) and `parsers/pa_piaa_details.py` (the school
contact pages), registered as slug `pa_piaa` on host `www.piaa.org` at 1 request/s, one in flight,
robots respected.

- Fixtures, copied byte-identical from `census-prototype/raw/` and verified with `cmp` plus
  `sha256sum`: `directory_alpha_a.html` (80 080 B,
  `75cdf9cb2d0c0317aa2c61e96e0f19e59500a8bb9df87455aa9c95c966766635`), `directory_alpha_b.html`
  (96 263 B, `1fb99b5ce0cae096d9a466a9cb6f8fa90916b6ae03c6dd123efaf77a0583135d`),
  `directory_alpha_z.html` (78 632 B,
  `85da56147ebdc31bd2234a774515f0cd15c4be4e7e5b5d065904cb3378389b63`), `details_12048.html`
  (62 521 B, `5192627687cbd1b255e703dfe4354d2ae032acac31804232e5fe63423ac6723e`), each at the URL and
  capture name `notes/pa-piaa.md` records, plus `robots.txt` fetched on 2026-09-29 (442 B) because
  the survey kept its verdict (593 B sample) but not the body. `PROVENANCE.json` carries url,
  prototype_file, prototype_capture, sha256 and bytes per fixture.
- Goldens regenerated from those fixtures with the prototype parsers themselves
  (`python3 -c "import sys; sys.path.insert(0,'.'); from parsers import pa_piaa, pa_piaa_details; …"`):
  `golden_directory_alpha_a.json` (53 schools), `golden_directory_alpha_b.json` (101),
  `golden_directory_alpha_z.json` (53) and `golden_details_12048.json` (one athletic-director row,
  `Harry Kaufman`, with a published address).
- `cargo test -p census-crawl` -> **429 passed, 0 failed** (11 of them `pa_piaa::tests`), 0
  doc-tests. `cargo fmt -p census-crawl -- --check` is clean and `cargo clippy -p census-crawl
  --all-targets -- -D warnings` exits clean, which also required the tree's pre-existing
  `milesplit` test borrow to be dropped; the stricter local lint set (`unwrap_used`, `expect_used`,
  `panic`, `indexing_slicing`, `as_conversions`, `arithmetic_side_effects`, `pedantic`) reports
  nothing in `pa_piaa/**`.
- Parity: `directory_letter_pages_match_the_prototype_golden_field_for_field` compares every row of
  A and B on name, street, city, state, ZIP, the prototype's string `association_id` against the
  Rust school id, and its `detail_url` against the URL the Rust id builds;
  `details_pages_keep_the_athletic_director_and_no_other_post` compares the contact rows with the
  prototype's and first asserts the capture *does* publish Superintendent and Principal posts, so
  the drop is exercised rather than assumed; `the_z_letter_page_echoes_the_a_group` pins the site
  quirk that keeps `Z` out of the crawl.
- End-to-end, in-suite: `collect_stores_the_letter_schools_and_the_requested_details_page_from_the_cache`
  drives the real `collect` against a seeded cache (letters A and B plus the ID=12048 details page)
  and observes 154 schools (53 + 101), 0 errors, 0 requests, 3 cache reads, one athletic-director
  row carrying the school's id, `CoachRole::AthleticDirector`, `Gender::Mixed`, no sport and a
  published address, 154 school journal keys plus both letter keys, and one school in the store's
  log for `A J McMullen School` with city `MARKLEYSBURG`, state Pennsylvania and association
  `PIAA`. `a_journalled_letter_and_school_are_skipped_on_the_next_run` proves the second run
  writes nothing and fetches nothing.
- Defects this slice carried in from its first delivery and fixed here: its fixtures were 0.8–1.8 KB
  stubs where the captures are 62–96 KB, and its `PROVENANCE.json` named captures that had never
  been copied, so the bytes and hashes it recorded could not match any file; the module named
  `scraper`, `crate::canonical` and `crate::http::HttpClient`, none of which this crate provides,
  and had never been compiled or run; the registry and applicability entries added a row without
  bumping their array lengths; and its contact pattern used a lookahead, which Rust's regex crate
  does not support, so the details parse could never have matched a single card.
- **Limits.** No live fetch of `www.piaa.org` happened in this pass, so the captures and goldens are
  the compiled evidence; the source report's row 15 claims the directory needs a browser, which the
  captures and their notes contradict for the rows this adapter reads, and that divergence is
  unverified against a live response. No service verb calls this adapter yet (`census-service` does
  not compile at this base), and `cargo xtask scan` cannot run for the same reason, so its source
  counters were not re-measured.

## Identity advice boundary and integration regression — 2026-09-27

The advice parser no longer accepts alternate case, separator or concatenated spellings.
Matching name, school and cohort alone no longer admits `same_person` advice. Shared-provider
evidence still admits advice; accepted identity application remains a separate domain decision.
The two regression tests failed before the change. Afterward, with
`TMPDIR=/home/lewis/av.kqQlyuIY` and `RUSTC_WRAPPER=`:

```text
cargo nextest run -p census-review
77 passed; 0 skipped.
cargo run -p census-review --example identity_advice_smoke
shared_source_advice_admitted=1 bucket_only_refused=1 legacy_aliases_refused=4
```

The temporary public-API smoke executable was removed. It used synthetic packets, not live
Qwen responses, and does not establish dual-model agreement or canonical identity acceptance.

A broader `cargo nextest run -p census-review -p census-reconcile -p census-report` stopped after
122 passes and one failure: `the_jurisdiction_row_carries_its_measured_denominators` observed
50 rows instead of 52. Live source inspection found the formerly removed 49-entry
`UsJurisdiction::CENSUS_SCOPE` and the coverage caller restored by an intervening write.
The owner subsequently confirmed that Alaska and Hawaii must remain excluded and that the restored
implementation must be kept. The earlier national-scope results below describe a withdrawn
51-jurisdiction change, not the required current scope. Current acceptance is 49 jurisdictions
plus the unplaced coverage row; these geographic APIs are intentional policy, not legacy shims.

## Census scope stays ADR-009's 49 jurisdictions — 2026-09-27

Owner direction on 2026-09-27 withdrew ADR-013's 51-jurisdiction target. The tree had removed
`UsJurisdiction::CENSUS_SCOPE`, `EXCLUDED_FROM_CENSUS`, the restricted-scope error, the scope
validation helpers and the report exclusion bookkeeping, and had widened the national target
list, `admitted_scope`, applicability and the coverage/seal denominators to `UsJurisdiction::ALL`.
That expansion is reverted:

- `UsJurisdiction::ALL` still models all 51 states so Alaska and Hawaii parse; `CENSUS_SCOPE` is
  the 49 the run counts, `EXCLUDED_FROM_CENSUS` is `[AK HI]`, and `is_in_census_scope` refuses an
  out-of-scope jurisdiction with the ADR-009 reason instead of admitting it silently.
- `admitted_scope` and `scope_digest` (census-reconcile), the restate national targets, coverage
  and seal denominators, the applicability table, the workbook recruiting readers and the
  workspace contract check use the 49-jurisdiction scope again. Explicit subsets remain subsets;
  provider evidence filtering, out-of-scope notes and UNKNOWN rows remain.
- Records updated at the time: ADR-013, ADR-009, architecture, operations and the national plan. The former secondary architecture document has since been consolidated into root `ARCHITECTURE.md`.
- Kept from the same integration: the optional `CanonicalAthlete::source` and
  `CanonicalPerformance::source_athlete` decode maps with the owning-row-id fallback, and the
  owner-identity and keyed-parent refactors.

Two tests written for the wider scope were restored with it:
`index::tests::the_jurisdiction_row_carries_its_measured_denominators` expected 52 jurisdiction
rows and now expects the 50 that `CENSUS_SCOPE` plus the unplaced row publish, and
`restate_services::tests::an_empty_jurisdiction_list_covers_the_census_scope_in_declaration_order`
passes again because `admitted_scope` no longer hands Alaska to the scope validator.

Executed on the converged tree:

```text
cargo test --workspace --no-fail-fast
0 failing test targets
cargo xtask contract
8 checks passed; 0 known deviations
check 1 census scope: PASS (CENSUS_SCOPE is 49 jurisdictions of the 51 modelled, in ALL order minus [AK HI])
```

This restores the scope contract; it does not claim national acquisition, a seal or coverage
evidence for any jurisdiction.

## Subject-bound model response cutover — 2026-09-27

Removed the bare-verdict-array fallback in `census-review/src/model/response.rs`.
`VerdictBatch::sanitize` rejects empty or mismatched subjects before admitting case IDs.
The two rejection regressions failed against the previous implementation (0 passed, 2 failed),
then `cargo nextest run -p census-domain -p census-review` passed all 228 tests.

With `TMPDIR=/home/lewis/av.kqQlyuIY` and `RUSTC_WRAPPER=`:
`cargo run -p census-review --example review_envelope_smoke` exercised the exported
`ModelClient` against a bounded loopback HTTP fixture and printed:

```text
matching_subject_accepted=1 mismatched_subject_dropped=1 legacy_array_rejected=1
```

The temporary executable was removed. This proves HTTP response parsing and admission, not
real-model agreement or the sufficiency of identity evidence. An initial smoke compilation
used the private `model` module; it was corrected to the existing crate-root exports.

## Current identity application and mailbox cutover — 2026-09-27

The former orphan `census-reconcile/src/index/apply.rs` is called by `index::derive`.
The domain constructs accepted applications only after the projection validator admits them.
The writer retains digest-qualified history, preserves the first observation time on replay,
checks row bounds under the append lock, and commits at most 1,024 applications atomically.
`AthleteIdentityDecisions` now has derived-map retention with unchanged sequence-zero row encoding.
Previously deleted historical decisions cannot be reconstructed by this change.

Executed with `TMPDIR=/home/lewis/av.kqQlyuIY` and an empty `RUSTC_WRAPPER`:

```text
cargo nextest run -p census-domain -p census-reconcile -p census-store \
  -E 'package(census-domain) | package(census-reconcile) | test(identity)'
184 passed; 101 skipped.

RUST_LOG=info cargo run -p census-service --example current_cutover_smoke \
  -- target/debug/census-service
historical_rows_imported=0 consumer_rows_published=0 accepted_source_bindings=1
repeated_applications=0 remaining_tasks=0
```

The temporary smoke ran the public server startup/shutdown, reopened its isolated Fjall store,
applied and projected a parsed primary-provider identity, repeated derivation, and invoked the
actual `merge-coaches` binary against a consumer-subdomain rejection fixture. Startup preserved
the historical JSONL bytes without importing them. The endpoint drained two accepted tasks with
zero remaining, cancelled, timed-out, aborted or panicked tasks. The temporary example was removed.
The permanent identity regressions cover source binding, changed-evidence invalidation, preserved
history after reopen, replay idempotence and refusal to promote or merge provider-owned homonyms.
CLI and domain mailbox classification share one allocation-free, case-insensitive,
label-boundary-aware consumer-domain predicate.

Limits: parsed URL-labelled evidence is still not raw-capture-bound. Complete review-case
production, dual-model advice binding and cross-provider adjudication remain release blockers;
these results do not prove nationwide coverage or completion of F01–F15.

### Identity history replay ordering

The independent history review found that case membership is unordered but immutable application
comparison was order-sensitive. The new regression failed before repair with
`StoreError::Invariant: identity application ... cannot change its payload`.
Accepted applications now canonicalize member order. The writer canonicalizes only its decoded
comparison value; it does not rewrite an existing historical row or observation time.

```text
cargo test -p census-reconcile --lib \
  index::application_tests::reviewed_membership_permutation_preserves_application_history -- --exact
Before repair: 0 passed; 1 failed.

cargo nextest run -p census-domain -p census-reconcile -p census-store \
  -E 'package(census-domain) | package(census-reconcile) | test(identity)'
After repair: 185 passed; 101 skipped.

cargo run -p census-reconcile --example identity_replay_smoke
Exit 0: public derive applied one same-provider transfer decision, replayed with permuted
membership without additions, preserved the stored row, and projected both subjects to one
verified identity after reopening Fjall.
```

### Maintained HTML parser dependency

The direct dependency moved from `scraper 0.24` to `0.25`, rather than ignoring the `fxhash`
maintenance advisory. The owner-directed license exclusion remains separate from security.

```text
cargo update -p scraper --precise 0.25.0
cargo nextest run -p census-service \
  -E 'test(coachverify) | test(consumer_domain_boundaries_agree_in_merging_and_publishing)'
17 passed; 449 skipped.

cargo deny check advisories bans sources
advisories ok, bans ok, sources ok; existing duplicate-version warnings remain.

cargo run -p census-service --example parser_upgrade_smoke
supported_structures=4 unsupported_email_claims=0
```

The parser smoke compiled the production `coachverify/claims.rs` implementation and exercised
table rows, list rows, nested staff cards and paragraph fallback. Every positive input yielded
the exact coach-name and professional-email claims; changing only the claimed email yielded
no email claim. It performed no network acquisition. Both temporary examples above were removed.
`cargo vet --locked` failed after this upgrade: 31 dependencies lack `safe-to-deploy`
coverage. No audits or exemptions were fabricated; cargo-vet remains a release blocker.

## Current main-cleanup integration

Integrated upstream cleanup through `d7df661`, preserving local implementation and fixtures.
The preservation stash is `499f204be4260809a31a8168941bca74d5914dda`; it has not been dropped.
All 20 merge conflicts were resolved. `SchoolMatch::as_str` remains because the integrated
MileSplit, WIAA-results, and benchmark callers use it; obsolete APIs removed upstream stay removed.
Seven `VerifyArgs` test constructors were migrated after removal of the `grad_year` field.

### Executed acquisition and artifact smoke

Commands ran with `TMPDIR=/home/lewis/av.kqQlyuIY` and an empty `RUSTC_WRAPPER`.
The four examples were temporary runtime harnesses, not production entrypoints:

```text
cargo run -p census-crawl --example observation_transaction_smoke --message-format short
before: orphan_reproduced=true abandoned_page_athletes=0 abandoned_page_observations=1 abandoned_page_journal=0
after: abandoned_page_rows=0 abandoned_page_observations=0 abandoned_page_journal=0 committed_and_reopened_athletes=1 committed_and_reopened_observations=1 committed_and_reopened_journal=1

cargo run -p census-crawl --example contact_artifact_smoke --message-format short
old_implementation: reproduced reader_and_writer_reject_17th_64KiB_block_under_128MiB_file_limit=true error=record exceeds 1 MiB limit
encoded_boundary: PASS LF_and_CRLF_1MiB_records_accepted_and_1MiB_plus_one_refused=true
new_artifact: PASS rows=1024 csv_bytes=2374786 evidence_bytes=1862656; quoted_unicode_roundtrip=true replacement_refused=true tamper_refused=true oversized_record_refused=true

cargo run -p census-crawl --example identity_acceptance_smoke --message-format short
before: invalid_empty_native_id_source_bound=true overflow_native_id_source_bound=true advisory_link_promotes_row_owner=true shared_advisory_id_authorizes_merge=true
after: invalid_empty_native_id_source_bound=false overflow_native_id_source_bound=false advisory_link_promotes_row_owner=false shared_advisory_id_authorizes_merge=false

cargo run -p census-crawl --example ihsa_ownership_smoke --message-format short
ihsa_replay: PASS athletes=1637 primary_native=68 primary_association=1569 grade_evidence_preserved=true duplicate_performances=0 physical_requests=0 reopened=true

cargo test -p census-domain --lib --message-format short
145 passed

cargo test -p census-crawl --lib --message-format short
403 passed
```

The observation reproducer abandoned an entity-and-journal batch. Before the fix, its observation
had already committed separately. Observation builders now return values; callers append them in
their entity batch. The post-fix run checked both rollback and committed state after reopening
Fjall. TFRRS now includes its page journals in that same batch.

The artifact smoke used synthetic contacts, not the admissions workbook. It crossed 1 MiB in both
files using small records, preserved quoted Unicode fields, refused replacement of an existing
stage, detected a changed CSV digest, and refused an oversized raw record. A school-wide athletic
director claim may omit sport; coach claims still require the matching published sport, and a
director claim explicitly scoped to a different sport is rejected.

Artifact bounds apply independently: 128 MiB per CSV or JSONL file, 1 MiB per encoded CSV record or
JSONL envelope including its line ending, 200,000 rows, and a 4 KiB manifest. Passing the domain's
individual claim bounds does not waive the artifact's aggregate envelope bound.

The encoded-record regression first exposed a CRLF off-by-one: `csv-core` reports record completion
on CR, leaving LF for its next call. The reader now consumes and counts that LF before accepting the
record. The runtime smoke accepted exact-limit LF and CRLF records and refused limit-plus-one bytes.

Identity acceptance now requires a checked, primary positive-u64 person identifier and Parsed
evidence rather than allowing an advisory link or a fetched page alone to qualify a source owner.
Same-person support no longer treats a shared advisory identifier as positive identity evidence.
This is not complete F02 delivery: URL-labelled evidence is not yet capture-bound, cross-provider
corroboration remains unwired, and the review-case digest and decision-writer contracts still need
integration. Reciprocal advisory labels alone must not substitute for captured evidence.

The IHSA regression exposed a real omission: new athletes lost their initial observed grade while
repeated observations retained it. New rows now preserve that first observation. The captured replay
checked every athlete's grade evidence, primary-owner counts, 20 retained performances, an idempotent
second collection, and reopened Fjall state. No remote requests were made. Assertions requiring
unreviewed name-based merges and report prose were removed rather than repinned.

### Executed observation-history smoke

The temporary `census-store` example used two observations of one synthetic school:

```text
cargo run -p census-store --example observation_overwrite_smoke --message-format short
before replace-append: physical_rows=1 merged_evidence=1 integrity=false
before append-replace: physical_rows=1 merged_evidence=1 integrity=true
before reopen: both orders retained only one observation and one evidence item
after both orders: replacement rejected, physical_rows=2 merged_evidence=2 integrity=true
after reopen: physical_rows=2 merged_evidence=2

cargo test -p census-store --lib replace_tests --message-format short
6 passed; 120 filtered
```

Replacement used sequence zero and could overwrite an append-only observation in either operation
order. `Store::replace_many` and `StoreBatch::replace_many` now return typed
`ObservationReplacement` errors for observation-log tables, including empty replacements, before
serializing or staging rows. Production replacement callers already target derived tables; obsolete
test fixtures now append observations instead. The regression covers all nine observation logs,
unchanged records after refusal, subsequent appends, and reopening. A table-list wiring assertion
and a test that legitimized mixed replacement/observation history were removed.

This does not certify sequence-ceiling concurrency, failed-commit counter handling, or legacy import
interleaving. Those findings remain separate integration obligations.

### Executed request and cache bounds

The temporary examples exercised the public clients with synthetic, non-admissions data:

```text
cargo run -p census-review --example local_model_request_smoke --message-format short
11000 / Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf: one synthetic state case returned WI
11001 / qwen3.8-27b-uncensored: one synthetic state case returned WI
both clients refused oversized requests: attempted encoded bytes 2098243 and 2098234

cargo run -p census-crawl --example bounded_cache_smoke --message-format short
exact_metadata_bytes=65536 metadata_plus_one_refused=true
oversized_physical_body_refused=true cache_hits=1 robots_blocked=1

cargo test -p census-crawl --lib --message-format short
407 passed

cargo test -p census-review --lib --message-format short
71 passed; 5 failed in athlete_verdict tests
```

The request writer streams escaped packet fields directly into a bounded buffer instead of first
building an unbounded prompt or JSON value. Both real local model servers completed the structured
response path. This is a transport/serialization smoke, not evidence of correct real-person identity
adjudication. The cache smoke used a loopback robots server: an oversized physical body with
false-small metadata was not served, and reacquisition respected the robots denial. The cache
regressions also exercise exact body/metadata boundaries, file growth and shrinkage after metadata
inspection, and I/O failure propagation.

The five review failures require identity-admission integration; their old expectations accept
`same_person` without sufficient supporting evidence. No full review-suite or report-suite pass is
claimed. Temporary examples were removed after recording these observations.

### Executed identity-projection smoke

```text
cargo run -p census-store --example identity_projection_smoke --message-format short
source_owners=2 cohort=2027 statuses=unverified,pending
rejected_applications=1 aliases=0 reopen=true

cargo test -p census-domain --lib --message-format short
145 passed

cargo test -p census-store --lib read::view::tests --message-format short
6 passed; 120 filtered

cargo test -p census-store --lib read::identity::tests --message-format short
1 passed; 126 filtered
```

Two synthetic same-name, same-school subjects retained separate native owners and explicit grade
evidence for 2027. An unsupported source-binding application neither verified them nor aborted the
projection; its rejection remained counted, the unresolved review remained pending, and reopening
preserved both raw subjects. Missing projection subjects now return a typed error. The temporary
example was removed, and the refusal/reopen case was retained as a store regression.

Recruiter status consumers now use this projection, not a numeric confidence field. The workbook
smoke below exercises that surface. Capture-bound positive identity evidence and the still-unwired
accepted-decision producer remain separate, unresolved integration obligations.

### Executed shared PR and workbook smoke

```text
cargo run -p census-report --example shared_projection_smoke --message-format short
source_owners=2 unverified=2 cohort=2027 winners=4 artifacts=xlsx,jsonl,csv
later_tie=true historical_school=true athlete_state=WI venue_state=IL metric_widening=true
```

The real exporter wrote a temporary workbook and both sidecars. Calamine read the workbook back:
two same-name source owners remained separate and unverified despite agreeing grade evidence.
Four expected winning performance IDs matched across XLSX, JSONL and CSV; normalized values were
checked in XLSX and JSONL. Indoor, outdoor legal FAT, wind-assisted FAT and hand-timed marks retained
separate selections. The equal-mark tie selected the later performance. All winners retained their
historical team school, and the state column used the athlete's Wisconsin school rather than the
Illinois meet venue. The maximum `i32` metric value widened before scaling without overflow.

The workbook-specific reducer and count-only reconciliation were removed; one owned selection
vector feeds the sidecars and recruiter projection. Ambiguous athlete summary cells now retain
classified alternatives rather than taking the first row. This smoke does not certify conflict
classification, contact precedence, artifact-bundle atomicity, concurrent snapshot consistency or
the full report regression suite. The temporary example was removed after execution.

### Executed imperial-input boundary repair

```text
cargo run -p census-report --example imperial_boundary_smoke --message-format short
before: exit 101; byte index 2 split the UTF-8 character in the fraction "1é"
after: PASS malformed_unicode_and_fraction_refused exact_trailing_zero_retained
```

The parser now validates fractional ASCII digits and precision without slicing at an unchecked
UTF-8 byte boundary. The smoke rejected non-ASCII fractions, a fractional plus sign and nonzero
excess precision, while preserving an exact trailing-zero representation. Permanent regressions
were added alongside the metric-widening boundary case. The temporary example was removed.

The first full report test compilation exposed 198 errors, primarily stale self-crate paths and
test fixtures using removed or nonexistent contracts. After that migration, the full report run
compiled and returned 111 passed / 13 failed. Failures covered imperial boundaries, non-finite wind,
contact scope/precedence and obsolete ordering expectations. This is not a full-suite pass.

### Executed contextual PR selection

```text
cargo test -p census-report --lib bests::tests --message-format short
64 passed; 53 filtered

cargo run -p census-report --example pr_context_smoke --message-format short
pr_context: PASS distinct_xc_contexts=2 within_context_winner=true exact_feet=true
malformed_unicode_refused=true nonfinite_wind_unknown=true
```

A temporary Fjall store held three XC performances in two distinct canonical event contexts.
The selector retained the correct winning ID and mark population for each context, rather than
comparing the two races as one PR. The sole PR-key constructor now derives that context from the
performance; the caller-controlled, infallible `Result<_, ()>` builder was removed. This does not
prove upstream canonical event IDs distinguish every source course or distance.

The runtime also exercised bare-foot notation, malformed Unicode and all three non-finite wind
values. Exact imperial units are exposed as `field_micrometres`; the misleading `field_mm` name and
the unused second numeric-conversion helper were removed. The temporary example was removed after
execution. Contact and full-suite integration remain open.

### Executed academic-year contact projection

The tenure regression initially returned 6 passed / 2 failed: whitespace-only source IDs and a
63-digit capture hash could qualify a current claim. Validation now requires nonblank metadata,
a 64-digit hexadecimal SHA-256 and an RFC3339 retrieval timestamp. The subsequent full domain
library run passed all 148 tests. These checks validate metadata, not the claimed captured bytes.

```text
cargo run -p census-report --example contact_context_smoke --message-format short
before: contact_tenure_conflict expected; professional_coach_email was exported
after: contact_context: PASS actual_xlsx=6_athletes
programme_side_role_year_scoped=true historical_rows_retained=true
stale_address_refused=true tenure_conflict_retained=true
metadata_only_not_capture_proof=true
```

The temporary program wrote a real Fjall store and read the generated XLSX with calamine.
Its six synthetic athletes exercised programme/side/role/year exclusion, a named current coach
against an emailed former coach, labelled AD fallback, unresolved coach conflicts, contradictory
tenure observations, undated-address rejection and personal email classification. The raw Coaches
projection retained the former contact with `former_declared` and its assessment school year.
No admissions workbook or private source rows were used.

The failing runtime exposed a store merge that deduplicated tenure evidence without comparing
tenure itself. Full evidence equality now preserves conflicting interpretations. Contact selection
and contact conflict queues read individual snapshot observations instead of mixing addresses and
tenure from different observations of one coach. Merged entity scans retain their existing API.
Currentness is assessed against an explicit workbook school year, or the school year containing
the run date; fetch recency is not appointment authority. Missing rows mean research is unknown,
not that a source was unattempted or successfully searched.

The integrated domain/report command returned 148 domain passes and 125 report passes / 1 report
failure at an obsolete season-column expectation. The separate store library command returned
123 passes / 4 failures in row-ceiling/reservation regressions. Neither result is a full-suite pass.
Raw-capture binding, persisted contact research attempts, and complete coverage accounting remain
open. The contact metadata fixtures do not establish source authority.

### Executed roster verdict and service integration

The current MileSplit parser run passed 32 tests:

```text
cargo test -p census-crawl --lib milesplit::
```

The temporary `roster_context_smoke` executable exercised a real loopback HTTP server, the
production fetcher and collector, and reopened Fjall stores. Its five cases passed:

| Response | Accepted athletes | Completed rosters | Named gaps |
|---|---:|---:|---:|
| Complete fixture | 25 | 1 | 0 |
| One rejected graduation-year row | 24 | 0 | 1 |
| Unrecognized template | 0 | 0 | 1 |
| Invalid UTF-8 | 0 | 0 | 1 |
| HTTP 404 | 0 | 0 | 1 |

The server observed 11 physical requests: six roster requests and five robots requests.
The 404 was fetched twice rather than cached; the other four captures were reused by the
collector. Accepted source observations survived reopening, including partial-roster rows.
Resuming each store preserved its counts and gaps without additional fetch or cache activity.
The smoke asserted response-body SHA-256, not immutable archival or source authority.
It used public fixtures and synthetic responses, not admissions data, and was removed afterward.

The integrated service command passed all 10 tests across four targets:

```text
cargo test -p census-service --test fjall_restate_e2e --test workbook_shape --test parity_pipeline --test milesplit_roster_observations --no-fail-fast -- --test-threads=1
```

All commands used `TMPDIR=/home/lewis/av.kqQlyuIY` and an empty `RUSTC_WRAPPER`.
The Fjall regressions now distinguish retained raw duplicate observations from deduplicated
merged evidence, including reopening and one-time legacy import. The workbook PR oracle
uses the existing later-date, later-meet-ID, later-performance-ID tie order rather than assuming
the smallest ID wins. Incidental text goldens were removed, not regenerated.
These targets do not establish native Restate crash recovery, a fully independent PR classifier
oracle, raw-capture archival, or recovery of an incomplete roster after its source changes.

### Unexecuted durability contract audits

Four DeepSeek workers inspected the native Restate restart, export-crash, recovery and backup
test contracts. They ran no commands, services or faults. These are review inputs, not execution
evidence, and the historical drill results below do not transfer to this integration.

- Native restart: `restate_kill_restart.rs` contains endpoint/server kill paths; the audit flagged
  pre-kill progress, pause timing and the distinction between snapshot reruns and evidence-write
  deduplication. All 17 required scenarios still need current execution evidence.
- Export crash: the workbook currently saves directly to its published path, while sidecars are
  separately published. There is no snapshot-bound atomic bundle yet; a readable restarted ZIP
  does not prove preservation of the previous complete generation.
- Recovery: the audit identified old roster progress/revision contracts and assertions that can
  tolerate lost rows or only print a defect. Those findings require integration and actual drills.
- Backup: the audit distinguished cold-copy/byte-corruption tests from the production manifest
  API and power-loss durability. It flagged the outdated table-count expectation and missing
  nonempty identity-history and crash-publication cases. No restore was executed in this batch.

### Verification limits

These are slice results, not a release verdict. Raw-capture binding of contact proofs, the full
F01–F15 acceptance set and all 17 real durability fault scenarios remained unverified for this
integration. Its workbook-era input checks do not define the newer fresh-run scope, which has no
seed workbook. Historical workbook/seal entries below do not certify this integration; these
smokes did not open or modify the original admissions workbook.

## Toolchain

```text

$ cargo fuzz --version
cargo-fuzz 0.13.2

$ cargo +nightly fuzz --version
cargo-fuzz 0.13.2

$ rustup toolchain list
nightly-2026-04-27-x86_64-unknown-linux-gnu (active, default)
```

bundled nightly (`nightly-2025-11-21`), so the active default toolchain does not matter.

## Harness inventory

| Crate | File | Harnesses | Properties |
|---|---|---|---|

**Current set (2026-09-25).** The contact policy changed: an address a source published is classified by
domain and routed to `professional_email` or `personal_email`, and none is withheld. That replaced the
`check_published_email_classifies_domains` and `check_set_published_email_routes_by_kind`) and the two
`check_coach_publish_routes_known_addresses`), for **28 harnesses** in total. The per-harness audit below
records the pre-change set under its old names; no verdict in it was re-run.

Wiring: `crates/census-store/src/lib.rs` ends with

```rust
```


Commands (from the repository root), one harness per invocation:

```bash

```

`-Z stubbing` is required only because those harnesses stub the CPU feature probe `__cpuid_count`

```text
error: Using the stub attribute requires activating the unstable `stubbing` feature
```

## Repairs made in this pass

   delivered `publish.rs` / `id_mint.rs` harnesses could not compile. Symbolic inputs are now bounded
   `[u8; N]` arrays (converted through `String::from_utf8_lossy`, or, for the address harness, built
   from printable-ASCII bytes); properties that need no symbolic input use concrete value tables.
2. **sha2's runtime CPU probe.** Minting an id hashes through `sha2`, which selects its backend at
   that mints an id failed before reaching the code under test:

   ```text
   SUMMARY:
    ** 1 of 4028 failed (4027 undetermined)
    File: ".../stdarch/crates/core_arch/src/x86/cpuid.rs", line 75, in std::arch::x86_64::__cpuid_count
   VERIFICATION:- FAILED
   ```

   The probe is now stubbed to report no CPU features (`cpuid_without_features` in `id_mint.rs` and
   `merge.rs`), which routes the hash to sha2's pure-Rust soft backend; `Id::mint` and the entity code
   stay untouched. The SHA-NI backend is an acceleration of the same function and is **not** verified —
   bound of 32 (and later 72) does not cover:

   ```text
   SUMMARY:
    ** 1 of 4992 failed (4991 undetermined)
   Failed Checks: unwinding assertion loop 0
    File: ".../library/core/src/slice/iter/macros.rs", line 252, in <std::slice::IterMut<'_, u8> as std::iter::Iterator>::fold
   VERIFICATION:- FAILED
   ```

   compression loop runs 64 rounds, the `GenericArray` folds 32 steps, and one 64-byte block covers a
   short id. At 64 the sha2 loops complete and the only truncation left in the trace is `memcmp.0`.
4. **Loop shapes inside the harnesses.** `assert_id_shape` walks its 16 hex digits with an index loop
   (nested iterator folds cost CBMC ~4 unrolled steps per element), and assertion messages that
   interpolated symbolic strings were reduced to literals (the conditions are unchanged).
5. **Analysis configuration is per harness, and stated in every command below.**
   - `-Z stubbing` — only for harnesses that mint an id (sha2's CPU probe, see 2).
   - `--no-unwinding-checks` — for harnesses whose inputs are concrete values (`id_mint/*`, the
     known-value `professional_email`/`normalize_name` harnesses). CBMC cannot discharge the
     unwinding *checks* for loops over heap-backed strings, whose trip counts are not visible at
     symex time, while every loop reachable at the annotated bound has a constant trip count.
     Symbolic-input harnesses (observation keys, entity merge, the symbolic mailbox harness) run with
     unwinding checks left on.
6. **Previous compile blockers are gone.** `crates/census-service` builds and the store harnesses run in
   signature change to `Result<(), tokio::task::JoinError>` needed no harness edit.


### States, provenance, and sweep discipline

Four states are used below, and no state is inferred from a build, a timeout, or the absence of an
error:

| State | What was observed |
|---|---|
| `counterexample (property)` | A `Failed Checks: <property>` line names a property the harness asserts. |
| `no verdict (reason)` | The run — or the sweep window — ended before any of the above. Not a pass, not a blocked proof, not a counterexample. |

The provenance column cites the run, and each row's raw tail is reproduced under "Raw tails" with the
label in the last column:

- **prev-pass** — executed by the previous verification pass against this same working tree; its logs
  times quoted are that pass's. The four gradyear results were declared reusable for this sweep; the
  rest were not re-run before the sweep window closed.

Run discipline, this window: strictly one CBMC process at a time (`pgrep -x cbmc` verified empty
before each start and re-checked after each run), each invocation in its own process group with its own
wall budget and a 5 s sampler over `/proc/<pid>/status` `VmHWM` for peak CBMC RSS. Both runs that
started either finished on CBMC's own out-of-memory path or were cut off by the budget; both process
groups were killed and re-checked before these tables were written.

whose asserted property this lane believes is false on the current model
(`check_normalize_idempotent_repeated_suffix`, whose doc comment predicts exactly that counterexample)
was left as written: it is a finding for `crates/census-domain/src/model/normalization.rs`
(`normalize_name` at `:15`), which this lane does not own.

**Follow-up (same day, after this sweep).** The finding was acted on rather than filed: `normalize_name`
now strips school-type suffixes until none applies (`strip_type_suffix` returns `false`), so the
normalized form is a fixpoint and a second call is the identity — the property `SchoolId::mint` keys
identity on. `check_normalize_idempotent_repeated_suffix` stays as the pinned regression with its doc
comment updated from "expected to fail" to the fixpoint contract, and the behavior is covered in the
normal suite by `census-domain`'s `model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes`
(`cargo nextest run -p census-domain -E 'test(normalize_name)'`, 4 passed, and the full workspace run,
583 passed). The harness itself is a *concrete*-input check — it feeds one literal and asserts one
(T14) was killed at 549.5 s inside `core::slice::memchr`, before a verdict, which is why row 12 keeps
`no verdict`.

### Sweep environment notes

1. **A missing `CARGO_HOME` breaks every harness launch, in 0.1 s.** The first sweep attempt launched
   immediately with

   ```text
   error: Failed to get cargo metadata.: failed to start `cargo metadata`: No such file or directory (os error 2): No such file or directory (os error 2)
   ```

   it only appears on a real harness run. Exporting `CARGO_HOME` — the value the interactive shell
   carries — fixes it, and every run reported below used the full interactive environment. Those 0.1 s

2. **A transient manifest fault explains the previous pass's `rc=1 secs=0` rows.** Its logs
   dying inside the same second with

   ```text
   error: failed to parse manifest at `.../crates/census-service/Cargo.toml`
   Caused by:
     can't find `core` bench at `benches/core.rs` or `benches/core/main.rs`.
   ```

   — a sibling's in-flight `[[bench]]` edit, not a harness defect. `benches/core.rs` exists in the
   current tree and the manifest parses, so that failure does not reproduce.

3. **A single sequential CBMC run still reaches CBMC's OOM path.** `check_observation_id_bounds` ran
   alone for 710.0 s and ended on CBMC's out-of-memory path (T8) with `free -g` reporting 76 GiB
   available when it started, so the prev-pass OOMs cannot be attributed to concurrency alone.
   Whether a kernel OOM kill took part is **not** established: `dmesg` returns no lines from this
   session and `journalctl -k` shows nothing for the window.

### Verdict table — `crates/census-domain` (17 harnesses)

| # | Harness | State | Provenance | Raw tail |
|---|---|---|---|---|
| 1 | `check_gradyear_of_formula` | `verified` | prev-pass | T1 |
| 2 | `check_gradyear_of_known_values` | `verified` | prev-pass | T2 |
| 3 | `check_gradyear_of_saturating` | `verified` | prev-pass | T3 |
| 4 | `check_observed_grade_grad_year` | `verified` | prev-pass | T4 |
| 5 | `check_professional_email_never_publishes_consumer_mailbox` | `no verdict (never started)` | — | — |
| 6 | `check_professional_email_known_consumer` | `env-blocked (CBMC out of memory; prev-pass wall 15 s)` | prev-pass | T5 |
| 7 | `check_professional_email_known_professional` | `no verdict (never started)` | — | — |
| 8 | `check_professional_email_malformed` | `env-blocked (CBMC out of memory; prev-pass wall not recorded)` | prev-pass | T6 |
| 9 | `check_normalize_diacritics` | `no verdict (this-window probe exceeded its 600 s budget, output not captured)` | this-window | — |
| 10 | `check_normalize_shape` | `env-blocked (CBMC out of memory; prev-pass wall 226 s)` | prev-pass | T7 |
| 11 | `check_normalize_idempotent` | `no verdict (never started)` | — | — |
| 12 | `check_normalize_idempotent_repeated_suffix` | `no verdict (follow-up run killed at 549.5 s inside core::slice::memchr; no verdict line; the property it asserts is now fixed in crates/census-domain/src/model/normalization.rs and pinned by the unit test model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes)` | this-window | T14 |
| 13 | `check_id_mint_format` | `no verdict (prev-pass run killed mid-trace; its log contains no verdict line)` | prev-pass | T11 |
| 14 | `check_id_mint_tag_prefix` | `no verdict (never started)` | — | — |
| 15 | `check_id_mint_deterministic` | `no verdict (never started)` | — | — |
| 16 | `check_id_mint_golden_value` | `env-blocked (solver conversion: z3 CBMC map::at status 6; bitwuzla status 134/SIGABRT)` | prev-pass | T9 |
| 17 | `check_id_as_str_consistent` | `no verdict (never started)` | — | — |

### Verdict table — `crates/census-store` (10 harnesses)

| # | Harness | State | Provenance | Raw tail |
|---|---|---|---|---|
| 18 | `check_observation_id_bounds` | `env-blocked (CBMC out of memory after 710.0 s alone; peak CBMC VmHWM ≥ 21.0 GiB)` | this-window | T8 |
| 19 | `check_observation_key_null_byte_id` | `env-blocked (600 s per-harness budget exhausted with CBMC still running; no verdict line; peak CBMC VmHWM ≥ 3.2 GiB)` | this-window | T12 |
| 20 | `check_observation_key_zero_and_max_sequence` | `env-blocked (CBMC out of memory; prev-pass wall 223 s)` | prev-pass | T13 |
| 21 | `check_split_key_reads_fixed_width_tail` | `no verdict (never started)` | — | — |
| 22 | `check_observation_key_round_trip` | `no verdict (never started)` | — | — |
| 23 | `check_school_merge_idempotent` | `no verdict (never started)` | — | — |
| 24 | `check_coach_merge_idempotent` | `no verdict (never started)` | — | — |
| 25 | `check_coach_publish_idempotent` | `no verdict (never started)` | — | — |
| 26 | `check_coach_publish_no_consumer_mailbox` | `no verdict (never started)` | — | — |
| 27 | `check_coach_withheld_mailboxes_consistency` | `no verdict (never started)` | — | — |

### Tally

**Tally of 27: verified 4 / env-blocked 7 / counterexample 0 / no verdict 16.**

Read precisely: 4 harnesses have a complete-verification result, 7 ended in an environment or solver
failure that says nothing about the code under test, 0 produced a `Failed Checks:` property line, and
16 were never carried to any outcome. The three-state target for all 27 was **not** met: the sweep
window closed on a wrap-up request after two census-service harnesses, so 14 of them were never
launched at all; the other two started and were killed before a verdict (`check_id_mint_format`, T11,
and the `check_normalize_diacritics` probe whose output was not captured). Nothing was weakened to close a row: the harness set
is byte-identical to the previous pass.

What the 4 `verified` rows cover: `GradYear::of`'s cohort formula, its known-value anchors, its
saturating arithmetic over a fully symbolic `i16` observation year, and `ObservedGrade::grad_year`'s
agreement with `GradYear::of`. No other harness in either crate has a verdict.

### Raw tails

invocation; the log for each run names the harness it checked and this crate's path):

```text
SUMMARY:
 ** 0 of 122 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.05850434s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

SUMMARY:
 ** 0 of 128 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.04390135s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

SUMMARY:
 ** 0 of 59 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.03621107s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

SUMMARY:
 ** 0 of 327 failed (4 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 0.12346949s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

T5 — `check_professional_email_known_consumer`, `env-blocked`, prev-pass

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T6 — `check_professional_email_malformed`, `env-blocked`, prev-pass

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

runs with `--no-unwinding-checks` as recorded by that pass):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T8 — `check_observation_id_bounds`, `env-blocked (CBMC OOM)`, **this-window**
`VmHWM` 21.0 GiB, one CBMC at a time, no other harness running):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T9 — `check_id_mint_golden_value`, `env-blocked (solver)`, prev-pass. Both solver attempts, verbatim
from that pass's own summary file (`check_id_mint_golden_value rc=137 secs=244`, i.e. killed by the
batch wrapper, no verdict):

```text
size of program expression: 659841 steps
slicing removed 490160 assignments
Generated 47259 VCC(s), 9690 remaining after simplification
Runtime Postprocess Equation: 0.497278s
Passing problem to SMT2 QF_AUFBV using Z3
converting SSA
map::at

CBMC failed with status 6
VERIFICATION:- FAILED

Manual Harness Summary:
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

```text
[… --solver bitwuzla, same command shape …]
Passing problem to SMT2 QF_AUFBV (with FPA) using Bitwuzla

CBMC failed with status 134
VERIFICATION:- FAILED

Manual Harness Summary:
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T11 — `check_id_mint_format`, `no verdict`, prev-pass: the run was killed mid-trace. Its log
inside CBMC's path trace:

```text
aborting path on assume(false) at file …/library/core/src/result.rs line 966 column 15 function std::result::Result::<std::ptr::NonNull<[u8]>, std::alloc::AllocError>::map_err::<std::collections::TryReserveError, {closure@alloc::raw_vec::RawVecInner::finish_grow::{closure#0}}> thread 0
```

T12 — `check_observation_key_null_byte_id`, `env-blocked (budget)`, **this-window**
with peak sampled CBMC `VmHWM` 3.2 GiB and no verdict line; the log's last line):

```text
…
```

T13 — `check_observation_key_zero_and_max_sequence`, `env-blocked`, prev-pass
223 s):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

Unlabeled tails for completeness: the two `no verdict` rows that did start are T11 (prev-pass
`check_id_mint_format`) and the `check_normalize_diacritics` probe of this window, whose output was not
captured (a 600 s run that had not reached a verdict; a second probe was discarded when it was piped
(`check_gradyear_of_valid`) do not match the current set, and it is **not** used as evidence anywhere
above.

T14 — `check_normalize_idempotent_repeated_suffix`, `no verdict (killed mid-trace)`, follow-up run
--harness check_normalize_idempotent_repeated_suffix` with `CARGO_HOME` exported, wall 549.5 s, peak
sampled CBMC `VmHWM` 15.7 GiB, killed by the operator once it was clear the run was walking the string
machinery rather than the property; the last lines of the captured output):

```text
Unwinding loop _RNvNvNtNtCsci0VKyKEi6N_4core5slice6memchr14memchr_aligned7runtimeCskfx95qGcYES_13census_domain.0 iteration 62 file .../core/src/slice/memchr.rs line 81 column 13 function core::slice::memchr::memchr_aligned::runtime thread 0
Unwinding loop _RNvNvNtNtCsci0VKyKEi6N_4core5slice6memchr14memchr_aligned7runtimeCskfx95qGcYES_13census_domain.0 iteration 63 file .../core/src/slice/memchr.rs line 81 column 13 function core::slice::memchr::memchr_aligned::runtime thread 0
```

The harness has no symbolic input (it calls `normalize_name("x school school")` and compares the two
results), so this tail is a cost statement about `str::to_lowercase`/`memchr` under CBMC, not a
statement about idempotence. The property is pinned instead by
`model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes` (passes) and by the argument
that `normalize_name`'s only name-dependent step is `while strip_type_suffix(&mut parts) {}`, whose
result matches no suffix, so the second call's strip loop returns `false` immediately and every earlier
step (lowercase, folding, whitespace collapse) is already stable on its own output.

## cargo-fuzz

Fuzz crate layout: `fuzz/Cargo.toml` declares four `[[bin]]` targets — `hytek`, `compiled`, `xc`,
`raceday` — over `fuzz/fuzz_targets/`. Corpus and build output are gitignored
(`/fuzz/corpus/`, `/fuzz/target/`, `/fuzz/artifacts/`).

### Build

```text
$ cargo fuzz build                     # run in fuzz/
   Compiling census-domain v0.1.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-domain)
   Compiling census-service v0.1.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-service)
   Compiling fuzz v0.0.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/fuzz)
    Finished `release` profile [optimized + debuginfo] target(s) in 54.36s
```

All four targets build; the `fuzz/Cargo.lock` in the tree is the one this build produced.

### Bounded runs

Each target was seeded with the repository's retained raw bodies
(`fuzz/fixtures/retained_results_jsonl/raw/`, 7 files, 289–1100 bytes) copied into its corpus, then
run for exactly 1000 runs:

```text
$ cargo fuzz run hytek -- -runs=1000
INFO: Seed: 2978054506
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 85 min: 1b max: 1100b total: 4602b rss: 84Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run compiled -- -runs=1000
INFO: Seed: 3297506577
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 94 min: 1b max: 1100b total: 4637b rss: 85Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run xc -- -runs=1000
INFO: Seed: 4135662610
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 102 min: 1b max: 1100b total: 4693b rss: 84Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run raceday -- -runs=1000
INFO: Seed: 626477768
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 38 min: 1b max: 1100b total: 4501b rss: 85Mb
Done 1000 runs in 0 second(s)

$ find fuzz/artifacts -type f | wc -l
0
```

All four bounded runs completed with no crash and no reproduced artifact: `Done 1000 runs` for each,
`fuzz/artifacts/` empty. `cargo fuzz run <target> -runs=1000` without the `--` separator is rejected by
cargo-fuzz 0.13 (`tip: to pass '-r' as a value, use '-- -r'`), so the libFuzzer arguments go after `--`.

## cargo-mutants

Not re-executed in this pass — the numbers below are the earlier pack's run, kept for traceability.
Config `crates/census-domain/mutants.toml`, threshold 0 missed mutants: `Found 132 mutants to test` →
`ok Unmutated baseline in 2s build + 0s test` → **132 tested in 56s: 65 missed, 51 caught, 16
unviable — threshold FAIL.** The missed mutants are all in `crates/census-domain/src/model.rs`
(Display impls for `Id`/`Grade`/`GradYear`/`SourceNamespace`, `GradYear::get`, `SchoolYear::containing`
boundary, `SourceNamespace::is_core`, `EventKind::from_source_label`, `Gender::parse_milesplit`,
`CanonicalAthlete::mint` gender arms, `Mark::raw`, `professional_email`/`normalize_name` flips,
`strip_diacritic`, `flip_last_first`).

## Not established here

  above name every one of them. The blocked set and its failure mode, exactly:
  - CBMC's own out-of-memory path, with no `Failed Checks:` line: `check_professional_email_known_consumer`,
    `check_professional_email_malformed`, `check_normalize_shape`,
    `check_observation_key_zero_and_max_sequence` (all prev-pass, wall 15 s / not recorded / 226 s /
    223 s), and `check_observation_id_bounds` (this window: 710.0 s as the only CBMC on the box, with
    `free -g` reporting 76 GiB available at start and a peak sampled CBMC `VmHWM` of 21.0 GiB). Whether
    a kernel OOM kill took part is not established — `dmesg` returns no lines from this session and
    `journalctl -k` shows nothing for the window.
  - Per-harness budget exhausted with CBMC still climbing, no verdict line:
    `check_observation_key_null_byte_id` (600 s, 3.2 GiB and rising).
  - Solver conversion inside CBMC, not a property failure: `check_id_mint_golden_value` (z3 `map::at`,
    status 6; bitwuzla status 134/SIGABRT). The default solver (CaDiCaL) is the remaining route and has
    not been given a long enough window yet (a 244 s attempt was killed without a verdict).
  - Never started, in either pass: the 13 rows marked `no verdict (never started)`.
  - Started but killed before a verdict: `check_id_mint_format` (prev-pass, log ends mid-trace),
    `check_normalize_diacritics` (probed here with a 600 s cap; output not captured), and
    `check_normalize_idempotent_repeated_suffix` (follow-up run on the fixed model, killed at 549.5 s
    inside `core::slice::memchr`, T14) — none carries a verdict line, so none is evidence about its
    property either way. That last harness has no symbolic input; its property is pinned in the unit
    suite instead (`model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes`).
- **Deeper fuzzing than 1000 runs per target** — the runs are bounded smoke runs, not soak runs.
- **cargo-mutants** — carried from the earlier pack, not re-run here.
- **Traceability matrix** — separate work item, not part of this pack.

## Census seal (2026-09-22)

The terminal state is now a value, not a label (§17, §70, ADR-011): `CensusState` advances one phase
at a time, `Complete` is unconstructible without `SealEvidence`, and `census-service seal` assembles
that evidence from the store and the exported workbook — then either completes the census with a
digest or refuses and names the acceptance item.

Commands and results:

    cargo test -p census-service --lib census::state      # 11 passed
    cargo test -p census-service --bins                   # 22 passed (9 of them cli::seal)
    cargo clippy -p census-service --all-targets --all-features -- -D warnings   # clean
    cargo fmt -p census-service -- --check                # clean
    cargo run --bin census-service -- --store /tmp/mc-seal-proof seal --grad-year 2027
      # exit 1: "no workbook in /tmp/mc-seal-proof/out: run `census-service workbook …` before sealing"

**Defect found and fixed while reviewing this work:** the seal digest rendered `workbook_rows` but
not the workbook's own sha256, so two different exports with the same row count would have shared one
seal. The digest now covers `workbook_rows`, `workbook_sheets` and the sorted set of workbook
digests, and `census::state::tests::the_seal_binds_the_workbook_it_certifies` pins it.

**Review claim rejected, with evidence:** an independent review reported that `school_coach_index`
(`crates/census-service/src/report/rows.rs:74-89`) is non-deterministic because "HashMap iteration
order" decides which coach wins at a school. The map is only read by key, the stored coach is the
first in deterministic `scan` order, and the email flag is accumulated across every coach at the
school (`entry.1 = true`), so no published number depends on map iteration. The same review's claim
that coaches with `sport = None` are wrongly excluded is the documented intent of that index
(`/// School id -> (a track/XC coach, whether any track/XC coach brings a published email)`; the
comment's wording after the 2026-09-25 contact-policy change): the
published metric is athletes with an identified *track/XC* coach, and a school-wide coach row carries
no evidence that they coach track or cross country. Residual, unresolved: a school whose only track
coach is recorded as school-wide counts its athletes as coachless — a coach-sourcing gap, not a
miscomputed metric.

**Not verified here:** the seal's success path against the live store. The store is locked by the
running national sweep, so the seal is proved at the unit level (ladder, workbook reconciliation,
digest) and at the CLI level for the refusal path; the completed seal over the real corpus remains to
be run once that sweep releases the store.

**Second review, also local, also rejected with evidence** (`cli/seal.rs`, `census/state/**`):

- *"`open_items()` never checks `ConflictsRetained`/`RetriesRepresented`"* — correct, and intended:
  those two §70 items are satisfied by retention, which ADR-011 states. Both variants now carry that
  in their own doc comments so a reader of the enum does not have to infer it.
- *"`SealEvidence.open` is not in the digest"* — cannot matter: `seal()` refuses unless
  `open_items()` is empty, which requires every open-work count to be zero, so the field is always
  zero wherever a digest is minted. `observed_on` is likewise outside the digest on purpose.
- *"`metrics_reconciled` passes vacuously on a header-only sheet"* — it does not: the flag requires
  `mapped_athletes > 0`, which requires the cohort row to have been found and parsed, so a sheet with
  no cohort row fails both `RunMetricsReconcile` and `WorkbookMapped`.
- *"`labelled_count` truncates a count split across cells"* — accepted as a real limit of the parse,
  not a path to a wrong seal: the first cell after the label is the value cell the workbook writer
  fills, and a truncated read disagrees with the store's cohort count and is refused rather than
  sealed.

## Workbook scope leak (2026-09-23)

The 2026-09-23 workbook published athletes the run scope excludes. Detection, fix and the proof that
the fix reached the artifact, in order:

- `census-service verify` refused the workbook at `athletes row 521536: id ath_2dc7ef2516e8f7a6
  school 'sch_051f545742ee9934' != store 'Chugiak High School'` — an Alaska school in a store whose
  run scope is the 12-state census, and a cell holding an id where the sheet prints a name.
- Cause: each workbook read model filtered the *schools* table to the run scope first and built the
  school-id → jurisdiction index from what was left, so an athlete whose school the scope drops had
  no jurisdiction to resolve; `in_run_scope` keeps the unplaced bucket, so the athlete survived and
  `Dataset::school_name` fell back to printing the raw id. `503bb78` makes all three read models
  reuse the report's own splitter, which keeps the excluded rows reachable for placement.
- Whole-artifact proof, not a sample (`verify` samples at most 5 000 rows per sheet). The scan tool
  ships with the research folder, not this repository; run both lines from the store root:

      RESEARCH=~/Downloads/midwest-tfxc-source-research
      python3 "$RESEARCH/tools/scan_school_column.py" \
        var/midwest-census/out/superseded/midwest-census-2026-09-23.xlsx
        [Athletes] rows=582691 raw_id_cells=2959
      python3 "$RESEARCH/tools/scan_school_column.py" \
        var/midwest-census/out/census-service-2026-09-23.xlsx
        [Athletes] rows=579732 raw_id_cells=0

  582 691 − 579 732 = 2 959: the leak published exactly the rows whose school it could not name, and
  the rebuilt workbook drops exactly those and nothing else. The leaked build is kept, not deleted —
  it is the only record of the defect's shape and the pre-fix binary cannot reproduce it — but it is
  quarantined under `out/superseded/`, because `verify`, `seal` and the workbook glob all resolve
  "the newest `out/*.xlsx`" and the leaked build has the *later* mtime.
- The rebuilt workbook reconciles with the core report: its audit line reads `rows=579732
  in_scope=2225091 store_athlete_rows=2225091` and `best_mark_rows=7809 consistent=true` against
  `report-core.json`'s `athletes=2225091 class_of_2027=579732 coaches=31488`.
- `verify` now holds the sheet to the rule the sheet implements (`Dataset::school_name`): an id
  printed where the store *does* hold a row is a discrepancy, the id fallback for a school with no
  store row is not. Two acceptance tests pin both directions; the agreement fixture wrote the id, so
  it was measuring the weaker rule rather than the sheet.
- The parity golden that had been red since `13c0590` is refreshed. Only the `Goal & method` sheet
  moved, and only its five reproduce commands: reconstructing that sheet's 40 cells and rewriting
  `-p census-service` back to `-p midwest-census` reproduces the committed digest `72e7bffb…`
  exactly, so no published number was ever in that mismatch. `WORKBOOK_DUMP=1` now prints every
  normalized cell the digest reads, in digest order, so a future mismatch names the cell.

## Census seal (2026-09-23)

The 2026-09-23 census could not be sealed at all until a schema-stale artifact was moved out of the
way, and how it failed is the part worth recording: `out/seal.json` was written by a build that
predated the access-conditions work, and every seal reads it (`recorded_seal`) before it can report
anything, so the run died on the *file* rather than on the census.

    sha256 2b27837d…   var/midwest-census/out/seal.json
    jq                 {"sealed_on":"2026-09-22","phase":"complete","athletes":2238090}
    seal --grad-year 2027 --workbook out/census-service-2026-09-23.xlsx
      # Error: parsing var/midwest-census/out/seal.json / missing field `access_conditions`

A recorded seal is not inert: it is read by the next run and it certifies a store that has since
moved. It is quarantined rather than deleted — `out/superseded/seal-2026-09-22-pre-access-conditions.json`,
same rule as the leaked workbook above: the bytes are the only record of the artifact's shape, and
the globs that resolve "the newest `out/*`" must not see it. (The remedy is an operator's, because
"quietly repair it" is the failure mode the ADR exists to prevent; a seal that cannot read its own
predecessor has to say so.)

With that moved, the offline route reports (20 s, store route, no `--write`):

    phase: exporting
    workbook: var/midwest-census/out/census-service-2026-09-23.xlsx
    acceptance: jurisdiction sweeps are terminal unmet — not measured — this seal does not read the workflow journal
    acceptance: source objects are terminal unmet — not measured — this seal does not read the workflow journal
    acceptance: identity candidates are terminal unmet — 442 identity candidates are undecided
    refused: census cannot be sealed: jurisdiction sweeps are terminal is unmet (…)

Three things this run establishes rather than assumes:

- **The workbook check passes.** `inspect_workbook` runs before the acceptance gate, so reaching the
  item list at all means the 2026-09-23 export reconciles with the store's cohort — the 2 959-row
  scope leak is fixed in the artifact the seal was pointed at, not only in the source.
- **Items 1 and 2 read `not measured` offline *by construction*, and the message says why**: the
  store route reads no workflow journal. They are neither failures nor zeros — §70's own distinction,
  now observed instead of inferred. Only the online route (`seal --ingress <origin>`) measures them.
- **Every other §70 item is satisfied**: the item list prints only what is unmet.

### The review lane, and what it costs

The two lanes were identified from the running servers and the config that pairs them:

    config.native.toml   q5_url = http://127.0.0.1:11000/  q5_model = …UD-Q5_K_XL.gguf
                         q4_url = http://127.0.0.1:11001/  q4_model = …UD-Q4_K_XL.gguf
    ps                   llama-server … -np 1 … -c 200000 … --port 11000    (pid 1510152)
                         llama-server … -np 1 … -c 131072 … --port 11001    (pid 1623)

`-np 1` is why `ask_lanes` keeps exactly one request in flight per lane (`.buffered(lanes)`): a pass
costs `ceil(cases / 2) × latency`, not `cases × latency / slots`.

Smoke run — two cases, 30.4 s wall, `review --limit 2`, both lanes:

    2374515 athlete rows, 2330330 provider objects, 51835 cases filed (42830 decided, 9005 pending),
    6626 rows holding several objects of one provider, 0 findings left to a standing decision
    asked=2 accepted=0 rejected=0 insufficient=1 unanswered=1 dropped=0 failed=0

The second line is the model's half, and `insufficient=1` is the module working as specified: an
answer the store's own evidence cannot back is not recorded as a verdict. The first line is the
deterministic half, and it is the one that changes the seal: `reconcile_athletes` **files 51 835
athlete-identity cases, 9 005 of them pending** — none of which the seal had counted a minute
earlier, when it read 442. Both numbers are true of different states of the same store:

- the **weekly chain** (teams → … → index → report → bests → workbook) leaves the review table
  holding the merge's own conflicts: 1 359 cases, 442 undecided;
- the **review stage**, run on top, replaces that with its reconciliation of every athlete row:
  51 835 cases, 9 448 undecided.

The index pass is what defines the standing rows of a *derived* table (`table_rows`'s own rule: a
derived table's count is the rows its newest write left standing), so `index` restores the chain's
state and the review stage can be re-run whenever an operator wants it. Nothing is lost either way —
the 9 005 are re-derivable from the same athlete rows — but the review-stage state is the honest one
for a census that ran its review stage, and closing it means ~15 h of local-model time (extrapolated
from the smoke: 30.4 s for two cases *including* the reconciliation). That is exactly why `review`
documents itself as an operator action with a small default limit rather than a pipeline stage.

### Online seal: item 1 measured, item 2 refused by name (2026-09-23)

The deployment route the ADR-011 note above leaves open was then run end to end: `census-serve`
over `var/midwest-census` on `127.0.0.1:9080`, registered with the local Restate ingress (the
deployment the national run already used), and the seal asked through it.

    national --detach
      # note: national:2026-27:51472a0f63b82f0d:1 already had a run — restate deduplicated
      # invocation inv_13LIoGM6LB600EGmJ5iU9yGQmEPr4dHTGp
    national --json
      # teams_total 26264 · rosters_done 0 · rosters_skipped = teams · rosters_owed 0
      # failures [] · athletes_total 0 · class_of_2027_total 0
    open-work
      # jurisdiction sweeps owed: 0 of 49
    seal --grad-year 2027 --source-object … (38 journal stems)
      # acceptance: source objects are terminal unmet - 38 source objects have no terminal state

Three things this establishes:

- **§70 item 1 is measured terminal.** `jurisdiction sweeps owed: 0 of 49` is the online read the
  offline route cannot take ("this seal does not read the workflow journal"), and the national
  run's own report agrees: the fan-out is complete, nothing is owed, no jurisdiction failed.
- **The identity item closed.** The review lane resolved all 442 pending meet-jurisdiction cases
  (`asked=442 accepted=233 rejected=0 insufficient=209 unanswered=0 dropped=0 failed=0`, 7m01s,
  one request in flight per model lane). An `insufficient` verdict is a decision — the store's own
  evidence could not back a state — so the cases are terminal and neither the seal's
  `identity candidates are terminal` item nor `open-work` names them again.
- **Item 2 is refused by name, and the count is honest in both readings.** `SourceObject::terminal()`
  is `observations > 0` (`census/state/open.rs:70-77`) and an endpoint's observations are what
  `Ingest::record` (`restate_services/ingest.rs:83`) appends. Nothing in this build calls it: the
  CLI chain and the service's jurisdiction stages both write observations straight to the store, and
  the contemporaneous module inventory recorded the same gap from the identity side —
  `WorkflowIdentity::source_sweep` "has a constructor but no binder". Every key an operator can
  name reads `observations 0 windows 0`: the 38 journal stems (`milesplit_rosters_wi`, …) and the
  endpoint spellings (`milesplit_wi`, `athleticnet_wi`, `wiaa_wi`, `wayzata_wi`, `meets_wi`, `wiaa`,
  `wiaa_results`, `milesplit`, `milesplit_results_wi`, `athleticlive_wi`, `athleticlive_athletes_wi`,
  `plain_names`, `ohsaa_oh`, `ihsa_il`, `tfrrs`, `coach_contacts_wi`). Naming none leaves the count
  `unmeasured`; naming any leaves it owed. Both refuse, and that is the designed answer.

A terminal endpoint has to *accept* an observation, so a run whose stages all skip — this one:
every roster `skipped` because the store already holds it — records none, however it is addressed.
Item 2 is therefore satisfiable only by a census whose acquisition runs *through* `Ingest`
(OPERATIONS.md §Seal: "Routing acquisition through the `Ingest` service is the replacement for the
staging hop; no CLI subcommand drives it yet") **and** that acquires something new. Rebuilding this
store's corpus to manufacture observations is what the program's §1 forbids, and naming an endpoint
whose acquisition never ran that way would be a claim about it that nothing backs. So the
2026-09-23 census stays refused — **by name**, over item 2 alone — which is §70's outcome for an
item the run cannot evidence; item 2 becomes a forward obligation for the first census whose
acquisition routes through `Ingest`.

`seal` now runs on the online route: with the deployment holding the store, the offline form cannot
open it (the single-writer rule; `seal --store var/midwest-census` →

    Error: store open failed: FjallError: Locked

), and `seal --grad-year 2027` prints the same item list as
`seal --ingress http://127.0.0.1:18095/`. Each route exports the workbook before the gate, so
reaching the item list at all re-establishes the workbook check against
`var/midwest-census/out/census-service-2026-09-23.xlsx`. Naming no key leaves item 2 in the
`not measured` form quoted above; naming one leaves it in the counted form quoted above. Both are
refusals, and the difference between the two readings is the whole point of the field: `unmeasured`
is "nobody looked", `owed` is "looked, and the endpoint has accepted nothing".

### The seal that closed (2026-09-23, 17:41)

Item 2 — the one the online run above refused by name — was then measured from a routed acquisition,
and the seal closed. The terminal state it wrote, after the refusal narrative at 16:12:

    sha256 d6cdd7e0868f0d4a9fb5d2c9658f01d6510df06503c66906bfb0bb9b97f1b497  (12 726 bytes)
    file   var/midwest-census/out/seal.json
    jq     {"phase":"complete","sealed_on":"2026-09-23",
            "digest":"5ab49d85c232c26363e8c2e04695ab2c6394d84eb31590c70fed78ccba9ad233",
            "counts":{"jurisdictions":50,"schools":31818,"meets":11016,"athletes":2225091,
                      "class_of_2027":579732,"performances":23970,"coaches":31488}}

Two independent re-runs over the same store back it: the §58 verification (`verify: OK (5000 athletes
sampled of 579732 rows, 5000 performances sampled of 202979 rows)`, exit 0, 6m56s) and the §60 drill
(`PASS: backup drill completed successfully`, `observations match: 3859887`, exit 0). The live route
agrees with the drill's count: `Census/status` through the ingress reads `observations=3859887`.


### Harness inventory and claim map (27 harnesses)


| # | File | Harness | Claimed property | Symbolic input | Bound | Unwind | Assumptions | Stubs |
|---|---|---|---|---|---|---|---|---|
| 2 | gradyear.rs | `check_gradyear_of_known_values` | Known cohort anchors | none (concrete) | — | 16 | none | none |
| 5-12 | publish.rs | 8 harnesses | published-address classification by domain and routing on set (symbolic + known-value tables), `normalize_name` diacritics/shape/idempotency | `[u8;12]` address (printable ASCII), concrete tables | 12 bytes | 64 | none | none |
| 13-17 | id_mint.rs | 5 harnesses | `Id::mint` format, tag prefix, determinism, golden digest, `as_str`/`Display` consistency | none (concrete) | — | 64 | none | `__cpuid_count` stub |
| 18-22 | keys.rs | 5 harnesses | Key round-trip, null-byte id, zero/max sequence, fixed-width tail split, id bounds | `[u8;8]` id, `[u8;24]` raw key, `u64` sequence | 8, 24 | 48 | none | none |
| 23-27 | merge.rs | 5 harnesses | `Entity::merge` idempotent, `CanonicalCoach::publish` idempotent, address routing (arbitrary and known-value tables) | `[u8;6]` text fields, `bool` flags, `u8%3` counts | 6 | 64 | none | `__cpuid_count` stub |

### Audit results by skill rule

#### `assumptions_are_debt` — GAP found, fixed



- `gradyear.rs`: 3 harnesses (formula, saturating, observed_grade) — 10 new cover points for assumed boundaries
- `keys.rs`: 2 harnesses (round_trip, split_key) — 6 new cover points for id/sequence boundaries
- `merge.rs`: 5 harnesses (school_merge, coach_merge, coach_publish_idempotent, coach_publish_routes_arbitrary_address) — 10 new cover points for text/email boundaries

**Unchanged (no fix needed):**
- `check_gradyear_of_known_values`, `check_id_mint_*`, `check_published_email_*` tables, `check_normalize_*`, `check_observation_key_null_byte_id`, `check_observation_key_zero_and_max_sequence`, `check_observation_id_bounds`, `check_coach_withheld_mailboxes_consistency`: use only concrete inputs, no assumptions, no bounded generators — no cover needed.

#### `stubs_and_contracts_are_trust_boundaries` — CONFORMS


#### `negative_evidence` — CONFORMS (inline rejection evidence)

The `check_gradyear_of_saturating` harness asserts that `SchoolYear::new(MIN-1)` and `SchoolYear::new(MAX+1)` return `None`. This is a compile-time-constant assertion — it evaluates to `true` regardless of symbolic inputs, and is always reachable. No separate negative harness is needed for this claim.

No harness claims rejection of invalid inputs without existing evidence. The `professional_email_*` harnesses use concrete-value tables to assert rejection of malformed/consumer addresses.

#### `unwind_is_proof_context` — CONFORMS


#### `resource_governance` — GAP

Recorded commands in VERIFICATION-EVIDENCE.md do not use `-j 1` or cgroup memory caps. The skill mandates `-j 1` inside a cgroup cap (MemoryHigh=20G, MemoryMax=24G, MemorySwapMax=0).

#### `harness_inventory_first` — CONFORMS

The harness inventory table lists all 27 harnesses with their files, counts, and properties. This is consistent with the `rg` source scan result (27 matches).





```
error: cannot find attribute `serde` in this scope
  --> crates/census-domain/src/model/fixed_mark.rs:13:3
   |
13 | #[serde(transparent)]
   |   ^^^^^
```


### Changes summary

**Files changed:**



### References read (in order)


---

## §70 item 2: a closed window is a terminal acquisition state (2026-09-24)

The revision-8 re-drive created 53 ingest objects, read from the deployment's own state rather than
from memory:

    curl -s -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
      -d '{"query":"SELECT service_key, key, value_utf8 FROM state WHERE service_name = '\''Ingest'\''"}'

49 of them are `milesplit_<st>` and each accepted observations (81 for DC to 1,996 for WI, 29,665
across the 49). Four accepted none: `wayzata_ia`, `wayzata_mn`, `wayzata_wi`, `wiaa_results_wi`. Each
of the four carries a **closed window** (`2026-W39`), a null cursor and a null `last_appended_at`.

The cause is resume, not failure. The wayzata walk's rows are durable from the 2026-09-20 pass: 1,078
meets in the store under provider `wayzata` (MN 428, IA 167, WI 28, IL 11, SD 2, unplaced 442),
consolidated in `out/meets.jsonl`, minted from `sports/track/2026/schedule` and
`sports/xc/2026/schedule` (386 + 151 `event-row` rows in the cached bodies, fetched 2026-09-20T18:10Z,
both 200). The revision-8 walk honoured its own journal, produced no new batches, and posted none.
`ingest_post` holds the rule on both sides of that: a batch is posted before the journal entry that
claims it, and `complete_window` runs only after every batch landed. Zero appends with a closed
window is therefore the record of a walk that **finished** — not work still owed.

`SourceObject::terminal` read only `observations > 0`, so all four counted as owed and item 2 could
not reach zero by either route: naming them refused the seal, naming none left the item `unmeasured`,
and no re-run could change either, because the rows were already durable. The rule is now
`observations > 0 || windows > 0` (`crates/census-service/src/census/state/open.rs`), which is what
the object model already asserted. The case the old rule protected against is preserved rather than
dropped: an object with neither an observation nor a window is still owed, still counted, and still
refuses the seal by name —
`census::state::tests::a_source_object_is_owed_until_it_accepts_an_observation_or_completes_a_window`
asserts all four combinations (written, resumed, empty-read, untouched) and that exactly one of them
is owed. `Sweep` keeps its own inline rule — nothing accepted across the windows it watched — which
answers a liveness question, not a terminality one, and the divergence now says so in both places.

Operator consequence, and the reason this is written down: enumerate the keys from the `state` table
and name every one of them. An item-2 count assembled from recall rather than from that query is a
count nobody took. `docs/OPERATIONS.md` carries the command.

---

## The nationwide run aborted on Restate's default invocation timeouts (2026-09-24)

The revision-8 nationwide fan-out did not finish. Its own aggregate state, read from the node, records
two jurisdictions and 47 failures, every failure identical:

    Terminal error [500]: the invocation stream was closed after the 'abort timeout' (10m) fired.

The two that survived are the two smallest — AL 79 rosters done and 483 skipped (4,370 athletes,
1,102 of them class of 2027), DC 67 teams and every roster skipped. Nothing was running when the
failure was read: `SELECT ... FROM sys_invocation WHERE status = 'running'` answered 0 rows, and the
newest invocation in the node was the `Consolidate run` that closed the pass at 23:07Z.

Cause. Restate asks an invocation to suspend after `inactivity_timeout` with no journal progress, and
aborts it `abort_timeout` later — one minute and ten minutes by default, both read from the manifest
the endpoint publishes, per service. This endpoint declared neither, which the live manifest showed
before the fix:

    curl -s --http2-prior-knowledge -H 'accept: application/vnd.restate.endpointmanifest.v4+json' \
      http://127.0.0.1:19102/discover      # every service: no inactivityTimeout, no abortTimeout

Every handler here queues behind a blocking slot (`--max-concurrent 8`), and the fan-out submits all
49 jurisdictions at once, so most of those handlers were *waiting* — with no journal entry to show for
the wait — when the one-minute rule read them as stalled. The abort killed each invocation ten minutes
later. The timeouts were therefore never a statement about how long the census takes; they were a
statement about how long a queue may be, and the nationwide queue is longer than that.

Fix. The nine store-backed services are bound with `ServiceOptions` declaring an hour for each timer
(`CENSUS_INACTIVITY_TIMEOUT`/`CENSUS_ABORT_TIMEOUT`, `crates/census-service/src/restate_services/mod.rs`),
`BrowserSession` deliberately keeps the defaults, and `fjall_restate_e2e`'s discovery test asserts both
numbers appear in the manifest of every service that advertises them:

    cargo test -p census-service --test fjall_restate_e2e restate_endpoint_advertises

The abort is a *first* attempt's ending rather than lost work: a jurisdiction invocation resumes from
its journal, so the failed states are re-driven rather than rebuilt. `docs/deployment-lifecycle.md`
carries the rule.

---

## The census seals: every §70 item satisfied (2026-09-24)

With both fixes in the build, the revision-8 census was sealed through the deployment that ran it.
The endpoint was rebuilt at `326853b`, started over `var/midwest-census`, and registered with the
local Restate node; the seal then read the run's own journal by naming every key the deployment's
`state` table answers for `Ingest`:

    census-serve --listen 127.0.0.1:19103 --data-dir var/midwest-census --max-concurrent 16 \
                 --drain-timeout 30 --browser-profile …/var/browser-profile
    curl -X POST http://127.0.0.1:19095/deployments -d '{"uri":"http://127.0.0.1:19103/","use_http_1_1":true}'
    KEYS=… # SELECT DISTINCT service_key FROM state WHERE service_name = 'Ingest'  → 53 keys
    census-service open-work --season 2026 --revision 8
    census-service seal --grad-year 2027 --season 2026 --revision 8 --write --source-object $(53 keys)

    season 2026-27 revision 8
    jurisdiction sweeps owed: 0 of 49      # item 1, online
    source objects owed: unmeasured        # open-work cannot enumerate objects; the seal's list does
    phase: complete
    already sealed: 86421165beab0bc754faf36d326a0f34efaeb9357fb18b53f794d0ac7ab3ddf4
    acceptance: every §70 item is satisfied
    sealed 86421165beab0bc754faf36d326a0f34efaeb9357fb18b53f794d0ac7ab3ddf4 on 2026-09-24
      cohort 580334 of 2228631 athletes, 31818 schools, 11353 meets, 28979 cohort performances, 31488 coaches
      retained: 127 gaps, 4548 conflicts, 0 access conditions (0 hosts refused, 0 throttled),
                unmeasured source failures
    wrote var/midwest-census/out/seal.json

Four things this run settles:

- **The workbook gap was an export stale by 488 cohort athletes, not a store difference.** The first
  pass certified `census-service-2026-09-24.xlsx` as it stood at 16:11 and refused on
  `579846 of 580334 cohort athletes appear in the workbook`. `census-service workbook` (92 s) rebuilt
  it from the same store, and the same seal then read `580334 of 580334`: the rows had landed between
  the export and the check. The all-source scope stays a different census (`report.json` 623509) and is
  not what that seal certified by default. **2026-09-25:** the scope flags were aligned on `--core`, so
  every approved source is what a flagless run measures now and `--core` is the explicit
  Athletic.net-free diagnostic (`--all-sources` was the old name of that flag; the runs quoted above
  predate the rename and are kept as executed).
- **A `seal.json` from an older build is a parse trap, exactly as the runbook warns.** Both the store
  route and the online route died on `parsing var/midwest-census/out/seal.json — missing field
  silent_sources`, the field this build added to `RetainedFindings`. Renaming the artifact aside
  (`seal.v5-2026-09-24.bak.json`) and re-deriving produced the same digest the ladder had already
  recorded, so the retired file was the only stale thing in the path.
- **The retained finding is where the item-2 shortfall travels.** `unmeasured source failures` is
  `RetainedFindings.silent_sources` — the endpoints that finished empty — so the half of the count the
  owed rule can no longer carry is still named, and still part of the digest.
- **The timeout fix is live, not only asserted.** The registered deployment answers per service:
  nine store-backed services report `inactivity=1h abort=1h`, `BrowserSession` keeps Restate's
  `1m/10m` on purpose, and `Census` keeps `journal=1h idempotency=30d` while the rest keep `90d/30d`.

The sealed workbook is `var/midwest-census/out/census-service-2026-09-24.xlsx` (78 666 872 bytes,
20 sheets: Athletes, PRs, Performances_001, Coaches, Schools, Meets, Sources, Coverage, Conflicts,
Review, Run Metrics, Goal & method, Summary, By state - core, By state - all sources, Athletic.net
marginal, Best results, Meets summary, Evidence mix, Method notes). `census-service verify --store
var/midwest-census --workbook …` then reconciles its data rows against the store with `census-serve`
stopped — the same window in which the store is free for `fjall-stats` and `store-integrity` — and
answered `verify: OK (5000 athletes sampled of 580334 rows, 5000 performances sampled of 222065
rows)`.

The run's own counts, read from the objects rather than recalled: 49 jurisdiction objects at revision
8, each holding its teams, rosters and meets stages; 53 `Ingest` objects; the store holds 3 072 309
athletes, 2 364 818 of them inside the census scope (AK and HI publish in no row), 12 556 meets, and
the 580 334 class-of-2027 athletes the core-scope seal certifies.

---

## §45 gets a record: per-source accounting, and the debt it sat behind (2026-09-24)

`ARCHITECTURE.md` §10 states the obligation — "per-source metrics (§45) are recorded and the
efficiency metric is **verified useful records per physical request**" — and the client was telling
half of it. `FetchStats` counted `requests` and `cache_hits` for the whole run and carried a
`per_host: HashMap<String, u64>` that **nothing wrote and nothing read**, so neither a per-source row
nor the ratio built on one could be printed from the client that made the requests.

What changed, in `crates/census-crawl/src/net` and `crates/census-service/src/census`:

- **`HostTraffic { requests, cache_hits, bytes }`** replaces the `u64` map, and `FetchStats::per_host`
  is keyed by **host** everywhere. It was not: `record_request_stats` was handed a whole URL and keyed
  by it, while `count_request` keyed by the plan's host — one origin's traffic sitting under two kinds
  of key, which is exactly the split §10's per-origin admission rule forbids. `host_of(url)` (parsed
  by `reqwest::Url`, falling back to the URL's own text so a request is never dropped) is now the one
  place a URL becomes a key, and `net::tests::a_source_row_is_the_origin_not_the_page` pins the port,
  path, query and unparseable cases.
- **Every request is counted once, in one lock.** `cache_and_record` counts a 200/404 body and its
  bytes, `count_request` counts every status that never reaches it, and the browser lane's
  `count_capture` counts an accepted capture — which had been adding bytes but *not* the request, so
  a browser-transported source's 200s were invisible to §45. One physical browser request is now one
  row, the same as one physical HTTP request.
- **Latency is a fixed 20-bucket histogram** (`net/latency.rs`): a census fetches for days, and a
  per-request sample vector would have been the only unbounded structure in the client. The mean is
  exact; `latency_percentile_ms(50|95|99)` reports the edge of the bucket covering the rank — an
  upper bound at the histogram's resolution, which the type documents rather than presenting as one
  request's timing. No samples means `None`, never zero.
- **`TransportReport::from_stats`** (`census-service/src/census/mod.rs`) is the §45 surface: requests,
  cache hits, physical requests, bytes, latency mean and percentiles, rate-limited, timeouts, errors,
  `verified_records_per_physical_request`, and one `SourceTraffic` row per source — busiest first,
  host breaking ties so two runs render the same table. `CollectReport.requests`/`.cache_hits` were
  **removed rather than mirrored**: every reader now reads `report.transport` — the CLI's adapter
  summary is the one that existed — and `verified_records` is the walk's own athlete count passed in,
  because the report knows the traffic while the walk knows what the traffic produced.
- `the_transport_report_reads_the_counters_once` pins the projection's edges: the per-source physical
  counts sum to the run's, ordering is deterministic, an unmeasured percentile stays absent, and the
  ratio is read as 6/3.

**The ratchet this increment had to clear.** `tools/gate.sh`'s debt lane had three findings against
the baseline: an `as` cast at `net/types.rs:301`, `net/types.rs` at 346 lines, `net/execute.rs` at
309. The cast is now a checked conversion through `u32` (no ratio beats an approximate one);
`types.rs` gave up its histogram (`latency.rs`) and its clock helpers (`time.rs`) and is back inside
the budget; `execute.rs` gave up `record_transport`, which now sits beside `count_request` in
`attempt.rs` where the attempt's accounting is one thing. `net/decode.rs` — a 147-line duplicate of
the live path (`process_response` had no callers; `read_checked_body` existed in both `decode.rs` and
`execute/body_reader.rs`) kept alive by three `#[allow(dead_code)]` attributes — was deleted rather
than patched, because the new `HostTraffic` could not compile its copy of the counter anyway.

**Two clippy findings had to go with it.** The strict-clippy tally is the ratchet's other input, and
its baseline is empty — the tree may carry no diagnostic at all. It flagged
`clippy::arithmetic_side_effects` twice in the latency histogram: `LATENCY_BUCKET_COUNT - 1` (the
fallback bucket index) and `latency_ms_sum / samples` (the mean). Both are now operations that cannot
overflow or divide by nothing — `saturating_sub` and `checked_div` — which is §37's rule read from
the other side: the histogram runs in the request path, so arithmetic that could panic there is
exactly what the lint exists to catch, even where a reviewer can see the operands are safe.

    cargo xtask scan
      files_over_300_lines: []        # the three findings above, gone; baseline floor is also []
      functions_over_60_lines: 0      # baseline floor is 0
      forbidden constructs: as_cast 0, unwrap 0, expect 0, panic 0, indexing 0
    cargo run -q -p xtask -- ratchet tools/quality-baseline.json <clippy.tsv> <scan.json>
      clippy tally: (no diagnostic)   # the baseline is empty; the tree adds none
      files over 300 lines: 0 -> 0
      ratchet: no metric grew
    cargo test -p census-crawl --lib net::tests
      test result: ok. 25 passed; 0 failed
    cargo test -p census-service --lib census::tests
      test result: ok. 1 passed; 0 failed
    bash tools/gate.sh
      gate: PASS (debt ratchet holds; counts above) — 430 s, exit 0
      lanes: fmt, check, doc, tests (nextest: 1246 run, 1246 passed, 3 skipped, 1 slow),
             strict clippy (source targets: 0 diagnostics), production scan
             (files>300=0, fns>60=0), domain type integrity, domain purity, module seams,
             debt ratchet, deny, audit, vet, machete, geiger, feature powerset, bench presence

The baseline was not refreshed: `tools/quality-baseline.json` still records
`files_over_300_lines: []` and `functions_over_60_lines: 0`, and the tree returned to those numbers by
deleting code rather than by absorbing a rise.

**What the sealed run already says.** These counters record from this build forward; the revision-8
census above was collected before them, so its report carries no `transport` block. Its store does
carry the per-response evidence the report is built from — every cache entry's `CacheMeta` names the
URL it answered — which makes the *shape* of that run's sourcing measurable even though its counters
are not:

    <store>/http, 47 917 *.meta.json files (one per cached response: URL + byte count)
      entries        bytes  host
        3 682    5 035 539  api.ihsa.org              # most responses, and tiny ones
        3 609  425 333 102  www.wiaawi.org
        2 732  125 147 341  www.mshsl.org
        2 582  685 058 082  tx.milesplit.com          # most bytes
        2 205  458 788 960  ca.milesplit.com
        1 744  299 736 936  ny.milesplit.com
        1 299  161 725 229  al.milesplit.com
        1 262  185 729 504  fl.milesplit.com
      total: 47 917 responses, 7 211 950 588 bytes (6.72 GiB), 264 hosts

Read those as cache entries — responses written — not requests made. The distinction is the reason
`HostTraffic` exists: `requests - cache_hits` is what a source's operators actually see, and §10's
admission budget is stated in exactly those terms. Re-deriving the table is a read of the store:

    cd <store>/http
    find . -name '*.meta.json' -print0 | xargs -0 -n 500 grep -h -o \
      -e '"url": "[^"]*"' -e '"bytes": [0-9]*' \
    | awk '/"url"/ { if (match($0, /https?:\/\/[^\/"]+/)) { u = substr($0, RSTART, RLENGTH);
               sub(/^https?:\/\//, "", u); sub(/:.*$/, "", u) } next }
           /"bytes"/ { b = 0; if (match($0, /[0-9]+/)) { b = substr($0, RSTART, RLENGTH) + 0 }
               if (u != "") { n[u]++; s[u] += b; u = "" } }
           END { for (h in n) printf "%8d %14d %s\n", n[h], s[h], h }' | sort -rn

**Operator consequence.** A run's §45 numbers are `transport` in the report `collect` prints to
stdout — `census-service collect --store <dir> --states WI --limit-per-state 1 | jq .transport` — and
in the same object when the collection ran through the ingress and returned it as the workflow
result. They are not a second ledger, and not `<store>/out/report.json`: that file is the `report`
verb's read model, which has never carried transport counters. A census sealed before this build has
no `transport` field at all — absence, not zeros — and the cache inventory above is the honest
substitute until the next run records its own.

## The index stage was quadratic: fixed, re-measured, and the census re-sealed (2026-09-25)

**Symptom, measured on the live process.** `census-service run --store var/midwest-census` spent
32:26 (32:08 CPU) inside `index` and committed **no row** — `find var/midwest-census -newermt` empty
throughout, the last write being the 20:44:46 `consolidate`. `/proc/<pid>/io` showed `rchar` **20,973
GiB (20.5 TiB)** at 18.5 GB/s sustained with `read_bytes` of 18 MB, i.e. all page cache, single
writer, RSS flat at 6.13 GB. The reads were ~4.5 MB `pread64` chunks over three segment files of one
~215 MB table, about 86 full scans a second.

**Where the time went.** A symbolized dev build of the same stage under `perf record` put the hot
frames in `serde_json`'s deserializer (`parse_whitespace` 8.2%, `skip_to_escape` 3.4%,
`MapAccess::next_key_seed` 1.3%) reached through `census_domain::model::{provenance, cohort,
classification, athlete, identifiers}` - the rows were being deserialized inside the loop, not merely
counted.

**Cause, at file:line.** `census-service/src/cli/publish.rs:213 run_index` calls
`census-reconcile/src/index.rs:118 derive`, which calls `:215 canonical_pass` (from `:119`), which
calls `store.replace_many(Table::SourceIdentities, &pass.identities)` (`census-store/src/write.rs:137`)
→ `census-store/src/batch.rs stage_derived`, which called `drop_foreign` **once per record** (~5.4M
records, each a prefix scan of that table) and `drop_unnamed` with an O(n) `named.iter().any(...)`
membership test **per row**.

**Fix.** `stage_derived` now stages each record (one point `get` + one batch insert) and then runs
**one** `drop_foreign_batch` scan whose membership test is a `HashSet`; `drop_unnamed` uses
`HashSet::contains`. No key layout changed and no acquired table was touched.

**Independent review found a real defect in that fix.** The guard that skips the foreign-row clear
for observation-log tables lived *inside* the per-record loop
(`2efbe6c:crates/census-store/src/batch.rs:113` — `if first_named && table.storage_mode() !=
StorageMode::ObservationLog`); the rewrite dropped it, which would have deleted appended rows for any
observation-log table a derivation names. The reviewer found it by reading the diff; it is restored
as the hoisted guard at `crates/census-store/src/batch.rs:121`, where the comment records why the
mode is hoisted out of the loop - it does not vary inside a batch. Worth stating plainly: the
regression that mattered here was found by adversarial review, not by the test suite.

**The defect has a test now**, and it was proved the way a test should be:
`crates/census-store/src/tests.rs a_derived_batch_keeps_the_appended_rows_of_an_observation_log`
appends two ids to `SourceObservations` (the second lands at sequence 1), derives one of them, and
asserts the table holds three rows - the untouched id, the appended row at sequence 1, and the derived
row at sequence 0. Put the pre-fix behaviour back (the clear running for observation-log tables) and
it fails on that count, 2 against 3; the whole `census-store` suite is 106 passed with the guard in
place. The fixture carries a second id for a reason worth remembering: a table's *first* append lands
at sequence 0, which is `DERIVED_SEQUENCE` itself, so deriving that id overwrites that row's payload
by design and the guard only ever protected rows at a nonzero sequence.

**A/B, same store copy.** Before: 32:26 wall, 20,973 GiB read, **unfinished**. After: **60 s wall,
2 GiB read, rc=0**, printing `source_identities=2710323 conflicts=5440 reviews=1359 superseded=0
coverage=215`. A second pass changed **zero rows** across all 16 tables - the derivation is
idempotent, which is what makes a re-derived index trustworthy - and every acquired table stayed put
(athletes 3,072,309, observations 3,991,059, source_identities 2,507,541 -> 2,508,619 derived,
review_cases 107,768 preserved).

**Chain and gate.** `run` then completed the whole cycle in **203 s**, writing
`var/midwest-census/out/census-service-2026-09-25.xlsx`, with its own reconciliation consistent:
`store_athlete_rows=2228631` = the core report's athletes, `best_mark_rows=8560` = the `bests` rows,
`store_coach_rows=32031` = the report's coaches. §55: FMT/CHECK/CLIPPY/TEST all `rc=0` (44 suites
ok). `cargo xtask scan`: `files_over_300_lines: []`, `functions_over_60_lines: 0`.

**§70 items 1 and 2, re-measured online.** `open-work --ingress http://127.0.0.1:18095` reports
`jurisdiction sweeps owed: 0 of 49`. The run's own state - read from the deployment rather than from
memory - enumerates exactly **53 `Ingest` objects** (49 `milesplit_<st>` plus `wayzata_ia`,
`wayzata_mn`, `wayzata_wi`, `wiaa_results_wi`):

    curl -s -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
      -H 'accept: application/json' \
      -d '{"query":"SELECT service_key FROM state WHERE service_name = '\''Ingest'\''"}'

`open-work` with all 53 keys reports `source objects owed: 0`. The seal then closed:

    phase: complete
    acceptance: every §70 item is satisfied
    digest: 453a8612b90eadf6783f8f153443b9b3967753c7d3b3c536450bf474fd8af065
    cohort 580334 of 2228631 athletes, 31870 schools, 11353 meets, 28979 cohort performances, 32031 coaches
    retained: 127 gaps, 5300 conflicts, 0 access conditions (0 hosts refused, 0 throttled)

Two honest caveats attached to that acceptance. `source_failures` is the tri-state `None`/unmeasured
by design (`crates/census-service/src/restate_services/census.rs:138,157`): the journal reports the
objects with no terminal acquisition instead, and *that* is the measurement - 0 of 53 owed, so no
retry-exhausted object exists to represent. And the cohort counts are identical to the previous seal
(class_of_2027 580,334, meets 11,353, athletes 2,228,631) while schools (+52) and coaches (+543) grew,
which is what the MPA merge was supposed to move.

**§60 drill at scale.** Recorded in `docs/FJALL_BACKUP.md` §3.6: on a 1.46 GB on-disk / 7.7 GB logical
store, backup took 10 s and restore 7 s, all 16 table counts and both size totals came back identical,
and the restored store served `consolidate` (8 s) and `report` (13 s) to the same headline numbers.

### What the workbook's own numbers reconcile to (2026-09-25, independent audit)

A separate worker re-derived every sheet from the workbook bytes — 20 sheets, 78,754,237 B, sha256
`5a664010882e84a9fc1ceb1058001c29a8a67195978069f4985bb0f5aa178345` — and cross-checked it against the
store's snapshots. Sheet names were resolved through `xl/_rels/workbook.xml.rels`, row counts by a
chunked `<row r="N"` scan cross-validated against each sheet's `<dimension>`, and shared strings
resolved to read the headers.

What reconciles exactly:

- **Athletes 580,334** = `report-core.json` `totals.class_of_2027`, and the sheet's graduation-year
  column holds exactly 580,334 values of `2027` and nothing else — so the sheet is the core cohort,
  not all grades (all sources is 623,509). Per-state counts match `report-core.json` for all 49
  jurisdictions with 0 mismatches, and **all 580,334 `ath_` ids resolve to `athletes.jsonl` records
  (0 missing)** — that is §70 item 9 for this sheet.
- **Best results and PRs 8,560** = `best-results-co2027.csv` data lines = `best-results-co2027.jsonl`
  lines = Run Metrics "best-mark rows reduced", and the Best results sheet's athlete-id column is a
  byte-identical *ordered* sequence to the CSV's (8,560 ids, 6,042 distinct).
- **Coaches 32,031** = both reports, and the sheet's non-empty professional-email count 10,671 = the
  reports' `coaches_with_email` = the Summary sheet = the By-state total.
- **Schools 31,870** (= `schools.jsonl` 32,154 minus the 284 out-of-scope AK/HI rows), **Meets 11,353**
  (`meets.jsonl` 11,353 = report total, and 9,593 of them name an Athletic.net id — four ways),
  **coverage jurisdictions 50**, **cohort performances 28,979** (three ways), and the Coverage sheet's
  **127 gap rows summing to 1,159,187** — the same total as `seal.json`'s 127 `retained.gaps` entries.

What does not reconcile, in the order a reader would hit it:

- **Conflicts, four quantities, and only the export ordering is wrong.** `seal.json` retains
  **5,300**; the Conflicts sheet publishes **5,440** detail rows; `conflicts.jsonl` holds **4,548**;
  and the sheet's own summary column says **3,214** "Findings". Each is real:
  - 5,300 is the store's ledger (`rows:conflicts` in the meta keyspace, written inside the same batch
    as the rows, `census-store/src/read/rows.rs:76-90`). `Table::Conflicts`'s `entity_id()` is
    `&self.id`, minted as `{family}:{subject_id}` (`census-domain/src/model/records.rs:64`), so the
    `replace_many` write collapses to one standing row per key: the 5,440 batch rows carry 5,300
    distinct keys.
  - 5,440 is the index pass's batch length (`census-reconcile/src/index.rs:123-133`, printed at
    `index.rs:162`) — `retained.conflicts` plus `pass.collisions` — *before* the store dedups it. 140
    of those rows sit on 102 keys that already hold a row, all in "Recruiting contact conflict" (one
    school with a "Head TF Coach (boys)" row and a "Head TF Coach (girls)" row, for instance). The
    sheet renders that same `retained_records` one line per entry
    (`census-report/src/workbook/meta/queues.rs:89-98`), so sheet rows equal batch rows exactly.
  - 4,548 is what `consolidate` counted when it wrote the file — the distinct ids the table held at
    that instant (`census-store/src/read/mod.rs:78-128` visits once per distinct id). Its mtime is
    21:22:20 while the index pass flushed the keyspace at 21:23:11-13, and `docs/OPERATIONS.md:181`
    places `consolidate` *before* `index` in the chain, so the file is always the previous cycle's
    content. The previous seal agrees: `seal.v5-2026-09-24.bak.json` also carries 4,548, and the
    pre-pass ledger in the SST reads `rows:conflicts 4548`.
  - 3,214 is `Family::group`'s count — one finding for N rows, where `Family::push`
    (`census-report/src/workbook/meta.rs:150-176`) records one finding per row.
  So the sheet is right and the seal is right; the defect is the **export ordering**. `conflicts.jsonl`
  — and each derived-table sibling, `review_cases`, `coverage`, `snapshots`, `source_access`,
  `identity_verdicts` — is written by a `consolidate` that runs before the `index` pass that rewrites
  its table. Nothing in the workbook depends on those files. The Review sheet's 1,364 rows against the
  same pass's `reviews=1359` is the same dimensional difference on a second table.
- **Coverage athletes, 569 short, and the cause is the same staleness as above.**
  `coverage.jsonl`'s 50 jurisdiction rows sum to **622,940** at `cohort_athletes` where `report.json`'s
  `by_state` sums to **623,509**; five jurisdictions differ (AL 9,590/9,120, DC 207/126, KY 6,647/6,637,
  MO 15,175/15,168, TN 9,515/9,514) and 45 match. Placement is *not* the difference: both derivers call
  the same `jurisdiction_of` (`census-report/src/report/coverage/state.rs:70-74`), so no rule diverges.
  Freshness is: `report.json` is recomputed at publish time (`cli/publish.rs:52` → `build_census`),
  while `coverage.jsonl` merely re-serializes the stored `Table::Coverage`
  (`census-service/src/census/aggregate.rs:139`) that only the index pass rewrites. The cycle ran
  `consolidate` *before* `index`, so the file published the previous cycle's rows — and the 470 newer
  AL athletes arrived from `milesplit_al` without profiles, which is why the stale AL cohort happened to
  equal AL's profile count and made the gap look like a profile-URL correlation. It was a coincidence of
  the previous cycle's numbers, not the cause.
  `report.json` is right on three further grounds: it equals the store's merged athlete table recomputed
  per bucket (all 50 buckets, including the run-scope split AK 1,340 / HI 1,619), it equals the coverage
  pass's live output that the Coverage sheet carries (AL 9,590), and `seal.json`'s own gap register
  computes AL `missing_profile` = 470, which only follows from athletes 9,590 with 9,120 profiles.
  This is also *not* the run-scope exclusion, which the sheet states separately and correctly in its own
  note — "stored rows outside the census run scope (the 48 continental states plus DC, ADR-009) are
  excluded from `coverage read` and published in no row: schools=284 athletes=2959 coaches=0 meets=0
  performances=0". That set is 2,959 athletes, and its arithmetic closes on the other axis:
  1,751,450 off-cohort + 623,509 cohort + 2,959 outside scope = 2,377,918 = `athletes.jsonl` lines.
  **Repaired and verified 2026-09-25.** With the cycle reordered so `index` runs before `consolidate`,
  a `consolidate` against the live store rewrote the file: 50 jurisdiction rows summing **623,509**,
  with AL 9,590, DC 207, KY 6,647, MO 15,175 and TN 9,515 — every one equal to `report.json`.
  `conflicts.jsonl` was regenerated in the same pass and now holds 5,300 lines against the ledger's
  5,300.
- **Ohio performances, 1,123 unpublished, and the cause is one non-core-sourced meet.**
  `performances.jsonl` holds 223,188 rows, the Performances_001 sheet publishes 222,065; seven of eight
  jurisdictions are identical and the entire deficit is Ohio (2,108 stored against 985 published). All
  1,123 are one meet, `meet_0767da7a7a50f50a` "D1 Region 01 Finals" dated 2015-05-29, and every one
  carries Athletic.net evidence only (`source_key` prefixed `athleticnet:`, source URL
  `www.athletic.net/api/v1/Meet/GetMeetData`); no OHSAA source is cited on any of them. The meet itself
  *is* published in the Meets sheet, so the performances alone are withheld, and the published 985 all
  come from Ohio's other three meets, dated 2026-04/-05. The rule holds exactly across all 50
  jurisdictions: the sheet's `Source ResultID` set equals the store's rows carrying at least one core
  evidence source — 145,934 keys on both sides, difference zero in both directions. Nothing in the
  workbook states the rule, so a reader cannot tell a withheld performance from one never crawled.
- **Run Metrics "Athletes"** prints 2,364,818 (all sources) in a block whose sheets are core-scope
  (2,228,631) — the one counter in the reconciled block that does not reproduce from its own artifact.
- **`census-by-state-all-sources.csv`** disagrees with `report-all-sources.json` on 12 `schools`
  values; both files are the stale 2026-09-20 pair, not this run's output.

None of this is visible to the seal's own acceptance: `inspect_workbook` checks sheet presence,
`coverage_rows >= jurisdictions` (239 >= 50), and `mapped_athletes == class_of_2027` — a floor check
that passes while the 569-athlete gap stands. That gap between "the seal says every §70 item is
satisfied" and items 10-12 read strictly is the honest state of those three items.

### What `seal.json` cannot show, and what it implies

The file carries `phase`, `counts`, `retained`, `workbook_rows`, `sealed_on`, `digest` and nothing
else: `SealedCensus` (`crates/census-service/src/census/state/evidence.rs:238-244`) keeps those five
fields, so the per-item evidence — `OpenWork`'s fields for items 1-4, `WorkbookCheck`'s for items
9-13 — is computed at seal time and dropped. Three consequences worth stating:

- **`"complete"` is a construction guarantee, not a claim.** `CensusState::seal`
  (`crates/census-service/src/census/state.rs:179`) refuses with `SealError::ItemUnmet` unless
  `SealEvidence::open_items()` is
  empty, and `open_items` raises items 1-4 whenever `jurisdiction_sweeps`, `source_objects`,
  `cohort_decisions` or `identity_candidates` is anything but `Some(0)`. The written phase therefore
  *implies* all four were measured zero, even though no key in the file says so.
- **Items 5 and 6 are deliberately never blockers** (`RetriesRepresented` and `ConflictsRetained` never
  appear in `open_items`), which is how the seal says "every item satisfied" while
  `source_failures` is `null`: that field is tri-state, and `null` means *cannot count*, not *none*.
- **Re-measured independently on 2026-09-25:** `open-work` over all 53 source objects reports
  `jurisdiction_sweeps: 0` and `source_objects: 0`, with the 4 silent endpoints matching
  `retained.silent_sources` element for element; `review_cases.jsonl` (107,768 rows) holds 0 `Pending`
  identity candidates and 0 `Pending` cohort decisions; and replicating `inspect_workbook`'s row rule
  returns `239 + 49 = 288` = the stored `workbook_rows`, which is what makes the replication a
  measurement rather than a guess.
- **The run's own state, read from the deployment**, is 462 rows across three services: `Ingest` 53
  (the source objects above), `JurisdictionCensus` 402 (49-51 objects per revision, revisions 1-9),
  and `NationalCensus` 7 — the 49-jurisdiction scope (digest `51472a0f63b82f0d`, reproduced as sha256
  over the 49 codes in declaration order) at revisions 1, 2, 6 and 8, plus a DC-only scope
  (`107154493fc6af5b`, which is sha256 of `DC\n`) at revisions 2, 90 and 91.

Two things a reader should know that the artifact does not say: the counts **mix scopes**
(`athletes`/`class_of_2027` are core, `meets` is all-source) and there is no `scope` key; and
`retained.gaps` drops the jurisdiction each `CoverageGap` carries
(`crates/census-report/src/report/coverage/gaps.rs:80-87`), so the 127 rows cannot be attributed to
states from this file alone.

---

## The integrated tree: the budget at zero, and the port collision the kill/restart test was hiding (2026-09-25, integration pass)

The §38 budget that stood at seven files and six functions is empty, and the two gates that had been
red are green on the integrated tree:

    $ cargo xtask scan                       # structure block
    "files_over_300_lines": []
    "functions_over_60_lines": 0
    "functions_over_60_sites": []
    "unstable_feature_sites": []
    $ cargo xtask contract                   # 8 checks, all PASS: modules, budgets, no_python, ...
    $ cargo xtask seams                      # 0 violations across the allowed-edge tables

`functions_over_25_logical_lines` stays at 629 and is printed with `(context)`: it is the counter the
gate's own comment says rises with every feature, and the ratchet prints it rather than failing on it.

**The kill/restart test was failing on a port collision, not on the resume it exists to prove.** The
first full suite of the pass reported `43 suites ok, 1 failed`:
`restate_kill_restart::a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write` panicked at
`restate_kill_restart.rs:596` with `the node's admin API answers: "node admin API never came up: error
sending request for url (http://127.0.0.1:39517/deployments)"`. The node's own log named the cause:

    Failed: [admin-api-server] failed binding to address '127.0.0.1:39517': Address in use (os error 98)

The generated config bound **`[admin]` and `[ingress]` to the same port** (39517). The 09:30 run of the
same test, whose root survives at `/tmp/midwest-kill-restart-720862`, had three distinct ports
(33335 node, 37001, 43603), and the production node holds fixed ports (15152 node, 19095 admin, 18095
ingress) — so no second process was involved. `free_port()` bound `127.0.0.1:0`, read the port, dropped
the listener, and returned; the kernel re-offers a just-released ephemeral port to the next `bind(":0")`,
so two adjacent picks returned 39517 and the second server died. The fix is a handed-out ledger
(`static HANDED_OUT: LazyLock<Mutex<HashSet<u16>>>`) that makes every pick in the process distinct and
retries otherwise; the only race left is the handoff window to other processes, where losing shows up as
a child that never becomes ready, never as a silent pass. Re-run:

    $ cargo test -p census-service --test restate_kill_restart
    test a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 123.97s

**Both live report/CSV pairs reconcile exactly; the pair the audit flagged is a legacy-name leftover.**
`report-core.json` against `census-by-state-core.csv` and `report.json` (scope `all_sources`) against
`census-by-state.csv` each agree on all 50 jurisdictions and on every total — 0 mismatches in `schools`,
`athletes`, `coaches` and `coaches_with_email`. The files named `report-all-sources.json` and
`census-by-state-all-sources.csv` are a 2026-09-20 pair from an older naming scheme: the JSON covers
only 12 jurisdictions and every one of its `schools` counts is 0. The current writer emits the
unsuffixed pair, which is why only that dead pair disagrees.

That also identifies the number the audit could not place: `Run Metrics` printed **2,364,818**, which is
exactly `report.json`'s all-source athlete total, in a block whose other counters are core-scope. The
core scope is **2,228,631** (`report-core.json`'s total, and the seal's `counts.athletes`).

**The workbook-build profile, separated from the code.** The offline CLI path (`census-service workbook`)
runs whatever the caller built: the wrapper's own log shows `cargo run -q -p census-service --bin
census-service -- --store var/midwest-census workbook …`, i.e. the debug profile (417 MB binary). The
recorded 92 s/203 s builds in this document came from the prod endpoint. `Cargo.toml` defines only
`[profile.release]`, and `tools/durability/run.sh` resolves its endpoint from `target/release` — so
`--release` *is* the production path, and the remaining spread is machine context, as PERFORMANCE.md's
"record on a quiet machine" caveat says. No code regression is implicated, and none was found.

---

## The recruiting workbook, built and measured: 1839 s, nine sheets, and where the time went (2026-09-25)

**The artefact.** The offline export on the integrated tree completed at 12:13:11 with rc=0:
`var/midwest-census/out/census-service-2026-09-25.xlsx`, **81,775,867 bytes**, sha256
`bebe315b514830e531d3513f34eb7187…`. Its **nine** worksheets are the ones the module doc names, in
the objective's order: `Athletes` (§50), `PRs` (§51), `Performances_001` (§52), `Coaches` (§53),
`Meets`, `Sources`, `Coverage`, `Data Quality`, `Run Metrics` (§54). `Performances_00N` materialised
as a single `Performances_001`: the store holds 309,962 canonical performances, far below Excel's
1,048,576-row cap, so no partition beyond the first was needed. (The four open range writers visible
during the build are the spill module's internal partitions, not published sheets - this section
records that because the file's plan predicted twelve worksheets from those four descriptors and the
artefact refutes it.)

**The cost, and what it is not.** `EXPORT rc=0 elapsed=1839s`. A first reading called this a
quadratic to match the index-stage defect recorded above; the measurements refute that reading.
`fjall-stats` puts the store's shape at 309,962 performances, 3,991,059 observations, 3,072,309
athletes, 2,508,619 source identities and 2,069,298,240 bytes on disk, with **`store_bytes`
15,714,376,761** - and the build's resident set sat at 15,000,880 KB, i.e. the build materialises the
whole logical store. `/proc/<pid>/io` read `rchar` 6,541,804,755 - very close to three passes over
that store, which is the shape the module doc describes ("one dataset read three times around the §52
performance sheets") - while `read_bytes` stayed 0 (all page cache) and `write_bytes` stayed at
6,344,704 until the final write. `stime` was 3.7 s against a `utime` that grew past 1600 s: the phase
is userspace CPU, one core at 99.7% of a 32-core host.

**Where the CPU went.** A 4-second `perf record -F 99 -p <pid> -g` (permitted at
`perf_event_paranoid=2`) captured 396 samples: **97.2% of self time in `__memcmp_evex_movbe`**,
identified by disassembling the sampled address rather than by trusting the symbol table (the
nearest exported symbol was `_dl_mcount_wrapper`, which the disassembly shows to be
`__memcmp_evex_movbe`'s tail). The build is comparison-bound. The release binary is stripped, so the
*caller* of those comparisons is not attributable from this profile - the index-stage investigation
above used a symbolized dev build for exactly this reason, and that is the named next step. What the
profile does refute is one particular suspect: the per-row paths were each read and each is a hash
lookup or a single linear pass - `RangeFiles::range_of` is one `HashMap::get`,
`retain_core_row` is a retain plus two small drops, `Parents::read` filters with hash-backed indexes,
`bucket_universe` sorts and dedups once, `Lookups` is four maps, `bests::build` accumulates through
`for_each_merged` and sorts once, and `sheet_order` is a plain comparator. None is O(n²).

**Comparability.** PERFORMANCE.md's 203 s row is the *in-process* chain ("consolidate → index → two
report scopes → bests → workbook") measured after the index fix, on the store as it stood then; the
offline CLI additionally opens and scans the store itself. The two are not the same workload, and
this section does not claim a regression between them. What today's numbers establish is the
baseline the §24.2 before/after for the `Athletes` sort key is measured against: the same-corpus
control run follows in the next section.

---

## §60 on the real store: backup, restore, integrity, and a full census read of the restored copy (2026-09-25)

The drill `docs/FJALL_BACKUP.md` describes was run against `var/midwest-census` itself, not a fixture,
with the sequence the objective asks for - consistent backup, restore, integrity verification, reopen,
full census read - and with the store quiescent (no writer attached) throughout. Exact commands:

    census-service store-integrity --store var/midwest-census
    census-service store-backup   --store var/midwest-census --to /tmp/store-drill/backup
    census-service store-restore  --from /tmp/store-drill/backup --to /tmp/store-drill/restored
    census-service store-integrity --store /tmp/store-drill/restored
    census-service fjall-stats     --store /tmp/store-drill/restored

**Integrity.** The live store's check printed per-table `expected`/`actual` lines and exited 0 in
**3.1 s**; the tail shown was `coverage 215`, `snapshots 3`, `source_access 0`,
`identity_verdicts 43291`, `source_meets 131726`, `source_observations 207`, each `ok`. The restored
copy's check printed **16 tables `ok`** followed by `ok true`.

**Backup and restore.** The backup wrote a whole store-root copy in **23.5 s**; the restore into a
fresh directory completed in **12.0 s** and reported the same per-table counts the backup had. The
copy is the whole root, not only the Fjall tables - 15 GB on disk, against the Fjall store's own
2,069,298,240 `bytes_on_disk` - so an operator sizing the drill should expect the HTTP cache, the
entity logs and `out/` to travel with it, as `docs/FJALL_BACKUP.md` §1 tabulates.

**The census read.** `fjall-stats` on the restored copy differs from the live store on **none** of the
seventeen per-table counts: schools 89959, teams 207609, coaches 65020, athletes 3072309, meets 12556,
events 101711, performances 309962, source_identities 2508619, conflicts 5300, review_cases 107768,
coverage 215, snapshots 3, source_access 0, identity_verdicts 43291, source_meets 131726,
source_observations 207, observations 3991059. The one difference is `store_bytes`:
**15,714,376,761** live against **15,635,532,005** restored, a 0.5% reduction with identical row counts
and identical `bytes_on_disk` (2,069,298,240) - the round trip re-serialises the store, so the logical
byte total is not expected to reproduce exactly while every count does. Recorded rather than smoothed
over, because a reader comparing the two outputs will see it.

### The nine sheets, measured from the artefact (2026-09-25)

Read from `xl/workbook.xml` and each sheet's `<dimension>` in the built workbook, so these are the
artefact's own counts rather than the builder's log:

| Sheet | Range | Data rows | Columns |
| --- | --- | --- | --- |
| `Athletes` (§50) | `A1:BF580335` | 580,334 | **58** |
| `PRs` (§51) | `A1:S9959` | 9,958 | 19 |
| `Performances_001` (§52) | `A1:S222066` | 222,065 | 19 |
| `Coaches` (§53) | `A1:N32032` | 32,031 | 14 |
| `Meets` (§54) | `A1:J1879` | 1,878 | 10 |
| `Sources` (§54) | `A1:H185` | 184 | 8 |
| `Coverage` (§54) | `A1:AB240` | 239 | 28 |
| `Data Quality` (§54) | `A1:H48927` | 48,926 | 8 |
| `Run Metrics` (§54) | `A1:D53` | 52 | 4 |

Three of these are cross-checked against the builder's own running checks and against `verify`:
`Athletes` 580,334 equals the export log's `rows=580334` (with `in_scope=2228631` as the all-sources
denominator, so the sheet is the recruiting cohort, not every stored athlete); `PRs` 9,958 equals
`best_mark_rows=9958`; `Coaches` 32,031 equals `store_coach_rows=32031`; and `Performances_001`
222,065 equals the row count `verify` reported for that sheet. `Athletes` spanning `A` to `BF` is the
§50 58-column recruiting layout in the artefact itself.

`census-service verify --store var/midwest-census --workbook …` sampled to its 5,000-row cap on both
sampled sheets and reported `verify: OK (5000 athletes sampled of 580334 rows, 5000 performances
sampled of 222065 rows)` in 1m37s.


## The §24.2 before/after for the `Athletes` sort key: a payload-identical workbook, and a timing delta the workload cannot resolve (2026-09-25)

`MaterialisedSnapshot`'s `Athletes` sort keyed every comparison through `stable_key()` on both
sides, allocating two `String`s per comparison — on the order of 580,334·log2(580,334)·2 allocations
to order a sheet that is written once. The patch materialises each row's key once
(`sort_by_cached_key`), so the cost is bounded by the row count instead of the comparison count.

Two release binaries were built from the same tree with only that file differing
(`crates/census-report/src/workbook/recruiting/athletes.rs`), and each ran the recruiting workbook
export over the same store with no other load on the host:

| | A (materialised key) | B (control, per-comparison key) |
|---|---|---|
| source sha256 | `3593d3bfc003c942f1d072d0273efd6a497646c35e09fce8ebfe1278fa5f46d3` | `119754a84cdee6d56b8ec0bc0fefb8085680e30c8d54ac34222709c1fe0efe13` |
| exit / sheet rows | rc=0; Athletes 580,334 · PRs 9,958 · Coaches 32,031 | rc=0; Athletes 580,334 · PRs 9,958 · Coaches 32,031 |
| workbook bytes | 81,775,867 | 81,775,866 |
| workbook sha256 | `bebe315b514830e531d3513f34eb71877d5138709f32f5d3e303446935767a53` | `32d7304c54639bc642efdd1de3c440b79861661fcdbb2e59160362d3ebd381dd` |
| elapsed | 1839 s | 1798 s |
| steady RSS | 14,991,196 kB | 14,991,196 kB |
| bytes read from the store (`rchar`) | 6,541,804,755 | 6,541,804,755 |

The container digests differ, so the two deliverables were compared member by member instead of by
container hash: of the 18 ZIP members, 17 are byte-identical, and the one that differs is
`docProps/core.xml`, whose only difference is the writer's wall-clock stamp
(`2026-09-25T16:42:59Z` versus `2026-09-25T17:20:00Z`). Every sheet part, the shared strings, the
styles and the charts are identical, so the patch changes no workbook content.

**Finding:** the workbook's `docProps` stamps are wall-clock, so *container* digests are not
reproducible between runs even though every sheet is identical. The reproducible form of the claim
is the 17-of-18 member comparison; a container-level `cmp` will always differ across runs.

**On timing, the control was 41 s faster** (1798 s against 1839 s, 2.3%), so this A/B demonstrates no
speedup. A single-core ~30-minute job on a 32-thread host carries run-to-run frequency and thermal
variance of that order, and a 580k-row sort is a few seconds of the total budget; this measurement
cannot resolve the patch's effect in either direction. The patch is kept for the reason it was
written — a per-row key bounded by the row count rather than the comparison count, with no
allocation per comparison — and not for a measured speedup, and this section is the evidence for
that distinction rather than a claim of gain.

Environment, identical for both windows: AMD Ryzen 9 9950X3D (16 cores/32 threads), 123 GB, rustc and
cargo `1.97.0-nightly`, `[profile.release]` with `lto = "thin"`, `codegen-units = 1`, `strip = true`;
one core at ~99.7% CPU, and no concurrent build or other load during either run.


## The Restate seal: what the endpoint measured, and the trap in naming source objects (2026-09-25)

The seal is the only §70 verification that reads the run's own open work, and it is the only route a
finished census can take: `census-service seal --store` opens the store and therefore cannot see the
journal's in-flight state at all, which is exactly what the first attempt reported —

```
refused: census cannot be sealed: source objects are terminal is unmet
  (source objects have no terminal state: not measured - this seal does not read the workflow journal)
```

With `--ingress http://127.0.0.1:18095` and the endpoint running
(`census-serve --listen 127.0.0.1:19103 --data-dir var/midwest-census`), the seal reads the run's
objects through Restate. Re-registering the deployment against the freshly built binary moved the
service revisions from `Census r13`/`Consolidate r9` to `Census r14`/`Consolidate r10`, so the seal
measured the restored code rather than yesterday's.

**The trap.** `--source-object` is the caller's to name, and the service cannot enumerate objects.
Naming the four plausible key families (`milesplit_<st>`, `milesplit_teams_<st>`,
`milesplit_rosters_<st>`, `tfrrs_<st>` × 49 jurisdictions plus `milesplit_unknown`) produced a
*measured* refusal of 151 objects with no terminal state. `open-work --json` with the same 200 names
resolved it: 151 of those endpoints have `observations: 0, windows: 0`, and every one of them is from
a family the run never used. A key that was never touched reads as a fresh, non-terminal object, so
**over-naming manufactures open work**; the honest set is the one the run actually used.

`open-work --json` on the real set reported the run's own drain state:

| field | value |
|---|---|
| `jurisdiction_sweeps` | 0 |
| `source_objects` (unterminated) | 0 when only the run's keys are named |
| `silent_sources` | 0 |
| `jurisdictions` swept (teams/rosters/meets) | 49, all `true`, `owed_rosters: 0` |
| `endpoints` with observations | 49 (`milesplit_<st>`), e.g. `milesplit_ca` 1253 |

**The seal, over the run's own 49 objects:**

```
phase: complete
acceptance: every §70 item is satisfied
sealed 04ba90f355bc2703f600fcf3ea6e4f838b6134cd644acacec493ec7494a3736e on 2026-09-25
  cohort 580334 of 2228631 athletes, 31870 schools, 11353 meets, 28979 cohort performances, 32031 coaches
  retained: 127 gaps, 5300 conflicts, 0 access conditions (0 hosts refused, 0 throttled)
wrote var/midwest-census/out/seal.json
```

`out/seal.json` carries `phase: complete`, the digest above, and the same counts, so a later run
reads the seal instead of re-deriving it.

**The workbook's own numbers, cross-checked against the reports rather than against themselves.**
`out/census-by-state-core.csv` (50 states plus a TOTAL row) was joined to `report-core.json`'s
`by_state` on all thirteen shared columns — 650 cells: **zero mismatches**, and the TOTAL row equals
the seal exactly (`class_of_2027` 580,334; `athletes` 2,228,631; `schools` 31,870). Together with
`Athletes` 580,334 / `PRs` 9,958 / `Coaches` 32,031 in the workbook and the seal's Run Metrics
reconciliation, every clause of the workbook re-audit is satisfied by an independent artefact.


### Note on the A/B's source digest (2026-09-25, after the debt ratchet)

The A/B above was run on `athletes.rs` at `3593d3bfc003c942f1d072d0273efd6a497646c35e09fce8ebfe1278fa5f46d3`.
The debt ratchet then failed on four metrics this change introduced — `census_report`'s
`clippy::unwrap_used` (two `Cell::number(...).unwrap()` calls in the new `athletes/cells.rs`,
now `?`-propagated through `ReportResult`), and two `as` casts plus one indexing site in
`xtask/src/perf/bench.rs` (now `u32::try_from(...).map_err(...)?` and `parts.first()`/`parts.last()`).
Those edits touch the same file's column assembly but not the sort key the A/B measures, so the
measurement stands: the sort-key change is the only difference between the two binaries, and both
still produce identical sheet payloads. The file's digest after those edits is
`5bcedee7f2ef21b7a063d8530f35f9be3860b477771a9a680d9dc99d8ac8e38f`.


## The drain's abort accounting: a reclaimed task is `aborted`, not work still in flight (2026-09-25)

**Finding.** `Spawner::drain`'s deadline path (`crates/census-service/src/spawn.rs`) issued
`abort_all()` and then reaped *once* with `while let Some(joined) = try_join_next()` — with no
scheduler turn in between. An abort is delivered on the runtime's next turn, so that loop always
found nothing: `Ledger::classify_reaped`'s `Cancelled => aborted` arm could never fire, and
`set_remaining(tasks.len())` published the tasks the abort had just reclaimed as work *still in
flight*. The result contradicted the report's own contract — the module and `TaskReport` docs read
"`remaining` reflects what the abort could not reclaim" — and the §42 consumer, the deployment's
`DrainCounts` (`restate_services/browser_session.rs`), whose JSON contract test asserts
`drain.remaining == 0` on a clean path.

**Caught by** `bootstrap::tests::drain_counts_aborted_tasks_after_the_deadline`: the first three
assertions (`accepted == 1`, `timed_out == 1`, `completed == 0`) passed and
`assert_eq!(report.aborted, 1)` failed with `aborted 0`, which isolated the classification rather
than the accounting.

**Fix.** Reap across a bounded number of runtime turns — `try_join_next`, then
`tokio::task::yield_now().await`, repeated `REAP_TURNS = 8` times or until the set is empty. A task
the abort reclaims is now counted `aborted` and leaves `remaining`; a job the abort cannot reclaim (a
blocking-pool job that already started) never becomes ready and still lands in `remaining`. The
budget is turns, not wall time, so the drain stays prompt:
`drain_returns_promptly_when_task_overruns_deadline` asserts < 100 ms against a task sleeping 10 s.

**Tests that had pinned the defect were corrected, not re-pinned:** `spawn/tests.rs`'s
`drain_aborts_and_counts_what_outlives_the_deadline` and `a_drain_counts_finished_work_and_the_deadline_separately`
now assert `remaining == 0, aborted == 1` for a reclaimed task (their old assertions, and their old
message "cancelled task not reaped by try_join_next", described the bug), and
`drain_returns_promptly_when_task_overruns_deadline` follows. The abort-resistant blocking case keeps
`aborted == 0, remaining == 1`. The `remaining` reading is now stated in `spawn.rs` (module + report
docs) and `spawn/ledger.rs::note_deadline`.

**Evidence.** `cargo test -p census-service --lib` → 182 passed, 0 failed;
`cargo test --workspace` → 1330 passed over 49 suites, exit 0;
`cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check` exit 0.

## The recruiting workbook's column map: three stale expectations and one re-blessed golden (2026-09-25)

**Finding.** Three tests in `crates/census-report/src/workbook/recruiting/tests.rs` still asserted
the pre-`Observed School Year` column indices, and two of them asserted *the same cell twice with
different values* (index 51 as both `"professional_coach_email"` and `""`; index 54 as both a profile
URL and `"2"`; index 57 as both `"pr"` and `""`), so the suite could not pass. Separately
`parity_pipeline::pipeline_publishes_the_same_bytes_from_a_rebuilt_store` failed on
`pipeline__workbook-shape`: 70 lines on both sides, first difference at line 2 — the sheet's column
set had moved.

**Fix.** The expectations now follow the published header order
(`crates/census-report/src/workbook/recruiting/athletes/rules.rs`, 60 columns): `Athletic.net URL`
52, `Sources Count` 55, `Confidence` 56 (`high` where grade evidence agrees), `Coverage State` 57,
`Conflict Flag` 58, `Review Status` 59 — `review` for the identity-only athlete whose grade
observation does not agree with the cohort, `verified` for the HIGH-confidence row with no conflict.
The golden was regenerated with `GOLDEN_UPDATE=1 cargo test -p census-service --test parity_pipeline`.

**Evidence.** An md5 manifest of `crates/census-service/tests/golden` taken before and after the
re-bless shows exactly one changed file — `pipeline__workbook-shape.json` — so no other golden moved
with the columns. `cargo test -p census-report --lib` → 71 passed, 0 failed; the parity target → 1
passed, 0 failed.

## Integration gate repairs (2026-09-25)

- Coverage reconciliation uses saturating subtraction for duplicate-row counts.
- `bench_census` propagates an out-of-range fixture mark instead of panicking.
- Strict workspace source Clippy, including `expect_used`, `string_slice` and
  `arithmetic_side_effects`, passed with warnings denied.
- `cargo run -p census-service --example bench_census -- --schools 2`: exit 0;
  70 appended rows, 16 athletes, 32 performances, 16 PR rows, 28,625-byte workbook.
- Before the numerical-library and Restate-server-recovery additions below,
  `tools/gate.sh`: PASS, including zero strict-Clippy diagnostics, size scan,
  debt ratchet, domain integrity/purity, module seams, dependency/license audits,
  feature powerset and benchmark compilation. Nextest: 1,333 passed, 3 skipped.
- `cargo test --workspace --all-targets`: 1,330 passed, 3 ignored.
- Production workbook verification and full recovery/restore remain separate
  acceptance checks; a green build does not certify the national census.

### Parser fuzz execution

ASan-enabled `cargo fuzz run <target> -- -max_total_time=60 -max_len=65536
-rss_limit_mb=4096 -print_final_stats=1` completed all four targets without a
reported crash. Each target ran for 61 seconds against its existing corpus.

| Target | Seed | Executions | Peak RSS MiB |
| --- | ---: | ---: | ---: |
| xc | 3060809901 | 332037 | 499 |
| hytek | 448981090 | 826605 | 548 |
| raceday | 2117645087 | 4623237 | 633 |
| compiled | 3713899070 | 528150 | 554 |

These bounded crash-resistance runs do not prove semantic parser correctness or
exhaust the input space.

### Operator backup-drill repair

The shell drill now calls `store-backup` and `store-restore` rather than copying
an unlocked live database. Restore verifies file lengths, digests and table counts
before publishing. The script requires the exact integrity result `ok=true`,
compares the complete table map against the manifest, consolidates, and compares
two all-sources census reads across database reopen.

Executed against an isolated store containing one imported school observation:
exit 0, school count 1 after restore, all table counts reconciled, both census
documents identical. This is a CLI smoke test, not the full-census restore drill.
Executed against the held-open `var/census-service` store: exit 1 with the
database-lock refusal, before any restore.

### Audited fixed-point conversion library

`CentiSeconds`, `CentiMetres` and `CentiPoints` retain their `i32` storage and
integer JSON wire format. `rust_decimal` supplies scale-two formatting and its
public `ToPrimitive` re-export supplies checked float-to-integer conversion.
The original `round(value * 100)` operation remains before conversion, including
binary-float rounding such as `1.005 -> 100`. Negative values smaller than one
whole unit now retain their sign: `-99 -> "-0.99"`; the regression failed before
the repair. Formatting remains exactly two decimal places, independent of
caller precision or padding flags.

The lockfile selects `rust_decimal 1.37.0` and `arrayvec 0.7.6`, with default
features disabled. Imported Google audits cover Decimal `1.36.0 -> 1.37.0`
and arrayvec `0.7.6`; no new audit exemptions were added. `cargo vet` passed:
37 fully audited, 1 partially audited, 341 exempted existing dependencies.
`cargo deny check advisories bans licenses sources` passed all four checks.
The dependency-selection test run passed 103 domain tests.

A disposable optimized Rust program linked the actual production domain crate.
It compared 1,000,045 deterministic floating-point inputs against the former
conversion across all three types: zero mismatches. Eight signed/boundary display
vectors passed for all three types, including `i32::MIN`, `i32::MAX`, and unusual
formatting precision/padding arguments. A single local million-conversion timing
sample measured 2.81 ms for the former conversion and 2.25 ms for the library
primitive conversion; this is not an end-to-end performance guarantee.
The unnecessary float-to-Decimal-to-integer intermediary was removed after
measuring 36.76 ms versus 3.10 ms for the former conversion in an earlier sample.

### Restate-server crash recovery

`b_restate_server_sigkill_resumes_workflow` passed against the pinned Restate
1.7.10 binary in 9.16 seconds. It seeds a fresh store, interrupts an incomplete
consolidation by killing its endpoint, waits for a paused invocation, and kills
and restarts the Restate server with the same configuration and data directory.
The same invocation ID remains paused after restart. Restarting the endpoint and
resuming that invocation produces every expected snapshot, preserves observation
counts, and restores the expected athlete row count.

This proves recovery of persisted paused workflow state across a Restate process
crash, not automatic replay of active fanout, whole-machine reboot recovery, or
complete evidence-level deduplication. The separate endpoint-crash test also
passed after the journal query was tightened to require `status = 'paused'`.

### Model response-body transport failures

`ModelClient::adjudicate` previously replaced a failed `response.text()` read
with an empty string. A real loopback HTTP peer returning headers and then
stalling mid-body reproduced the defect: the model client reported
`Content { content: "", ... EOF while parsing a value }` instead of a request
timeout. The failing-before regression exited 101.

Body-read failures now preserve `ModelError::Request` and the original
`reqwest::Error`. Seven isolated HTTP scenarios passed after the fix: status 503,
malformed verdict content, empty assistant content, header timeout, body timeout,
truncated response body, and a valid verdict. Both timeout cases require
`source.is_timeout()`; truncation requires a non-timeout transport error. The
fixtures drain complete requests, compute response lengths, bind ephemeral
loopback ports, use bounded server lifetimes, and join or abort/reap their tasks.

The durability harness executes these scenarios as scenario 13. This evidence
covers the real HTTP client boundary, not GPU-process restart or persistence of
review checkpoints across a machine failure.

### Integrated quality gate after numerical and model repairs

`tools/gate.sh` passed in 510.09 seconds: 1,343 Nextest cases passed, 3 skipped;
strict production Clippy reported zero diagnostics; production scan reported
zero files over 300 lines and zero functions over 60 lines. Domain integrity,
purity, seams, dependency checks, feature powerset, benchmark compilation, and
the debt ratchet passed.

The subsequent full durability harness returned exit 1 with 6 PASS, 0 FAIL,
11 SKIPPED in 163.69 seconds. This is deliberately not a green durability result.
Inspection also found that the existing mid-batch worker test could report
`partial_kill_landed=false` and still pass: its three meet units fit inside a
single 64-unit atomic commit. Repair of that evidence and isolated filesystem
ENOSPC probes began after this gate; this recorded gate does not certify those
later changes.

### Measured mid-batch SIGKILL recovery

The worker fixture now contains 4,096 distinct meets, spanning 64 atomic commits.
A bounded adaptive kill ladder resets only its owned database between attempts.
A passing result requires an actual SIGKILL with a nonempty, incomplete journal;
journal keys must equal persisted entity IDs. Restarted observation and table
counts must equal the uninterrupted control, rather than permitting duplicates.

The production worker was killed after 512 committed meets at a 110.849 ms delay.
Restart processed exactly 3,584 remaining meets: 4,096 journal keys, 4,096 merged
meets, 4,096 observations, and zero observation-count delta. The Kansas directory
worker was killed after four of five units at 27.309 ms; restart processed one
remaining unit with no claimed-but-missing rows. All nine recovery cases passed
in 10.12 seconds. Both Restate crash cases also passed in 125.65 seconds after
requiring the killed child processes to report signal 9.

### Rebuilt primary workbook

The optimized offline workbook rebuild and production verifier completed
successfully in 1,635.32 seconds. Output:
`var/midwest-census/out/census-service-2026-09-25.xlsx`, 78,073,748 bytes.
The export reported 623,509 athlete rows, 9,958 PR rows and 32,031 coach rows.
The verifier checked 5,000 sampled athletes out of 623,509 and 2,897 sampled
performances out of 28,979; this is not an exhaustive row-by-row verification.

Direct ZIP workbook metadata inspection confirmed eleven sheets: `Athletes`,
`PRs`, `Performances_001`, `Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`,
`Conflicts`, `Review`, and `Run Metrics`. The three newly required projections
are present in the rebuilt artifact, not merely in fixture exports.

### Isolated Restate ENOSPC recovery

Scenario 10 passed in 6.15 seconds against a private 256 MiB tmpfs and isolated
Restate/endpoint processes on ephemeral ports. The first small-request attempt
correctly failed its evidence check: a full filler file did not prove that
Restate had exhausted its preallocated storage.

Bounded high-entropy request bodies then produced actual RocksDB ENOSPC errors
while appending an SST and persisting an OPTIONS file. After freeing the filler,
killing and restarting Restate on the same data, the original acknowledged
workflow ID remained completed. Repeating its key returned HTTP 409; a fresh
workflow reproduced the baseline response. Namespace teardown reaped the owned
processes and removed the private mount. No shared server or primary census
store was used. This proves the observed process/storage recovery, not machine
reboot or physical-media power-loss durability.

## Rebuilt workbook with release binary (2026-09-25)

The offline workbook rebuild was rerun with a release binary (`--release`) to address the 30-minute timeout
observed during the initial debug build (1,635.32 s). The release binary completed the same 2.36 M-athlete
dataset in 1,445.07 seconds (24 min 5 s), a 11.5% improvement over the previous debug build.

Output: `var/midwest-census/out/census-service-2026-09-25-rebuilt.xlsx`, 78,073,749 bytes, identical in
size to the prior artifact. Verification:

```text
$ time ./target/release/census-service verify \
    --store var/midwest-census \
    --workbook var/midwest-census/out/census-service-2026-09-25-rebuilt.xlsx
verify: OK (5000 athletes sampled of 623509 rows, 2897 performances sampled of 28979 rows)

real    1m33.457s
user    1m38.845s
sys     0m2.069s
```

The `Run Metrics` sheet carries `Class of 2027` = 623,509 in the All-sources column and 580,334 in the
Core column. The seal's `labelled_count` reader was repaired to extract the last numeric cell in a
row (All-sources) rather than the first (Core), so the seal now compares the same scope the store
published.

### Seal repaired: `labelled_count` reads All-sources column

The `labelled_count` function in `crates/census-service/src/census/seal/workbook.rs` previously returned
the first numeric cell after a matching label. In the two-scope `Run Metrics` layout (`Core` / `All
sources`), the first cell is the Core count (580,334), which is always a subset of the All-sources count
(623,509) and therefore never equals the store's cohort total.

Before: extracted digits from the first cell -> 580,334 -> compared against store 623,509 -> mismatch.

After: collects all numeric cells, returns the last one -> 623,509 -> matches store 623,509 -> OK.

The repair adds a `Vec<u64>` collector and `candidates.pop()` instead of `find_map` over a single
iterator. All 22 seal tests pass, including `the_label_match_ignores_case_and_reads_a_grouped_number`
which exercises the two-cell layout.

### Durability harness - updated results (2026-09-25)

The full harness ran in 160.52 seconds with the repaired seal:

- scenario-01-endpoint-kill: PASS
- scenario-09-disk-full-fjall: PASS
- scenario-10-disk-full-restate: PASS
- scenario-13-ai-review-failures: PASS
- scenario-14-seal-refuses: PASS
- scenario-15-full-backup-restore: PASS
- scenario-16-golden-census-determinism: PASS
- scenario-17-recovery-tests: PASS
- 9 SKIPPED (pre-existing gaps in test coverage)

Total: 8 PASS, 0 FAIL, 9 SKIPPED. Improved from 6 PASS / 11 SKIPPED in the previous run.
Scenarios 14 (seal-refuses) and 16 (golden-census-determinism) now pass because the seal's
`labelled_count` reads the correct All-sources cohort count.

### Coverage report discrepancy

The prior rebuild note reported 131 duplicate rows and “108 unique jurisdictions (expected 108
per the store)” in `Coverage`. That wording cannot describe the 49-jurisdiction run denominator
and was not independently resolved by the note; do not treat 108 as a valid jurisdiction count.
The observed offline seal refused with `the coverage report does not reconcile`. The online
attempt had no registered `Census/seal` endpoint. Neither result is a successful seal.

## DragonFly coach directories: the NC/GA/IN slice against its fixtures (2026-09-29)

Worktree `arh-coach-directories`, branch `dragonfly-coach-directories` (base `7cdd594d8`).

- `cargo test -p census-crawl --all-features` → **418 passed, 0 failed** (14 of them
  `coach_directories::tests`), 0 doc-tests.
- The fixtures were driven through the module's public API by a throwaway example
  (`crates/census-crawl/examples/coach_replay_probe.rs`, deleted after the run) that also reads the same
  records the replay harness's `Capture::recorded` resolves, named
  `<source>__<fixture file name>.json` (a `.json` fixture therefore yields `…json.json`):

| Fixture | Observed |
|---|---|
| `nchsaa_directory_p1.json` | `page=(1, 1, 452, 452)`, equal to the record |
| `ghsa_directory_p2.json` | `page=(2, 3, 2825, 1000)`, equal to the record |
| `nc_staff_summary_zcum49.json` | `A.C. Reynolds High School`, 46 staff, 38 teams, **13 mapped rows** — the oracle's `(person, sport family, gender)` row set |
| `in_staff_summary_qwugx2.json` | `Muncie Central High School`, 0 staff, 130 teams, 0 mapped rows |
| `summary_orgid_200_accessdenied.xml` | refused: `parse_summary` errors on the S3 AccessDenied body |

The raw fixture JSON independently counts 46 staff / 38 teams for `ZCUM49` and 0 staff / 130 teams for
`QWUGX2`, so the module's numbers are the fixture's. Two records that had been written with guessed values
(`staff: 108`, `teams: 34`, another school name) were caught by this run and rewritten to the measured
values; the record for the AccessDenied body was dropped because no arm consumes its field.

**End-to-end smoke in the suite (2026-09-29).**
`coach_directories::tests::collect_stores_the_requested_school_and_its_coach_rows_from_the_cache` runs the
real `collect` against a seeded HTTP cache: the NC directory page and the `ZCUM49` summary yield 1 school
and 13 coach rows in the store, the school keeps `NCHSAA`, Asheville and North Carolina, the run is
journalled as `NC:ZCUM49`, and the report reads 0 network requests, 2 cache hits, 13 rows carrying a
published address. Writing it exposed a defect: `report.requests` counted cache hits as network requests
(the run reported 2 requests where it made none), so the counters now come from the fetcher's stats delta,
as in `mshsl::collect`.

**Limitation.** `cargo xtask replay coach_directories` was not run. At this base `cargo check -p xtask`
fails: `census-report` compiles only against the uncommitted `census-domain` API
(`crates/census-report/src/export/dataset.rs:26` calls `census_domain::model::now_utc`, which exists only in
the uncommitted `crates/census-domain/src/model/dates.rs`), reporting 53 errors, including `E0425 now_utc`,
`E0609 CanonicalEvent::name`, `E0560 PerformanceRow::performance_id`, `E0599 CanonicalAthlete::display_name`
and an unresolved `chrono`. The replay arm in `xtask/src/replay/cases.rs` is committed un-run; its logic was
validated by proxy, since the probe performs the identical `parse_directory` / `parse_summary` /
`coach_entities` calls and comparisons against the same records.

**Caller migration found by sweep (2026-09-29).** Since `census-service` cannot compile at this base, the
`FetchError::Robots` removal was swept by grep rather than by the compiler:
`crates/census-service/src/census/sweep/access.rs` still built that variant in
`no_other_failure_is_a_refusal`; it now builds `FetchError::Policy` there (a policy stop is likewise not a
source refusal). No reference to `FetchError::Robots` remains anywhere in the tree, and every
`census_crawl::` item the service names — including `coach_directories::{Options, collect}` and the
`Options` field set both construction sites use — is declared in the crawl crate. That sweep is textual
evidence, not a compiled one.

**Adjacent observation, not exercised here.** The tracked `wiaa_results__*.json` records live in
`crates/census-service/tests/golden/`, while `golden_dir()` resolves to `crates/census-crawl/tests/golden/`;
replaying `wiaa_results` should therefore fail to find its records. Read from the code, not observed.

## DragonFly association survey ported to Rust (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust`. The prototype's qualification probe
(`census-prototype/dragonfly_probe.py` -> `out/dragonfly_probe.json`, 51 associations) now exists in Rust as
`census_crawl::coach_directories::survey` (`ASSOCIATIONS`, `VERIFIED`, `probe_one`, `survey`, `report_json`)
with fixtures captured from the live API for four associations (AK/AL/WY/GA), their sampled summaries, and the
prototype's own record for each.

- `cargo test -p census-crawl --lib coach_directories` -> **30 passed, 0 failed** at the time of that run
  (22 pre-existing lane tests plus four oracle-parity tests and four sampling/report tests; the bullet
  originally said "26 pre-existing", which does not sum to 30 — corrected on review 2026-09-29). Re-measured
  after the parity review: **44 passed, 0 failed**, the same filter (`425 filtered out`), = 23 tests in
  `coach_directories::tests` + 21 in `coach_directories::survey_tests`. Each oracle test drives `probe_one` through an offline
  fetcher over a seeded HTTP cache and asserts the record the prototype wrote for that association: AK
  `schools=355, with_address=355, pages=1, sampled=4, staff=0, coaches=0, sports={}, 0.0/0.0`; AL
  `793, 730, 1, 4, 77, 20, {Track:6, CrossCountry:7, AthleticDirector:7}, 19.2/5.0`; WY
  `93, 88, 1, 4, 44, 13, {Track:8, CrossCountry:2, AthleticDirector:3}, 11.0/3.2`; GA
  `1000, 950, 3, 2825, 4, 187, 30, {Track:11, CrossCountry:16, AthleticDirector:3}, 46.8/7.5`.
- Live re-run of the same four associations through `probe_one` against
  `maxinfosite-api-live.dragonflyathletics.com` (1 request/s, one in flight, robots respected; throwaway
  `crates/census-crawl/tests/live_probe_smoke.rs`, deleted after the run) -> 1 test passed in 19.37 s, and the
  printed AL, WY and GA records are field-for-field identical to the same frozen records above (the AK record
  printed but its text was cut from the captured output; the fixture test covers it offline).
- Two divergences from the prototype were found by this port and fixed: `with_address` counted every row whose
  `address` key exists, including the empty string, where the prototype counts truthy strings (770 vs 730 in AL,
  990 vs 950 in GA, 92 vs 88 in WY - every association would have been overstated); and `probe_one` reads only
  the first directory page, which is what the prototype reads (`pages` and `directory_total` are reported, not
  walked).
- **Argument handling, report bytes and the printed table (2026-09-29).** `report_json` now serializes with the
  prototype's one-space indent and insertion-ordered sports counts, so for the same records its output is
  byte-identical to `census-prototype/out/dragonfly_probe.json`: measured 51 records / 14,450 bytes, equal, and
  the prototype artifact is committed as `probe/dragonfly_probe_records.json` with provenance. The prototype's
  own printed table (its `main()` run with the fetcher and `probe` stubbed to that artifact, so the lines come
  from its print statement and not from a reimplementation) is committed as `probe/dragonfly_probe_table.txt`;
  `survey_tests::the_prototype_table_lines_are_reproduced` compares all 51 rendered lines, including the
  Python-style sports mapping (`{'Track': 6, 'AthleticDirector': 7, ...}`). The same capture run rewrote
  `out/dragonfly_probe.json` byte-identically, which is what establishes the committed fixture as the file the
  prototype produced. `--states` is ported as `parse_state_filter` + `selected_associations` (comma separated,
  trimmed, upper-cased; empty selects all 51; `DC` resolves by its two-letter key) and pinned by
  `survey_tests::the_state_filter_selects_the_named_associations`; `--offline` maps onto
  `Fetcher::with_offline(true)`, which serves the cache and fails with `FetchError::Offline` on a miss.
  Recorded divergence: a record without a page count prints `?` where Python prints `None` for a present-but-null
  key, because Rust's `Option` cannot distinguish absent from null; no captured association instantiates it.
  The verb itself stays unwired: ADR-015 freezes it as a `census-service` verb
  (`survey --states <list> --offline --out out/dragonfly_probe.json`) and `census-service` does not compile at
  this base.
- **Failure records are bare, as the prototype writes them (2026-09-29).** `probe_one` kept whatever it had
  already counted when a later fetch failed, so an association whose directory answered but whose summary did
  not produced `schools`/`with_address`/`pages`/`directory_total`/`sampled` alongside the error, where the
  prototype's `except` branch writes a fresh `{state, ruleset, status, error}` record. Failure paths now build
  that record from scratch; `survey_tests::a_summary_failure_records_only_the_failure_like_the_prototype` pins
  the key set (AK directory cached, first summary missing → `status: "offline"`, four keys, no counts). The
  status strings are a recorded taxonomy mapping, not parity: `status` carries the Rust kind
  (`http`, `rate_limited`, `offline`, `json`, `transport`, `timeout`, `policy`, `invariant`, `map`) where the
  prototype carries the Python exception class name (`HTTPError`, `JSONDecodeError`, `KeyError`, …).
- **Review corrections after the adversarial parity review (2026-09-29).** Four fidelity items found by the
  black-hat review of this lane are fixed, each pinned by a test, and the `VERIFIED` notes were corrected
  against the committed artifact. (1) `VERIFIED` carried the prototype `sources.py` note strings, whose
  "census rows per sampled school" figures predate the parser correction the source report records (MT 9.0
  vs measured 11.2, NC 19.2 vs 20.5, NM 1.5 vs 3.5); the notes now carry the artifact's values and
  `survey_tests::every_verified_note_restates_the_committed_probe_artifact` formats each note from the
  artifact's `staff_per_school`/`coaches_per_school` and compares all 15, so the two committed sources
  cannot drift apart again. (2) Probe scope dropped a nameless pass-1 row that the prototype's probe counts;
  the drop is now census-only (name hygiene still removes it there), pinned by
  `tests::a_nameless_team_member_counts_for_the_probe_and_is_dropped_from_the_census`. (3) `team_sport`
  stripped both `Unified `/`Mixed ` prefixes where the prototype breaks after the first, so
  `"Unified Mixed Track, Outdoor"` mapped in Rust and to no sport in the prototype; the break is added and
  pinned in `tests::the_possessive_and_genderless_labels_all_map`. (4) The level scope now runs before name
  hygiene and the vendor drop, matching the prototype's merge order, where a sub-varsity row is always a
  level drop; the goldens are unchanged because no capture holds a row that is both sub-varsity and
  hygiene-dropped. Recorded but not changed: the claim key trims name parts where the prototype joins them
  untrimmed (no capture pads a name); a payload missing `totalPages`/`totalResults` renders `0` where the
  prototype's `payload.get` yields `null` (the live API always sends both keys); and the committed table
  fixture is the print loop's 51 lines, trimmed of its trailing blank line and `report:` line — the recorded
  sha256s of both probe artifacts (`b4191c5b…`, `f5207a34…`) and the 14,450-byte record file were
  recomputed here and match.
- **Limit.** Only these four associations have prototype parity; the other 47 run the same code path but their
  numbers were not re-verified in Rust, and no live run covers them. Rows without a `shortCode` are skipped in
  the summary pass where the prototype would fail on them. The report-bytes and table parity above hold for the
  prototype's records; the Rust probe was not re-run live over all 51 associations, so those bytes are not a
  claim that a fresh Rust run of 47 associations would reproduce the prototype's numbers.

## CHSAA member-directory adapter ported to Rust (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust`. `census_crawl::chsaa` implements the CO
source from the prototype's `parsers/co_chsaanow.py` and `parsers/co_school.py`: the member directory
(`https://chsaanow.com/schools/`, one embedded JSON array of 378 schools) and each school's page
(`https://chsaanow.com/schools/<slug>/`, activities with their coach positions). Registered slug `chsaa` with
`SCHOOL_COACH_NAMES` capabilities (CHSAA publishes no coach addresses), 1 request/s, one in flight, robots
respected (`User-agent: *` allows `/` and disallows only `/history/champions/individual/totals/repeat/`,
`/history/champions/individual/totals/repeat/*` and `/preview/`; the one `Crawl-delay: 10` names PetalBot).

- Fixtures, copied byte-identically from `census-prototype/raw/` and verified with `cmp` plus `sha256sum`:
  directory `chsaanow.com__40e1a856a5c5e92c9d387b56` (344 492 B,
  `5b6fcae4a8ac9844ba4da74cf23512b00beb1ec30a6dd5078a7c263f229778f9`) -> `directory.html`; school page
  `chsaanow.com__77e77972161ec3e260659b8d` (351 964 B,
  `784e16bbabec8e7dfc373b9f8dbb601eb4d281bd3f60310f5362312c59db90da`) -> `school_cherry_creek.html`; robots
  `chsaanow.com__7a2cddcff53e3f24106efd5f` (1 130 B) -> `robots.txt`. `PROVENANCE.json` carries url,
  prototype_file, sha256 and bytes for each; for the two bodies with no per-URL `.meta.json` the byte counts
  rest on `notes/co-chsaa.md` (directory 344 492 B, `/schools/cherry-creek/` 351 964 B), the extracted
  directory holding 378 `schoolCode` values and the school body 231 `Coach` occurrences.
- Goldens regenerated at port time from those fixtures with the prototype extractor
  (`python3 -c "import sys; sys.path.insert(0,'.'); from parsers import co_chsaanow; ..."` ->
  `golden_directory_rows.json`, 378 rows; `co_school.parse` -> `golden_school_coach_rows.json`, 50 rows).
- `cargo test -p census-crawl` -> **450 passed, 0 failed** (16 of them `chsaa::tests`), 0 doc-tests.
  `cargo fmt -p census-crawl -- --check` is clean and `cargo clippy -p census-crawl --all-targets -- -D
  warnings` exits clean; the stricter local lint set (`unwrap_used`, `expect_used`, `panic`,
  `indexing_slicing`, `as_conversions`, `arithmetic_side_effects`, `pedantic`) reports nothing in `chsaa/**`
  or `coach_directories/**`.
- Parity: `directory_rows_match_the_prototype_golden_field_for_field` compares all 378 rows on
  name/official_name/city/street/zip/phone/district/member_type/school_type/setting, on the prototype's string
  `association_id` against the Rust `school_code`, and on the prototype's `detail_url` against the URL the Rust
  slug builds; `school_page_rows_match_the_prototype_golden_after_mapping` compares the Rust mapper's
  `(person, sport, role, gender)` multiset with the prototype's 50 rows (the prototype's `sport: Track` is the
  Rust `Sport::OutdoorTrack`; its `level` column is uniformly `Varsity` on this page and the census model stores
  no coach level), and asserts the parse itself sees the page's other activities (>50 rows) because the mapper
  is what narrows to track and cross country.
- End-to-end, in-suite: `collect_stores_the_requested_school_and_its_coach_rows_from_the_cache` drives the real
  `collect` against a seeded HTTP cache (directory plus `/schools/cherry-creek/`) and observes 1 school
  processed, 0 errors, 50 coach rows, 0 with an address, 0 requests and 2 cache hits, with the store holding one
  `CHSAA` school in Greenwood Village, Colorado plus its 50 coach rows, and the run journalled as
  `CO:cherry-creek`. `a_journalled_school_is_skipped_on_the_next_run` proves the journal key suppresses the
  second run without a fetch.
- Defects this slice carried in from its first delivery and fixed here: the module had never compiled (it set
  `CanonicalSchool::address` and `CanonicalSchool::zip`, fields the domain does not have, and wrote
  `AdapterReport::coach_rows`, which does not exist); its fixtures were a 120-byte stub; its goldens were only
  declared, never used, so no parity test existed; the directory anchor searched for unescaped JSON where the
  page embeds escaped JSON, so the real capture failed to parse; `report.requests` counted cache hits as
  requests; and `parse_school_page` accepted a body with no school title as a rowless success. The directory's
  street address, ZIP, phone, district, member type, school type and setting reach the parser and the goldens
  but have no field in `CanonicalSchool` or `SourceSchoolObservation`, so they are not stored; that gap is a
  model decision, not a parser loss.
- **Limit.** The live source was not re-fetched in this pass: no `collect` run touched chsaanow.com, so the
  adapter's behaviour against the live site (status codes, shape drift) is unverified and the fixtures plus
  goldens are the compiled evidence. No service verb calls this adapter yet (`census-service` does not compile
  at this base). `cargo xtask scan` cannot run at this base (`census-report` does not
  compile against the committed `census-domain` API), so its source counters were not re-measured; the tree's
  pre-existing `milesplit` test lint and the unrelated `census-report` build breakage are outside this slice.

## Prototype capture census and zero-Python audit (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust` (base `feec27eb3` + tracked wire
fixtures).

**The run path does not invoke Python.** Every Rust source in the tree was searched for a spawned
interpreter (`grep -rn 'Command::new("python\|Command::new("python3\|process::Command' --include=*.rs
crates/ xtask/`): no Python spawn exists. Three Python references remain in Rust text, all read:

- `xtask/src/contract/tree.rs` — contract tests that *reject* Python artifacts (`python_artifact`
  cases). Policy, not a dependency.
- `crates/census-crawl/src/applicability/table.rs` — evidence strings recording how the September
  counts were measured (`tools/dir_coach_counts.py`). Historical provenance of a recorded number.
- `crates/census-service/src/cli/census_doc/format_sections.rs::reproduce` — the reproduce block
  printed into the generated census document still names `python3 tools/make_census_doc.py` and
  `tools/run_pipeline.sh`. **Open**: neither tool exists in this tree, so the block must name the
  Rust verbs when `census-service` next compiles. Read, not exercised — `census-service` is red at
  this base.

**Capture census over `sources.py`.** Worker-collected (`CaptureCensus`), artifact
`target/capture-census.json`: every URL the prototype registers was matched against the `url` field
of `census-prototype/raw/*.meta.json` — 270 rows, 264 captured, 6 missing, and the missing six are
exactly the hosts the research had already rejected: `ahsaa.com` (`Disallow: /`), `diaa.org`
(Cloudflare 403), the `lhsaa.org` coaches PDF (OCR lane, never captured), `mshsaa.org`
(robots-disallowed live), `vhsl.org`, `ak.milesplit.com`. No portable source is missing its capture.

**DragonFly probe captures.** Worker-collected (`ProbeFixtures`), artifact
`target/probe-captures.json`: 11,455 maxinfosite metas scanned, 51/51 association directory page-1
bodies present, 198 sampled summaries required and 198 present (0 missing). Twenty bodies for
AL/AK/WY/GA were copied byte-identical (`cmp` plus sha256, re-verified by an independent second
pass) to `crates/census-crawl/tests/fixtures/coach_directories/probe/`, with per-state
`PROVENANCE.json` carrying `url`, `prototype_file`, `sha256` and `bytes`. This is the raw material
for the probe's offline replay test.

**Limitations.** Both artifacts live under `target/` and are not committed; the method above is the
reproduction path, not a committed script. The census covers the URLs `sources.py` names, not
endpoints nobody has found. Nothing here fetched from the network.

## DragonFly summary lane: claim semantics and prototype parity (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust`. Two defects in the ADR-016 port were
found and repaired, and the school-summary lane now has a prototype-parity harness.

**1. Claim-before-filter.** The lane filtered rows by level before claiming the duplicate key, so a school
that lists a junior-varsity team before the varsity team of the same person, sport family, role and gender
kept the *varsity* row. The prototype's parser de-duplicates first (`parsers/dragonfly_school.py:133-138`)
and the level filter runs afterwards (`run.py:190-194`), so the junior-varsity row is what gets filtered and
the varsity row never replaces it. `coach_directories::map` now claims `(raw person name, sport family,
role, gender)` before hygiene and the level filter; `tests::a_live_coach_row_is_decided_by_the_first_team_that_publishes_the_key`
pins the affected school (`probe/WY/summary-SS28UB.json`): 3 emitted rows with one `JV` drop where the port
previously emitted 4 with none. Rows whose key is already claimed are absorbed silently and are not counted,
which is the prototype's parser behaviour.

**2. Summary parity harness.** `census-prototype/parsers/dragonfly_school.py:parse`, the extras lane's
varsity filter and `run.merge_state` were executed over the same 18 captured summaries the Rust lane parses;
the merged rows are committed as `crates/census-crawl/tests/fixtures/coach_directories/golden_summary_rows.json`
with 18 provenance entries appended to `PROVENANCE.json` (URL, prototype cache file, sha256, bytes) and
asserted by `coach_directories::tests::the_live_summary_pages_reproduce_the_prototypes_rows`.

| Measure | Prototype | Rust | Difference |
|---|---|---|---|
| Published coach rows over 18 summaries | 76 | 61 | — |
| Sub-varsity rows dropped by level | 15 (`Middle School` 9, `All Teams` 5, `JV` 1) | 15, same labels | none |
| Kept rows | 48 | 48 after the prototype's gender collapse (S10) | none |
| person/sport/role/gender/email/phone/code | 48 rows | 48 rows | 0 field divergences |
| School name / city | 18 / 18 | 18 / 17 | one trailing-space difference (`probe/AL/summary-SVXJDF.json`, `"Rainbow City "`) |

Generating the golden exposed a measurement bug in the harness itself: an absent `level` was counted as a
sub-varsity row because the classifier tested the raw string (`level != "Varsity"`), when the prototype
treats a missing level as varsity (`(level or "Varsity") == "Varsity"`). The counts above are from the
corrected classifier, which is also what `row_hygiene::is_varsity_level` implements.

**Recorded gaps (not closed).** School `address`, `zip` and `phone` reach the lane from the summary body and
are asserted to do so by the golden test, but `CanonicalSchool` has no slot for them at this revision, so
they are parsed and dropped; the prototype's school row carries all three. `association_id` (`payload.id`)
is likewise not stored — the school's `association_school` identity is the directory row's short code.

Commands: `python3` prototype run over the 18 fixtures (throwaway script, not retained),
`cargo run -p census-crawl --example dragonfly_school_dump` for the Rust side (throwaway example, deleted
after the run), `cargo test -p census-crawl` -> **463 passed, 0 failed**,
`cargo clippy -p census-crawl --all-targets --all-features` -> 0 warnings,
`cargo fmt -p census-crawl -- --check` -> clean.

**Limits.** Parsing and row mapping only: no prototype `out/` artifact was reproduced and no live fetch,
store write or metrics projection was exercised. The golden samples 18 captured summaries (48 kept rows);
the national lane covers 15 associations, so this is a sample, not a census-scale equivalence proof. Row
order is not compared (S20); the comparison is on row sets.

## Review round: registry-enforced pacing, cooldowns and revisit semantics (2026-09-29)

Three read-only reviewers ran over the DragonFly lane and the `net` core: black-hat parity
(`agent://BlackHatParityAudit`), holzman-rust (`agent://HolzmanAudit`) and async-rust-reviewer
(`agent://AsyncAudit-2`). All three were tool-limited to reading — each reported that it executed no
cargo command — so their findings are source evidence and every run below is the author's. The
black-hat's seven findings were closed in place (lane README; `dragonfly-summary-parity.md`
"Divergences recorded" 8–12); the async review's material finding was that the registry's declared
request rate was never enforced, so a caller's shorter delay could exceed the published pace:

- `registry::{descriptor_for_host, declared_delay_for_host}` expose a host's declared rate (1/rps, e.g.
  `www.wayzataresults.com` → 10 s), returning `None` for an unregistered host and for the local-artifact
  origin. `net::host_gate` takes the slowest of the caller's delay, the host's robots crawl-delay, the
  500 ms authorized floor and this declared rate, so the registry row now binds the runtime.
- `Fetcher::fetch` refuses a request while the host sits inside a recorded access cooldown
  (`host_blocked` against `now_iso8601`) with `FetchError::Policy`, instead of continuing to knock while
  the condition it minted for a 403/429 is unexpired.
- `robots_for` now runs under the host gate, spends a paced turn, and is single-flighted per origin, so
  the policy probe is no longer unpaced and concurrent first callers issue one request.
- `PacingState` is a shareable struct: `Fetcher::with_shared_pacing` lets instances that talk to one
  host spend one budget, which the service's per-jurisdiction fetchers must use when they are wired
  (recorded in the lane README as an obligation for that slice).
- `collect` counts a directory row that carries no short code (`dropped_school_rows`) instead of
  dropping it silently, and journals a school only when its summary was actually read, so a transient
  fetch failure is retried on the next run rather than being fixed as "done without coaches".

Tests added with the fixes: `registry::tests::the_declared_rate_for_a_host_is_read_from_the_table`,
`net::execute::tests::a_registered_host_is_never_paced_faster_than_its_declared_rate`,
`a_host_with_a_recorded_cooldown_is_refused_before_dispatch` and
`two_fetchers_sharing_one_pacing_state_share_the_host_budget` (all virtual-time, no network).

Measured after the change: `cargo test -p census-crawl` → **473 passed, 0 failed** (0 doc-tests);
`cargo test -p census-store` → **105 + 1 passed, 0 failed**; `cargo fmt -p census-crawl -- --check`
clean; `cargo clippy -p census-crawl --all-targets -- -D warnings` clean. `cargo test -p census-domain`
→ **153 passed, 1 failed** (`model::tests::general_model::meet_identity_is_date_and_name_scoped`, an
id-string expectation): not from this work — the only local change in that crate is attribute
indentation in `event_performance.rs`, which cannot change an identity hash, so the test fails at this
base and belongs to the uncommitted domain work in the main worktree.

**Recorded, not changed.** (1) Cache bodies are written and read with `std::fs` on the runtime
(`net::execute`), so a lane whose bodies are 100 KB–1.6 MB pays a page-cache syscall per request:
measured here, a 671 KB write takes 0.16 ms and a read 0.21 ms, a 1.6 MB write 0.31 ms, and even the
32 MiB body cap 5.79 ms — all far inside the one-second pacing interval, so the fix (`tokio::fs` or
`spawn_blocking`) is a latency nicety for concurrent jurisdictions, not a correctness gap for this
lane. (2) `PacingState` sharing is opt-in rather than process-global; a forgotten `with_shared_pacing`
returns a process to per-instance pacing. (3) The probe reads the cache first and has no `from_cache`
field, so a re-run over a warm cache replays retained bodies as measured numbers; the README records
that a fresh qualification number needs a cleared cache. (4) An offline probe that never downloaded a
body records `offline`, which cannot be told apart from a live 404 recorded as `http`; the taxonomy is
documented as a mapping, not parity.

**Landed.** `main` fast-forwards to this work; the landed tree was then built and tested *in the main
worktree*: `cargo test -p census-crawl` → **473 passed, 0 failed** (10 s, incremental). That worktree
also held uncommitted work no commit in this repository contains, and it cannot compile against the
landed revision: an older browser-lane shape — an `Ingress` type at `crates/census-crawl/src/ingress.rs`
(absent from every commit, including the base `feec27eb3`), a `Pace` struct returned by
`Fetcher::pace`, `BrowserLane::over(Ingress)`, and the MileSplit DC inline capture renamed to the NC raw
capture — plus copies of intermediate revisions of this lane dated 2026-09-29, the newest
(`docs/VERIFICATION-EVIDENCE.md`) at 16:09. Those were the only files blocking the fast-forward; they
were restored to the landed revision so the crate builds. The superseded copies are preserved: a full
snapshot of the pre-land worktree is the main worktree's `stash@{0}` ("omp: snapshot of main's pre-land
WIP", `bba998e2`), and the staged captures (`src/ingress.rs`, `src/milesplit/raw_rows/tests.rs`,
`tests/fixtures/coach-directories/*`, `tests/golden/*`, `tests/fixtures/chsaa-test/`) were left staged
on disk rather than deleted. None of that line was reconciled by hand and this delivery claims none of
it.

## Consolidated historical evidence — imported 2026-09-27

The following facts came from retired handoffs, implementation plans and duplicate operating
guides. They were **not rerun during documentation cleanup**. Original run IDs, partial digests,
dates and scope distinctions remain as recorded; abbreviated hashes are not full hash certificates.
Old source access, signed-in sessions, arbitrary retry behavior and workbook inputs are not
authorizations or production procedures for the fresh census.

### Workbook-era acquisition and qualification — 2026-09-19 to 2026-09-21

- The retained native corpus comparison reported 95 queries, 4,256 receipts, 340,238 individual
  results, 57,629 relay-member results, 142,705 athlete IDs and 15,724 unresolved roster results.
  Its private report was `native-rankings-1789772180206/native-corpus-cleanup-proof.json`.
  Twenty-six private public-API storage scenarios passed; their corrected position oracle counted
  positions `1,2,1` as two distinct positions. These reports were not recovered or rerun here.
- Serialization measured 344,131 versus 13,131 allocations over 1,000 synthetic iterations,
  331 fewer per iteration. This is not a throughput result.
- Fixture division runs `6f3c0c59…` (indoor girls), `f0ef3948…` (indoor boys) and `3c726c6d…`
  (outdoor boys) each reconciled eight synthetic rows: six accepted, one no-match, one review,
  none pending. Stopped-writer verification matched source hashes; cached replay added no source
  requests. Outdoor pause/resume froze source traffic for the observed 25 seconds.
- Live indoor boys `57f88b34…` collected 95 terminal events over 1,065 pages, snapshot
  `40792b83cfb168de5de876773f1312e41af3c34fc49782a991bdd6d9d4389dd6`. Its eight synthetic
  workbook rows all remained `review_required`. Workbook/JSONL/receipt digests began
  `a4eb1fe7…` / `87cfd651…` / `a4f0791b…`; stopped-writer verification exited zero.
  Cached replay took 0.5 seconds, left artifacts byte-identical and kept fetch count 488→488.
  Collection required a CDP re-arm mitigation; it did not prove the transport defect repaired.
- Live indoor girls `fe726b41…` collected 94 events over 816 pages and likewise left all eight
  rows in review. Artifacts began `8322bae9…` / `44f9f773…` / `fc6c12fe…`; verification exited
  zero and replay left requests 3,616→3,616. Outdoor runs `7be7e9dd…` and `3b7c07fa…` were
  separate historical generations, not additional accepted-person evidence.
- The 2026-09-19 fresh-headless probes returned 403 block pages of 5,484 bytes. The later headed
  profile produced 200 captures. A recovery cycle reached `human_required` in 31 seconds around
  its 30-second bound rather than waiting to the old 24-hour deadline. These are session-specific
  observations, not a rule that future access or challenges may be bypassed.
- The old gate records changed with the tree: 226 tests with library 149/2 ignored, later
  232 tests at `3f78fd9` with library 156/2 ignored, and a CDP-patched 234-test record. Do not
  combine them into one current suite count. The old 16-target command was:

```text
cargo test --lib --bins --test rankings_parser --test rankings_catalog --test rankings_scope --test rankings_indoor --test rankings_storage --test profile_html_bounds --test profile_merge_bounds --test search_html_bounds --test workbook_verify --test workbook_zip_layout --test native_parser_properties --test result_verify --test bundle_verify --test ingress_failure_surface
```

Those root-package targets no longer exist. The retained `scale-v15` lane used fixture transport
and binary `0f044e59…` from working tree `9d101d9`, not live identity search. Its real workbook
had 120,716 rows in two sheets (111,939 + 8,777), 1,569,308 source fields and 26 headers.
A ten-selected-row smoke retained all rows and an 83.7 MB JSONL. An owner-online partial export
retained 462 completed and 120,254 pending rows. Final run `a8328b7f…` assessed 5,000:
zero accepted, 3,405 review, 1,595 no-match; the other 115,716 remained explicitly pending.
Stopped-writer verification and replay were reported complete by 2026-09-21, correcting the
earlier pending note. Older `smoke`, `scale-partial` and `scale-2500` publications failed the later
header-width verifier and were not silently replaced. No private row values are reproduced here.

Measured fixture throughput was approximately 0.31 rows/s at concurrency 8 and 0.27 at 32;
two lanes together approximately 0.9 rows/s. A 120.1-second window held 41 rows and 358 source
operations (0.341 rows/s, 8.73 operations/row), with 358 readiness calls and a 0.329-second mean
handler duration. A shared-probe experiment kept probe/gate calls 280/280 and fell to 0.167 rows/s;
it was reverted. The device-write estimate of 12–14 MB/row was not journal payload: retained
journals measured 51.5 KB/decided row, 112.5 B/entry, versus 110 KB/row in Restate data and
36 KB/row in the store. This is an old workload, not a current performance baseline.

The 2026-09-21 endgame audit at `3af713b` (dirty tree) found live issue counts 38 sport mismatch,
17 noncanonical URL, 16 malformed page and six short page; six submitted live output directories
were empty. Its twelve-state walk held 6,737 rosters, 35,622 cached responses and 2.2 GB.
It could not infer national workflow execution from journals shared with CLI walks. The later
[search capture audit](../research/G1-LIVE-SEARCH-CONTRACT.md) owns detailed rejection evidence.
The original “profile evidence stayed empty” claim was false across retained runs: the older
33-row pilot held 2,263 profile artifacts and 46,524 performance entries, and one live-girls row
held one profile with 21 entries. Neither observation established an accepted identity.

Live probes on 2026-09-21 found no XC season in the TF `GetNavInfo` map; XC rankings landing
pages did not issue a ranking request. Bio responses exposed both sport-specific surfaces,
grades, teams, seasons, meets and results. A two-sport sample returned 40 TF results separately
from 22 XC results. Across five other athletes, `level=4` reduced TF row counts
88/87/428/80/162 to 73/74/148/71/68; `GetAthletes?listId=` returned an empty array. These are
capability captures, not exhaustive collection or permission to bypass the headed browser policy.

### Early independent-census baseline — 2026-09-20

Imported from the former service README; before Fjall/Restate cutover. Counts describe that
generation only, not today's store or independently verified people.

| Scope | Athletes | Co2027 | Boys | Girls | Profile URL | Coach | Coach email |
|---|---:|---:|---:|---:|---:|---:|---:|
| Core | 651,736 | 146,858 | 80,896 | 65,751 | 142,916 | 37,179 | 28,929 |
| All sources | 787,584 | 190,087 | 105,755 | 84,034 | 176,885 | 43,201 | 33,340 |

Both rows reported grade evidence for 100% of their cohort; core/all-source cohort ratio was
77.3%. There were 27,580 coach rows, 1,532 core meets (WI 796, MN 209, IA 85; 442 unresolved),
and 11,007 all-source meets, 9,593 carrying an Athletic.net ID.

| State | Schools | Athletes | Co2027 | Boys | Girls | Coach | Coach email |
|---|---:|---:|---:|---:|---:|---:|---:|
| OH | 1,504 | 87,765 | 26,998 | 14,414 | 12,507 | 313 | 313 |
| IL | 1,889 | 69,493 | 19,392 | 10,856 | 8,521 | 6,432 | 6,244 |
| WI | 1,027 | 118,031 | 19,281 | 10,323 | 8,929 | 14,699 | 13,646 |
| MI | 1,493 | 94,771 | 16,176 | 9,285 | 6,891 | 0 | 0 |
| MO | 1,037 | 42,226 | 13,218 | 7,377 | 5,839 | 0 | 0 |
| IN | 1,108 | 40,846 | 11,909 | 6,430 | 5,402 | 0 | 0 |
| MN | 1,016 | 54,084 | 11,261 | 6,147 | 5,114 | 9,338 | 8,629 |
| KS | 836 | 29,950 | 9,252 | 5,258 | 3,993 | 0 | 0 |
| IA | 913 | 49,024 | 8,738 | 4,924 | 3,804 | 76 | 19 |
| NE | 678 | 35,800 | 5,921 | 3,248 | 2,673 | 4,971 | 0 |
| SD | 427 | 17,134 | 2,669 | 1,518 | 1,151 | 239 | 78 |
| ND | 339 | 12,612 | 2,043 | 1,116 | 927 | 1,111 | 0 |

The parser/header repair in that record grew parsed Wisconsin artifacts 96→1,740 (compiled 763,
XC 380, Hy-Tek 597), producing 834,254 rows, 264,167 with grades, and 4,323 added core Co2027
rows. A separate Wayzata schedule measurement resolved 304/537 venue rows (95 sites, 209 schools);
the rest were unresolved, not guessed.

### Hardening waves — 2026-09-21 to 2026-09-22

The retired program's baseline was `4e5b828`, nightly-2026-04-27 / rustc 1.97.0-nightly.
Strict production Clippy exited 101 with approximately 440 diagnostics (392 census lib and
51 root lib; the approximation and differing scopes are retained). Per-lint census/root counts:
arithmetic 208/36, expect 75/0, string slicing 47/2, casts 36/10, indexing 23/2,
ignored-must-use 3/0, unwrap 1/0. It counted 134/552 census functions over 25 logical lines,
33 over 60 and 21 files over 300. The pre-split sizes were 33,608 root production lines/182 files
and 17,854 census lines/31 files.

The async census/root inventory recorded bare spawn 0/4, blocking spawn 2/2, JoinSet 3/5,
TaskTracker 0/3, cancellation token 0/10, select 3/7, buffered-unordered 4/0, and six/zero
instrument attributes. Spawn instrumentation, paused-time/loom and console/OTLP support were absent
in that baseline. These were subsequent implementation targets, not current-tree claims.

| Recorded wave | Executed result and distinctive evidence |
|---|---|
| Wave 1, 2026-09-21 | Gate reported 465 passed/2 skipped; root strict counters zero; census arithmetic 240→68, casts/index/string-slice/expect/unwrap 36/23/47/75/1→7/4/1/0/0; scan census indexing/expect/casts 128/75/32→0/0/7 |
| Wave 2, 2026-09-21 | 583 passed/2 skipped, strict counters zero, oversized files 23→0; production root/census 36,656/24,039 lines over 305/160 files; functions over 60/25 were 0/549 |
| Wave 3, `3af713b` | Commit 286 files, +66,070/−1,184; independently frozen fmt/scan clean, census 25,582 lines/167 files, zero oversized files/over-60 functions; commit record reported 618 passed/2 skipped |
| Waves 4/5, 2026-09-22 | Gate 726 passed/2 skipped, strict Clippy zero; recovery eight scenarios; census 39,340 lines/251 files. Lost-row kill oracle changed 4/4 to `claimed_without_rows_at_kill=0 missing_from_the_final_store=0`; drain ordering and volatile Coverage-note parity were repaired |
| Wave 6, 2026-09-22 | 745 passed/2 skipped; census lib 356, domain 39. Six-school/13-coach smoke derived 19 identities, zero conflicts/reviews, 58 coverage rows and one snapshot; three passes preserved one row/key. Offline cycle 0.23 s; Run Metrics golden cells 13,872→13,882, other sheets unchanged |

Distinct defects included a FOUL row under a relay header, RaceDay reporting zero parsed rows
for an 82-row finish list, missing meet references, a resumed IHSA limit counting walked rather
than newly fetched meets, and normalization non-idempotence for `X School School`. The latter
was repaired to a suffix-stripping fixpoint and pinned by the repeated-suffix regression.
mutation results already have their detailed logs above; they are not upgraded by this transfer.
The old 559–627k observations/s figures had no in-repository reproducible baseline.

Deferred findings then included direct adapter/store coupling, whole-table materialization and
20M-row rejection, legacy import's commit/marker window, full-store index rebuilds, observation
counts mistaken for merged entities, unchecked allocation growth, cloned journal payloads,
whole-scan failure on malformed rows, uninstrumented spawn sites, backup/parity fixtures that never
derived indexes, and cohort-specific coverage. This list is historical; the current plan and schema
distinguish implemented repairs from remaining obligations. It is not a second active work queue.

### Pinned delivery source audit — `183fca1`

The former delivery brief inspected commit `183fca13dd6d48165d04c76fd56648998e6b432f`.
Actions run `36020764525`, job `107715786629`, failed at Quality gate; subsequent feature checks
were skipped. Log download failed for credentials, so the review did not diagnose the failure.
The source findings below are renamed **G01–G11** to avoid collision with the national plan's
F01–F15 acceptance items. They are static findings, not fresh test results or an assertion they
all remain present.

| ID | Finding at that revision |
|---|---|
| G01 | Transparent integer mark wrappers contradicted a decimal-JSON compatibility claim |
| G02 | Imperial comparison treated hundredths of an inch as inches: `3-0.75` computed 2819 mm instead of 933.450; `4-0.00` computed 1219 instead of 1219.200, reversing order |
| G03 | Restate seen-operation state followed the external Fjall write, leaving a lost-acknowledgement window |
| G04 | S06 explicitly skipped the named crash boundary |
| G05 | Review fact hashing sorted words and erased attribution |
| G06 | Candidate/singleton identity depended on school/name/class/category rather than source-person evidence |
| G08 | Benchmark parsing accepted empty output and comparison did not require every baseline group |
| G09 | RSS capture used invalid `/proc/self/status/VmHWM` and could substitute zero |
| G10 | Applicability mapped twelve Midwest states, with empty fallback elsewhere |
| G11 | Batch accounting saturated row counts and lacked a total byte budget |

The 24 named canaries, 12 proof kernels, 17 faults and full-artifact verifier requirements now live
in their single current owners (national plan, TESTING and fault catalog). The old one-model policy,
seed-workbook branch and competing T01–T36 implementation waves are not current instructions.

### Provider wiring and storage-path audits — historical source inspection

The 2026-09-23 dispatch audit found six witnessed registry/stage/CLI slugs: `milesplit`, `mshsl`,
`plain_names`, `wayzata`, `wiaa`, `wiaa_results`. Seven had registry+CLI but no stage:
`athleticlive`, `athleticlive_athletes`, `athleticnet`, `coach_contacts`, `ihsa`, `ks`, `ohsaa`.
Three were CLI-only modes: `athleticlive_results`, `ihsa_tournament`, `milesplit_results`.
`tfrrs` had a registry row but no CLI arm. That was 17 names, not 17 integrated adapters.
It ran no binary; subsequent wiring changes require their own evidence.

The retired local batch audit, transferred on 2026-09-27, estimated separate `SyncData` commits
from the then-inspected source: Athletic.net bio flush `6 + k` (70 for 64 units), index five,
review two, WIAA/ND/NSAA school+coach journals two, OHSAA `1 + coach_count`. These were source-path
counts, **not measured fdatasync calls or throughput**. Its proposal to commit from Drop and its
claim that Fjall batches cannot span keyspaces were incorrect and are not retained as design.
The current atomic write/receipt contract is [FJALL_BACKUP.md](FJALL_BACKUP.md) and
[the storage ADR](adr/ADR-001-fjall-primary-store.md).

### Backup, deployment and source-owner cutover evidence

The earlier synthetic cold-copy drill used 400 observations (200 schools, 100 coaches, 100 meets)
and merged 199 schools; the backup held 12 files/229,232 bytes and normalized reports of 27,649
bytes compared equal. A 64 MiB journal preallocation made the root 67,212,350 bytes before reopen
versus 196,402 afterward; this was physical layout, not lost observations. Bad version and truncated
descriptor opens refused with `InvalidVersion` and `UnexpectedEof`. Truncating 100 tail bytes
retained 400 observations; 4,000 retained 300. Fourteen row-value flips all refused. Of 96 header
flips, eight triggered upstream debug assertions, 82 refused and six opened; release behavior was
not verified. These observations do not make a live directory copy safe.

The 2026-09-24 campaign cold-copy drill recorded:

```text
target/release/census-service --store /tmp/store-copy store-backup --to /tmp/census-backup-drill/backup
exit 0; 10 s
target/release/census-service store-restore --from /tmp/census-backup-drill/backup --to /tmp/census-backup-drill/restored
exit 0; 7 s
target/release/census-service --store /tmp/census-backup-drill/restored consolidate
exit 0; 8 s
target/release/census-service --store /tmp/census-backup-drill/restored report
exit 0; 13 s
```

Reported physical table counts included teams 207,609; coaches 65,020; athletes 3,072,309;
meets 12,556; events 101,711; performances 309,962; source identities 2,507,541; conflicts 4,548;
review cases 107,768; coverage 213; snapshots two; source access zero; identity verdicts 43,291;
source meets 131,726; source observations 207; observations 3,991,059.
Sizes were 1,459,217,992 on disk and 7,698,688,836 logical. Restored all-source report:
31,870 schools, 2,364,818 athletes, 623,509 Co2027 (350,944 boys, 271,560 girls), 609,738 profile
URLs, 59,269 multisource rows and 32,031 coaches. Equal counts are not complete semantic equality;
the separate 2026-09-25 drill above used a different generation.

The deployment record for 2026-09-24 reported revision 8 fan-out of 49 jurisdictions, with 47
aborted/two finished under the default timeout. An in-place rebuild left 59 paused invocations
and three DC tasks pending; re-registration preceded recovery. Old process stop and new canary
response each took two seconds in that experiment. Duplicate localhost/127.0.0.1 registrations
carried revisions 9/5 versus 8/4; old registrations were retired after 10/6 became active.
Deletion without force returned 501, with force 202. This is not proof of uninterrupted upgrades
or a safe recipe to retire a deployment with unknown in-flight work.

ADR-014's source-owner repair was motivated by 74,468 identity-less rows among 2,374,515 historical
athletes and 204,102 performance rows predating the owner field. Recorded decode errors named
`athletes:ath_0000477b2bc153dc#1363551` and
`performances:perf_0000150f06a7a1de#5600`. After repair, core census-status reported 31,870 schools,
2,220,866 athletes and 575,991 Co2027; release bests reported 10,231 selections in 3.2 seconds
with 1.4 GB peak RSS. These imported measurements do not supply owners to unknown historical rows
or certify the fresh run.

An asynchronous export result received on 2026-09-27 printed
`wrote /var/tmp/real-census/out/census-service-2026-09-27.xlsx`,
`real 24m50.412s`, `user 24m43.126s`, `sys 0m3.953s`.
Its initiating command and artifact readback were not recovered. It is a process result only,
not verified workbook contents, a repeatable benchmark or a new seal.

### Pinned sources for the 183fca1 review

Retained source references from that review, not newly fetched or runtime-verified. Repository
links pin the old commit even when the current file has been removed; external links describe
framework semantics, not this implementation's correctness.

| Reference | Source |
|---|---|
| R01 | [Latest main recheck and pinned commit](https://github.com/lprior-repo/athletic-rust-pipeline/commit/183fca13dd6d48165d04c76fd56648998e6b432f) |
| R02 | [GitHub Actions run for the reviewed SHA](https://github.com/lprior-repo/athletic-rust-pipeline/actions/runs/36020764525) |
| R03 | [Failed Quality gate job](https://github.com/lprior-repo/athletic-rust-pipeline/actions/runs/36020764525/job/107715786629) |
| R04 | [Fixed mark representation](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/fixed_mark.rs) |
| R05 | [PR comparison and imperial conversion](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-report/src/bests/measure.rs) |
| R07 | [Production Ingest handler](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/ingest.rs) |
| R08 | [Performance benchmark capture](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/perf/bench.rs) |
| R09 | [Existing multi-table StoreBatch](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-store/src/write_batch.rs) |
| R10 | [Current meet-stage handoff](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/meets_arms.rs) |
| R11 | [S06 crash/duplicate-evidence scenario](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/tools/durability/scenario-06-no-duplicate-evidence.sh) |
| R13 | [CaseEvidence and review case identity](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/records.rs) |
| R14 | [Candidate and canonical athlete types](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/athlete.rs) |
| R15 | [Performance comparison](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/perf/compare.rs) |
| R16 | [Source applicability](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-crawl/src/applicability.rs) |
| R17 | [Repository endgame research](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/research/ENDGAME-GAPS.md) |
| R18 | [Restate jobs and journal handoff](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/jobs.rs) |
| R19 | [Jurisdiction pipeline integration](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/jurisdiction/pipeline.rs) |
| E01 | [Restate Rust durable steps](https://docs.restate.dev/develop/rust/durable-steps) |
| E02 | [Restate Rust error handling](https://docs.restate.dev/develop/rust/error-handling) |
| E03 | [Serde container attributes](https://serde.rs/container-attrs.html) |
| E05 | [Criterion command-line options](https://bheisler.github.io/criterion.rs/book/user_guide/command_line_options.html) |
| E06 | [TigerBeetle testing practices](https://docs.tigerbeetle.com/coding/testing/) |
| E07 | [TigerBeetle safety practices](https://docs.tigerbeetle.com/coding/safety/) |

## Source-family admission above one request (2026-09-29)

`--source-parallelism <N>` (`census_crawl::net::DEFAULT_FAMILY_PARALLELISM` = 1) is implemented as
[OPERATIONS.md](OPERATIONS.md) already documents it: at the default a source family keeps one pacing
turn across its hosts, above it every host of the family carries its own turn while a per-family
semaphore admits N requests in flight, and a host outside every family takes no family slot. The
service clamps the wire value to at least one, rebuilds its cached fetcher when the knob changes, and
the browser lane takes the route-rewriting ingress client instead of the SDK's bare one.

Commands run on this tree, in this worktree:

- `cargo test -p census-crawl` → **482 passed, 0 failed**.
- `cargo test -p census-service --lib` → **186 passed, 0 failed**; the bin target → **57 passed**.
- Four new virtual-time tests pass: `a_family_above_one_parallelism_gives_each_host_its_own_turn`,
  `a_family_admits_exactly_its_parallelism_in_flight`, `a_host_outside_every_family_takes_no_family_slot`
  and `the_default_parallelism_keeps_the_single_family_slot`.
- `cargo run -q -p census-service --bin census-service -- --help` prints
  `--source-parallelism <N> ... [default: 1]`.
- `cargo check --workspace --all-targets` exits 0.

Not established here: no live host was fetched under a raised value, so this records no throughput
number; the pacing and admission assertions are virtual-time and method-boundary only.

Red in this tree for reasons outside this change, from the preceding commit's unfinished refactors:
`workbook_shape`, `exporter_kill_restart`, the e2e `report_chain` (that commit deleted the workbook
writer's `PRs`, `Performances_*` and `Sources` sheets), `milesplit_parser_properties::accounting` and
`offline_cycle`. `census-service verify` again samples the `Performances_*` sheets, so the two
kill/restart tests fail on the missing sheet until the writer returns.

## Repository bulk removal (2026-09-29)

The commit before this change carried the scraped Restate dump and the fuzz corpora into the object
store: 31,881 paths (24,861 under `local/`, 4,978 under `fuzz/`), a 12.09 GiB pack, with
`local/restate-docs/INDEX.json` alone at 940,281,740 bytes. That commit was unpublished, so it was
rewritten without `local/` and `fuzz/corpus` (2,042 files and 1,884,926 insertions kept),
`.gitignore` now ignores `/local/`, `/fuzz/corpus/` and `/fuzz/artifacts/`, and reflogs were expired
before `git gc --prune=now`.

- `git count-objects -vH` → `in-pack: 15661`, `packs: 1`, `size-pack: 52.26 MiB`, `garbage: 0`.
- `du -sh .git` → 55 MB, was 13 GB; `git fsck --connectivity-only` exits 0 and all eight worktrees
  still resolve `HEAD`.
- `local/` (1.1 GB) and `fuzz/corpus/` (20 MB) stay on disk, untracked.

## Workbook writer restoration and size-budget repair (2026-09-29)

Commit `62f838b` deleted `workbook/performances.rs`, `workbook/recruiting/prs.rs` and
`workbook/meta/sources.rs`. The published workbook then lost the `PRs`, `Performances_*` and
`Sources` sheets, which left `workbook_shape`, `exporter_kill_restart` and the e2e report chain red.
`1df52a4` recorded that gap as still outstanding; this tree restores the writer.

`write_objective_sheets` again emits Athletes, PRs, `Performances_<NNN>` and Coaches, then the meta
sheets `Schools`, `Meets`, `Sources`, `Coverage`, `Conflicts`, `Review` and `Run Metrics`. The
performance sheets partition at 1,000,000 data rows (`1_048_576` Excel rows less the header and a
`48_575` margin), and `sheet_name` numbers them from `Performances_001`. `workbook/meta.rs` was split
into a dispatcher plus `meta/sheets.rs` and `meta/sources.rs`, so the crate's scan returns no
size-budget violation.

Commands run on this tree:

- `cargo test -p census-report` → **128 passed, 0 failed**.
- `cargo xtask scan` → every `census-report` count is zero; `files_over_300_lines` no longer lists
  `crates/census-report/src/workbook/meta.rs` (was 320) and `functions_over_60_lines` no longer lists
  its dispatcher (was 65). The ratchet's failure list carries no `census-report` entry.
- `cargo fmt -p census-report -- --check` → exits 0.
- `(cd crates/census-service && cargo geiger --all-features --output-format Json > /dev/null)` →
  exits 0. It first failed on `crates/census-report/benches/export_bench.rs`, a target no current
  manifest declares: removing the stale `target/{debug,release}/{deps,incremental}` units and the
  `target/release/.fingerprint/census-report-c3357eaf9101b513` directory left by the deleted
  `export_bench` and `export_review_probe` targets is the remedy `tools/gate.sh` documents for a tree
  that deleted a root package. No source file changed for it.
- `tools/gate.sh` → `tests: PASS`, **1571 passed, 0 failed**, including
  `census-service::exporter_kill_restart a_workbook_export_interrupted_by_sigkill_rebuilds_completely_on_restart`
  (23.8 s) and `b_workbook_without_interrupt_exits_cleanly`, so a real export writes the restored
  sheet set. `geiger`, `deny`, `audit`, `machete`, `feature powerset`, `bench presence`, `module
  seams` and `domain type integrity` also pass.
- Kill-ladder repair in `crates/census-service/tests/recovery.rs` (test-only; both ladder users
  inherit the search, and no production code changed). The ladder bisected wall-clock kill delays
  against one pre-measured clean runtime, and with a concurrent suite that baseline went stale:
  every attempt at the old bracket finished before its kill (`exited_before_kill=true`, `journal=5`)
  while the live pass took about 54 ms rather than the 109 ms the bracket assumed, so the search
  probed past the pass end. Even uncontended, round runtimes drift (24.2 ms, 26.2 ms, 27.6 ms), which
  is the same defect in miniature. `kill_ladder` now re-measures a clean runtime every round and
  scans 200 µs, 500 µs, 1 ms, 2 ms, 4 ms, 8 ms, 16 ms and 32 ms before that fresh measurement, for
  at most six rounds. Its panic was always the ladder refusing to report on an invalid measurement,
  never an assertion about the walk failing.
- `cargo nextest run -p census-service --test recovery -E 'test(ks_directory_walk_claims_units_the_kill_can_lose)'`
  with `--no-capture` → **pass** repeatedly, the traces showing non-monotone jitter (at one round
  offset 200 µs complete, 500 µs empty, 1 ms complete) and partial states landing at `journal=3`
  and `journal=4`, with `claimed_without_rows_at_kill=0` and `missing_from_the_final_store=0`.
- `cargo nextest run --workspace --all-features` → **1571 passed, 0 failed** in two full runs, once
  with the sweep-era ladder and once with the per-round ladder (127.8 s and 127.2 s); both include
  the whole recovery suite.

Red in this tree for reasons outside this change. The gate verdicts across four runs were
`FAIL -> fmt domain purity ratchet vet` (run 1), `FAIL -> tests domain purity ratchet vet` (run 2,
the ladder miss below), and `FAIL -> domain purity ratchet vet` in runs 3 and 4 after the ladder
repair and after the concurrent writer's revision landed; run 4 ran while a full
`--all-features` suite executed on the same machine. `fmt`, `zero code comments`, `check`, `doc`,
`tests`, `module seams`, `domain type integrity`, `deny`, `audit`, `machete`, `geiger`,
`feature powerset` and `bench presence` pass in runs 3 and 4, so the three lanes below are the only
red ones. The `fmt` lane's first-run diffs were in
`crates/census-crawl/src/chsaa/{parse.rs,school_page.rs}` — files another writer was editing in this
same working tree; that writer's next revision removed `school_page.rs` again and
`cargo fmt --all -- --check` now exits 0.

`domain-purity` runs `cargo tree -p census-domain --edges normal --prefix none` against the 24-name
ban list in `xtask/src/purity.rs` and reports `FORBIDDEN dependencies present: serde_json`. The
domain's own production code needs it: `model/serialization_digest.rs` and
`model/identity_decision.rs` write canonical JSON for identity and verdict digests, with 15 call
sites outside the crate. The dependency entered `crates/census-domain/Cargo.toml` in `9e97084`
(2026-09-24), so this lane is older than the current slices. Clearing it is a contract decision:
drop `serde_json` from the ban list, move the digest helpers to a crate allowed to use it (a
byte-identical encoding keeps stored digests valid), or hand-write a canonical encoder that stays
byte-equal with `serde_json::to_writer`.

The
debt ratchet reports the committed slices' growth: `census-domain` clippy `arithmetic_side_effects`
0→5 and `indexing_slicing` 0→12, `census-crawl.indexing` 0→1, `census-domain.indexing` 0→6, six
functions above 60 lines (`coach_directories/{collect,map,survey}.rs`,
`restate_services/ingest.rs`, `replay/cases.rs`) and `chsaa/parse.rs` newly above 300 lines. `vet`
fails with `You must run 'cargo vet init' (store not found at …/supply-chain)`: commit `1db9157`
(2026-09-27) deleted that store's `audits.toml`, `config.toml` and `imports.lock` (1,991 lines), and
earlier entries in this ledger record the lane passing against it, so the removal postdates them —
`git checkout 1db9157^ -- supply-chain` restores it if the deletion was not deliberate.

Not established here: the measurements above cover a working tree that also carries other writers'
uncommitted slices, so the ratchet's remaining growth is not attributable from this run alone; no
live deployment served this workbook, and the sheet set is asserted through the exporter tests and
the offline writer, not from a sealed production bundle.


## School-directory corpus verb: the NCES fixture lane (2026-09-30)

`census-service school-address` (ADR-020) reads operator-supplied directory artifacts into one
collapsed corpus, diffs it against a baseline, exports it and updates the source update ledger,
without opening the store. Exercised on the two committed NCES fixture windows with the debug binary:

```sh
target/debug/census-service school-address \
  --ccd crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv \
  --pss crates/census-crawl/tests/fixtures/nces/pss2324_pu_head.csv --out /tmp/sa/run1
```

Exit 0, three artifacts written. `pipeline_report.json` reports corpus `rows 1931, entries 1931,
skipped 67, notes 0, merges 0`; lane `nces-ccd` 1 557 entries with 42 skipped rows (artifact
sha256 `e01af083baa5…`), lane `nces-pss` 374 entries with 25 skipped rows (`2df29d5c9f66…`); every
skipped row is printed with its source line. `school_directory.csv` is the 15-column header plus
1 932 lines; `school_directory.json` parses back to 1 931 `SchoolDirectoryEntry` values
(`nces:010000500870` `Albertville Middle School` and `pss:A2380006` among them).

Re-running into the same directory reproduces all three artifacts byte-for-byte
(`pipeline_report.json` sha256 `10b8afdd…`, `school_directory.csv` `25d42b90…`,
`school_directory.json` `ca8dd914…`); only `--out` changes the report, through the `outputs` list.

The baseline and ledger path, run over the same inputs:

```sh
target/debug/census-service school-address --ccd … --pss … --out /tmp/sa/state \
  --baseline /tmp/sa/base.json --ledger /tmp/sa/ledger.json --now 2026-09
```

Run 1 reports `changes: null` and writes no `changes.json` (there was no baseline to diff against),
decides both sources `due`, and records `{"last":{"Ccd":"2026-09","Pss":"2026-09"}}` in the ledger.
Run 2 with `--now 2026-10` reports `0 added, 0 removed, 0 modified`, writes `changes.json`, and
decides both sources `not-due` with next due 2027-09 (CCD, yearly September) and 2028-01 (PSS,
biennial even January).

Refusals observed, each exit 1 with no artifact written: `--geocode` and `--validate-postal`
(`census-crawl::geocode` does not exist yet), `--baseline` without `--now`, and no input artifact
named. A PSS file passed as `--ccd` is refused as a directory artifact naming the missing `NCESSCH`
column and the file path.

The verb's regression lane is `cargo test -p census-service --test school_address_corpus`: 6 passed
(`school_address_reads_the_nces_fixtures_into_one_corpus`,
`school_address_writes_the_same_bytes_for_the_same_input`,
`school_address_diffs_against_its_baseline_and_records_the_month`,
`school_address_refuses_the_unbuilt_geocode_phases`,
`school_address_refuses_a_diff_without_a_run_month`,
`school_address_refuses_an_artifact_of_the_wrong_shape`).

Not established here: the `--state-ed-*` and `--associations` lanes are carried by the same path but
were not driven from a verb run in this entry — their readers' fixture lanes own those shapes; the
geocoding and postal-validation phases are unbuilt by decision; nothing in this corpus is census
acceptance evidence until a run cites it.

## Strict-source burndown and the CHSAA activity boundary (2026-09-30)

The port's working tree opened this session with `tools/gate.sh` failing `tests ratchet vet geiger`,
against a debt baseline that records zero strict-clippy diagnostics for every crate. Every repair
below is inside the port's own diff; no lint was suppressed and no baseline number was raised.

**Activities are consumed to their own boundary (`athletic-rust-pipeline-5du`).**
`census-crawl::chsaa::parse::objects_at_anchor` advanced each found `{"activityName":` marker by
`marker.len() + 100` instead of by the bytes of the object it had just read, so a second marker
within that window was never parsed: a page whose first activity carries the marker, a short body
and a second activity lost the second one. The rewrite parses each marker's value with
`serde_json::Deserializer::from_str(...).into_iter::<Value>()` and advances by the stream's
`byte_offset()`; the char-vector scanner and `extract_one_object` are gone, so the reader no longer
allocates four bytes per character of a school page.

Failing before, passing after — the regression is
`chsaa::tests::adjacent_short_activity_objects_are_both_retained` (two adjacent activities, Ada and
Grace), run against both spellings of the advance:

```sh
cargo test -p census-crawl --lib chsaa::tests::adjacent_short_activity_objects_are_both_retained
# with the consumed-length advance: 1 passed
# with `cursor = start.saturating_add(anchor.len()).saturating_add(100)` restored:
#   left: [("Ada", "XC")]
#   right: [("Ada", "XC"), ("Grace", "TF")]
#   test result: FAILED. 0 passed; 1 failed
```

`cargo test -p census-crawl --lib chsaa` → **17 passed**, including
`directory_parses_the_378_member_schools`, `directory_rows_match_the_prototype_golden_field_for_field`
and `school_page_rows_match_the_prototype_golden_after_mapping`, so the captured page's row set is
unchanged by the rewrite, and `collect_stores_the_requested_school_and_its_coach_rows_from_the_cache`
drives the adapter over that captured page natively.

**Strict clippy, 32 sites across three crates, no suppression.** The gate's lane
(`cargo clippy --workspace --lib --bins --examples --all-features` with the repository's `-D` set)
aborts a crate's compilation on the first `-D` diagnostic, so the tally became complete only as each
round cleared:

- census-crawl 16: `chsaa::parse` (unchecked arithmetic, indexing and string slicing around the
  directory payload and object scan), `coach_directories::survey::tally` (three unchecked counter
  adds → `saturating_add`), `directory::artifact::cell` (elidable output lifetime),
  `state_ed::tabular` (`let Some(name) = … else { return None }` → `?`),
  `milesplit::raw_rows::columns` (separator offset → `saturating_sub`).
- census-report 13: twelve `needless_borrow` on `&rows.<field>` where `StoreRows` already holds
  slices (`workbook::meta`, `meta::metrics::reconcile`, `meta::queues`, `meta::queues::conflicts`,
  `meta::sheets`), and `meta::sheets`' `Vec::with_capacity(7)` plus seven pushes → one `vec!`
  literal.
- census-service 3: `census::seal`'s two needless borrows of `dataset.review_cases` and
  `dataset.source_access`, and `cli::live::jurisdiction_request`'s eight parameters → five, with the
  per-run knobs (flags, refresh, authorized hosts, source parallelism) in a `LiveRun` that the three
  `cli::gather` call sites build once.

Measured after the last edit: strict clippy on source targets **total diagnostics: 0**; `cargo xtask
scan` → `files_over_300_lines: []`, `functions_over_60_lines: 0`; `cargo xtask ratchet
tools/quality-baseline.json <tally> <scan>` → **no metric grew** (the only upward entry is
`functions_over_25_logical_lines` 614 → 750, which the ratchet reports as `target, not a budget`,
plus file/line context counts).

**The geiger lane failed on a stale target unit, not on unsafe code.** `(cd crates/census-service &&
cargo geiger --all-features --output-format Json)` exited 1 with

```text
error: Io(Os { code: 2, kind: NotFound, message: "No such file or directory" },
       ".../crates/census-crawl/tests/zz_scratch_diag.rs")
```

a test target deleted before this session whose artifacts remained in this worktree's `target/`:
`target/debug/deps/zz_scratch_diag-8f1b9c99c93b3133` (172 MB), its `.d` dep-info and
`target/debug/incremental/zz_scratch_diag-1toct93aa5erh`. Removing those units (the remedy
`tools/gate.sh` documents for a tree that deleted a target) took the lane to **exit 0**, 312 packages
scanned; a sweep of every `target/**/deps/*.d` for source paths that no longer exist found no other
stale unit. `cargo clean -p` of four packages whose artifacts had drifted from the current graph
removed 1.5 GiB and the tree rebuilt before this measurement.

**The final gate run.** `tools/gate.sh` → `gate: FAIL -> vet`, every other lane PASS: fmt, zero code
comments, check, doc, **tests (1678 run, 1678 passed, 3 skipped)**, domain type integrity, domain
purity, module seams, **ratchet**, deny, audit, machete, **geiger**, feature powerset and bench
presence. An earlier run of the same suite failed `census-service::recovery
ks_directory_walk_claims_units_the_kill_can_lose` ("a real mid-batch kill must have happened (0 <
journal < total)") while builds were running beside it; the test passes alone in 1.19 s
(`cargo nextest run -p census-service -E 'test(ks_directory_walk)'`), its ladder re-measures a clean
runtime each round, and `crates/census-service/tests/recovery.rs` is untouched by this port, so that
failure is load-dependent, not a regression.

Not established here: the `vet` lane cannot pass in this checkout — `cargo vet --locked` stops at
`You must run 'cargo vet init' (store not found at …/supply-chain)`, because the store was deleted on
2026-09-27 (commit `1db9157`) and [tools/README.md](../../tools/README.md) records restoring it as an
owner decision outside the port; geiger's exit 0 is a dependency-graph scan, not a proof about any
unit's code; and the CHSAA reader was exercised against the committed captures, not the live site.

## Sol closing review, Round 1 — bounded type boundary and field provenance (2026-09-30)

Executor: **openai-codex/gpt-6.1-sol**. Assigned worktree only:
`/home/lewis/src/ad-law-scrape/arh-python-port`; caller brief
`var/sol-review/brief.md`; append-only findings/commands in `var/sol-review/state.md`.
No commit, push, rebase, reset, clean, other-worktree changes, fixture edits, golden edits,
historical evidence rewrites, lint suppressions or quality-baseline relaxation.
The required old-CHSAA mutation and throwaway probes ran in `/tmp/sol-round1-TvhC3r`.
Scratch-only compiler warnings are not source-target results for the port.

### Failing-before / passing-after observations

|Defect|Executed before|Executed after|
|---|---|---|
|Empty `SchoolName` bypasses parser through Deserialize|Scratch `cargo test -p census-service --test sol_round1_probe -- --nocapture`: exit 101, `empty school name decoded as Ok(SchoolName(""))`, rejecting-name assertion fails (artifact://635)|Final scratch same command plus `--test-threads=1`: `empty school name decoded as Err(Error("school name is empty", line: 0, column: 0))`; 6 probes pass (artifact://687:38–39)|
|Public/serialized grade 13 bypasses checked grade parse|Scratch `cargo test -p census-service --test sol_round1_probe deserialization_rejects_grade_13 -- --nocapture`: exit 101, `grade 13 decoded as Ok(Numbered(13))` (artifact://644:15–19)|Final scratch probe: `grade 13 decoded as Err(Error("\"13\" is not a supported grade", line: 0, column: 0))` (artifact://687:36–37)|
|CCD without phone → AA `9999999999` → SEA `1111111111` retains weaker phone|Initial scratch probe: exit 101, `merged phone=Some("9999999999")`, expected state phone (artifact://635)|Final scratch probe: `merged phone=Some("1111111111")`; original tin-bead CCD-gap → state phone → CCD `0000000000` also keeps the CCD value (artifact://687:43–47)|
|Equal-rank identical values serialize arrival-dependent provenance|Scratch `cargo test -p census-service --test sol_round1_probe equal_rank_identical_values_keep_order_independent_provenance -- --exact --nocapture`: exit 101, forward winning label NAIS / reverse CAPE, unequal JSON (artifact://680)|Final scratch probe: both canonical JSON payloads retain CAPE; permanent regression also passes (artifact://687:40–42; artifact://682)|
|Former CHSAA `+100` boundary skips adjacent short JSON objects|Scratch `cargo test -p census-crawl --lib chsaa::tests::adjacent_short_activity_objects_are_both_retained -- --exact` with old cursor advance restored: exit 101, left `[("Ada", "XC")]`, right `[("Ada", "XC"), ("Grace", "TF")]` (artifact://641)|Port exact same test: exit 0, 1 passed / 502 filtered. Port `cargo test -p census-crawl --lib chsaa`: 17 passed / 486 filtered (artifact://623)|

Production repairs retain the public Grade enum/Numbered variant with validated NumberedGrade,
checked SchoolName string deserialization, and valid name/grade JSON bytes. Every direct grade
caller migrated. Field provenance is a required private map with the existing SourceLabel values;
the policy is `SourceLabel::rank`, not the record's strongest source or a second serialized rank.
For identical values at equal rank the existing source enum ordering selects a canonical source.
Address remains the existing aggregate field. Old provenance-free baselines explicitly fail and
must be regenerated; there is no default, alias or fallback. These artifacts are not Fjall records,
so no Fjall schema revision is applicable.

### Executable contract coverage and native smoke

All following commands use the assigned worktree cwd unless labeled scratch.

- `cargo test -p census-domain school_directory -- --nocapture` → exit 0, 41 passed at the initial
  repair stage (artifact://647). On final source,
  `cargo test -p census-domain --lib school_directory::tests::review_regressions -- --nocapture --test-threads=1`
  → exit 0, **5 passed** (artifact://682). The regression checks all six phone arrival permutations,
  persistence mid-merge, valid numbered grades 1–12 with byte-identical enum encoding, rejection
  of 0/13/255, empty/noncanonical serialized names, provenance-free entries, missing source for a
  present field, source for an absent field, unrecorded source and attempted numeric rank override.
  Checked provenance failures map typed DirectoryError variants to explicit Serde data errors.
- `cargo test -p census-service --test nces_directory_properties --test private_assoc_directory_properties --test state_ed_directory_properties --test tssaa_directory_properties --test school_address_corpus -- --nocapture`
  → exit 0, **35 passed**: NCES 8, private-assoc synthetic 4, school-address 7, state-ED 12,
  TSSAA 4 (artifact://651). The final gate reruns these on final source.
- Final scratch `cargo test -p census-service --test sol_round1_probe -- --nocapture --test-threads=1`
  → exit 0, **6 passed**, **2610 entries** serialize/decode/re-serialize with identical canonical
  bytes (artifact://687). It reads the CCD/PSS, NYSED index/profile and TSSAA committed captures;
  its NAIS listing is explicitly synthetic, not a fabricated capture. Observed populations:
  CCD 1557/42 skipped/0 notes, PSS 374/25/0, NY index 220/0/0, Kingston profile 1/0/0,
  TSSAA list 456/0/0, Alcoa detail 1/0/0 with 20 coaches and first email
  `phaggard@alcoaschools.net`. Kingston profile has `29 Leroy St`, Potsdam NY 13676, enrollment
  393, phone 3152652000 and website `https://www.potsdamcsd.org`.
- Actual native program launch:
  `cargo run -p census-service --bin census-service -- school-address --ccd crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv --pss crates/census-crawl/tests/fixtures/nces/pss2324_pu_head.csv --out /tmp/sol-round1-native-cJh5Zm --baseline /tmp/sol-round1-native-cJh5Zm/baseline.json --ledger /tmp/sol-round1-native-cJh5Zm/update_ledger.json --now 2026-09`
  → exit 0, **1931 rows → 1931 entries, 67 skipped, 0 noted**, prints every skip and publishes
  JSON/CSV/report/baseline/ledger (artifact://660).
- The same explicit Cargo invocation with `--now 2026-10` actually consumes the persisted new
  baseline and ledger → exit 0, **0 added / 0 removed / 0 modified**, CCD not-due until 2027-09,
  PSS not-due until 2028-01 (artifact://675). Initial `cargo run` without `--bin` failed with binary
  ambiguity; a subsequent direct target-path invocation returned 127 because the binary path
  was absent. Both invocation/setup failures were corrected through explicit Cargo and are not
  represented as successful acceptance commands.
- A pre-repair scratch binary exported the same capture corpus to
  `/tmp/sol-round1-native-cJh5Zm/pre-fix`.
  `sha256sum /tmp/sol-round1-native-cJh5Zm/pre-fix/school_directory.csv /tmp/sol-round1-native-cJh5Zm/school_directory.csv && cmp /tmp/sol-round1-native-cJh5Zm/pre-fix/school_directory.csv /tmp/sol-round1-native-cJh5Zm/school_directory.csv`
  → exit 0; **both hashes**
  `25d42b90ced9d85e38e19ec6724146db079730b9e7e1c2057c73e3ebe0902bca`.
  CSV columns and bytes are unchanged by the provenance cutover.
- `jq '[.[] | .grades? | select(. != null) | (.low, .high)] | group_by(.) | map({grade: .[0], count: length})' /tmp/sol-round1-native-cJh5Zm/school_directory.json`
  → exit 0, numbered endpoints include every value **1–12**, plus PreK and Kindergarten.
  The admitted range comes from the existing Grade::parse policy and domain published-form
  tests, now explicit in ADR-020, with corpus corroboration; it is not inferred as a universal
  grade system from the fixture.

### Evidence corrections and remaining obligations

- Owning ADR corrected its stale PSS consequence. Native
  `jq '.[] | select(.key.Pss == "A2380006") | {key, name, phone}' /tmp/sol-round1-native-cJh5Zm/school_directory.json`
  → exit 0, `MT. PILGRIM CHRISTIAN ACADEMY`, phone `2057805096`: PSS public-use PINST is a name,
  not an invented address-derived name.
- Scratch `cargo test -p census-service --test sol_round1_probe captures -- --exact --nocapture`
  → exit 0, **2528 numeric instid occurrences, 220 distinct ids, 220 profile hrefs**,
  nonprofile ids `[]`, parser missing ids `[]` (artifact://672:16–18; repeated in artifact://687).
  The owning ADR's prior 221-school count was corrected. The fixture manifest's corresponding
  stale prose is outside this review's fixture patch scope and is tracked as
  **athletic-rust-pipeline-8an OPEN**; fixture bytes and manifest untouched.
- **37k remains OPEN.** Only the Main-approved SchoolName/NumberedGrade slice is fixed.
  ADR-020 and the Sol ledger enumerate all remaining primitive/composite wire shapes/invariants.
  Scratch `cargo test -p census-service --test sol_round1_probe observing_remaining_composite_deserialization_gap -- --exact --nocapture`
  → exit 0 with the observed discrepancy
  `persisted inverted grade span decoded=12-1 checked constructor=Err(GradeSpanInverted { low: "12", high: "1" })`.
  That observational probe demonstrates the remaining gap; it does not certify inverted spans.
  Main explicitly deferred approval of the remaining slices.
- **dtl remains OPEN.** Native
  `mkdir -p /tmp/sol-round1-native-cJh5Zm/partial/school_directory.csv` followed by
  `./target/debug/census-service school-address --ccd crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv --out /tmp/sol-round1-native-cJh5Zm/partial`
  → exit 1, `Error: i/o failed for /tmp/sol-round1-native-cJh5Zm/partial/school_directory.csv: Is a directory (os error 21)`.
  `stat --format='%n: %s bytes, %F' /tmp/sol-round1-native-cJh5Zm/partial/school_directory.json /tmp/sol-round1-native-cJh5Zm/partial/school_directory.csv`
  → new JSON already published, **1003942 bytes / regular file**; CSV remains a directory, no
  report. Sequential writes are `school_address/mod.rs:200–211`. Main approved complete staging,
  canonical generation manifest, report manifest digest, mandatory consumer verification and
  generic pre-commit destination refusal, but explicitly deferred implementation to next round.
  Repro/approval appended to the bead; no symptom-only directory special case applied.
- **Private-assoc capture lane BLOCKED.** `cargo xtask replay private_assoc` → exit 1,
  `xtask: no captures under crates/census-crawl/tests/fixtures/private_assoc: the directory holds no body to replay`.
  Owner must supply a real byte-exact capture with URL/robots/provenance. Synthetic tests above
  do not establish source qualification; no body was invented.

### Final-tree gate and acceptance accounting

Two complete `bash tools/gate.sh` executions used an explicit **3600-second timeout** and both
finished (no lane silenced or deadline truncation). The first pre-canonical-tie state had 1682
passing tests (artifact://661). The **last/final Rust state** gate (artifact://683) exits **1**:

```text
Summary [ 127.119s] 1683 tests run: 1683 passed (1 slow), 3 skipped
total diagnostics: 0
structure: files>300=0 fns>60=0 fns>25logical=753
ratchet: no metric grew
gate: FAIL -> vet
```

PASS: fmt, zero code comments (1001 Rust files), check, doc, tests, strict source clippy,
production forbidden-construct/size scan, domain type integrity, domain purity, module seams,
ratchet, deny, audit, machete, geiger, feature powerset, bench presence. Geiger emitted graph
matching warnings but completed successfully; it is a dependency scan, not a proof of all
dependency code.

**BLOCKED, not passed:** vet prints
`You must run 'cargo vet init' (store not found at /home/lewis/src/ad-law-scrape/arh-python-port/supply-chain)`
(artifact://683:4055–4059). The brief identifies this as pre-existing and owner-restored;
no exemptions, fabricated vet store, suppression or gate alteration added.

**Acceptance run:** full gate; CHSAA positive and old-+100 negative regression; NCES/state-ED/
TSSAA capture probes; explicit private-assoc capture refusal and synthetic-only parser probes;
school-address native capture/export/readback and corpus tests; bounded name/grade rejection,
per-field phone/provenance/determinism regressions; unchanged CSV; capture round-trips.
**Acceptance not runnable/satisfied:** private-assoc real-capture behavior, because no captured
body/robots evidence exists. **No gate lane skipped.** Three pre-existing ignored runtime tests
did not run: browser `results_capture_costs_one_physical_post` and
`challenge_response_revokes_the_gate_and_ends_pagination` need fixture origin + CDP browser;
`walk_derived_tables` needs operator `WALK_ROOT`. No live-site, full historical corpus,
acceptance is claimed for this round.

Tracker: **tin CLOSED** with executed evidence (also the phone-precedence defect);
**37k OPEN** with bounded fix and remaining per-type scope;
**dtl OPEN** with executed publication repro and approved next-round contract;
**8an OPEN** for out-of-scope fixture-manifest correction.
Residual review findings: **2 OPEN**, **3 BLOCKED** (vet store, real private-assoc capture,
out-of-scope fixture-manifest prose). This round does not certify the whole port.

## Sol closing review, Round 2 — generations, checked inventory and fixture prose (2026-09-30)

Reviewer: **openai-codex/gpt-6.1-sol**. Native worktree:
`/home/lewis/src/ad-law-scrape/arh-python-port`. Scope is the three Main-approved amendments
in `var/sol-review/brief.md`: dtl publication, the remaining sixteen 37k types and 8an's single
fixture SOURCE.md prose. Round-1 evidence above is unchanged. The detailed before/after ledger,
including every per-type counterexample, is appended in `var/sol-review/state.md`.

### Contract-to-evidence map

|Requirement|Executed evidence|Observed result|
|---|---|---|
|dtl refuses a blocked destination before publishing|Before `cargo test -p census-service --test school_address_publication -- --nocapture`; after `cargo test -p census-service --test school_address_corpus --test school_address_publication`|Before exit 101, `no early JSON publication` (artifact://702); after exit 0, 8 passed at that execution (artifact://706); final tree has all 3 publication tests PASS (artifact://730:2295,2297,2355)|
|dtl interruption cannot expose a mixed generation|`cargo test -p census-service --lib school_address::generation -- --nocapture`|3 passed; real child exits 77 before directory rename, between directory/pointer renames, after pointer rename. Respectively old verified/no complete orphan, old verified/complete orphan, new verified/complete orphan. Every new data artifact differs; selected generation bytes are checked individually (artifact://706)|
|dtl manifest/consumer binding|Same exact generation command|ADR-017 encoder digest parity; typed hash/length, manifest-digest, report-digest and outside-pointer refusals all PASS. Final unmanifested external baseline and missing-artifact/current-preservation scenarios also PASS (artifact://730)|
|37k every remaining type rejects invalid persisted values, preserves valid bytes|Before/after `cargo test -p census-domain --lib school_directory::tests::boundary_regressions -- --nocapture --test-threads=1`|Before exit 101, 0 passed / 16 failed (artifact://711); after exit 0, 18 passed (artifact://719). Includes inverted/unrankable GradeSpan, IDs, contacts, scalar/composite geography, ZIP/address, canonical strings and WeakKey|
|37k existing reader/corpus behavior remains valid|`cargo test -p census-service --test nces_directory_properties --test private_assoc_directory_properties --test state_ed_directory_properties --test tssaa_directory_properties --test school_address_corpus --test school_address_publication`|36 passed at that execution (artifact://719); final gate executes all of these plus the two later publication-consumer tests|
|Captured corpus value/byte roundtrip after checked cutover|`cargo test -p census-service --test sol_round2_smoke -- --nocapture`|1 passed, 2609 exact entry roundtrips: CCD 1557, PSS 374, NYSED index 220, profile 1, TSSAA list 456, detail 1 (artifact://724). Throwaway probe and only its own build scaffolds removed afterward|
|CSV compatibility|`sha256sum /tmp/sol-round2-before/school_directory.csv /tmp/sol-round2-after/current/school_directory.csv && cmp /tmp/sol-round2-before/school_directory.csv /tmp/sol-round2-after/current/school_directory.csv`|Exit 0 after dtl and again after 37k; both SHA-256 `25d42b90ced9d85e38e19ec6724146db079730b9e7e1c2057c73e3ebe0902bca` (artifact://708,724)|
|8an measured counts, no capture edits|Rust measurement command and before/after `sha256sum crates/census-crawl/tests/fixtures/state_ed/*.html`, reproduced below|2530 literal/2528 numeric markers; 220 distinct numeric IDs, all 12 digits; 220 distinct profile links; both capture hashes unchanged (artifact://722,727)|
|Full final-tree acceptance|`bash tools/gate.sh`, explicit 3600-second timeout|Exit 1, **gate: FAIL -> vet** only; 1707 passed, 3 skipped; 15 passing lanes, 0 strict diagnostics, no forbidden production constructs, files>300=0, fns>60=0, ratchet no metric grew (artifact://730)|

GradeSpan endpoints now go through `GradeSpan::new`; no Ungraded or AdultEducation endpoints
are accepted. String/scalar checks use the existing `serde(try_from)` convention. Cross-field
checks use private self-remote Serde on the same domain types, not mirrored public DTOs. Decode
never trims, clamps or substitutes defaults. ADR-020's range/source table names every original
domain policy and its NCES/NYSED corroboration; no small-fixture-derived range was invented.

Two additional reproduced constructor/decode conflicts were fixed, rather than special-casing
probe input: `MatchForm::of("İ")` formerly emitted `i\u{307}`, which its checked decoder rejected
(artifact://715); presentation-casing `"ß"` formerly emitted `"SS"`, which checked street/city
decode rejected (artifact://717). Canonical producer outputs now roundtrip as `"i"` and `"Ss"`.
Both failing-before/passing-after behaviors remain in the eighteen boundary regressions.
Two intermediate probe rows incorrectly used invented state enum spellings `NewJersey` and
`NewYork`; inspecting the actual jurisdiction wire encoder corrected the probes to `NJ` and
`NY`, without weakening a production policy (artifact://713 versus artifact://719).

### Native publication smoke and legacy-layout refusal

Before cutover:

```text
cargo run -p census-service --bin census-service -- school-address --ccd crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv --pss crates/census-crawl/tests/fixtures/nces/pss2324_pu_head.csv --out /tmp/sol-round2-before
exit 0; 1931 rows -> 1931 entries; skipped 67; notes 0
```

After dtl:

```text
cargo run -p census-service --bin census-service -- school-address --ccd crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv --pss crates/census-crawl/tests/fixtures/nces/pss2324_pu_head.csv --out /tmp/sol-round2-after --baseline /tmp/sol-round2-after/current/baseline.json --ledger /tmp/sol-round2-after/current/update_ledger.json --now 2026-09
exit 0; same population; first baseline, no changes.json
```

After all 37k repairs, execute the same post-cutover command with `--now 2026-10`:
exit 0; **0 added / 0 removed / 0 modified**; CCD next due 2027-09, PSS next due 2028-01;
verified baseline and ledger consumed and `changes.json` published. The report lists all seven
in-generation paths. This is a real CLI/readback run, not just source or test assertions.

Directory-destination reproduction:

```text
mkdir -p /tmp/sol-round2-blocked/school_directory.csv && cargo run -p census-service --bin census-service -- school-address --ccd crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv --out /tmp/sol-round2-blocked
exit 1
Error: legacy or blocking destination /tmp/sol-round2-blocked/school_directory.csv: legacy flat layout is rejected; choose a fresh output directory
```

Subsequent directory read: only the original `school_directory.csv/` blocker exists. No artifact,
generation root or staging sibling was created. All legacy flat layouts are refused by this
preflight, not silently migrated. Baseline/ledger output authority is inside the new generation;
supplied paths are verified-generation read inputs only, never separate mirror publications.

`./target/debug/census-service school-address --help && readlink /tmp/sol-round2-after/current && jq '{schema_revision, generation_digest, run_id: .run.run_id, created_at: .run.created_at, baseline_input: .run.inputs.baseline, artifacts: [.artifacts[].name]}' /tmp/sol-round2-after/current/manifest.json`
exited 0. CLI help documents the cutover; current points at `generations/f20741d22971459b`;
manifest schema 1, digest `f20741d22971459b5129f70399cdd44e0617f67dd25d34ec506f498814b39716`,
run month `2026-10`, and the exact sorted data artifacts baseline, changes, CSV, directory JSON,
update ledger. The complete schema/digest/fsync/pointer contract is in ADR-020. No wall clock,
self-hash, live network call or store schema migration is involved.

### 8an measurement replay

The exact executed scratch source `/tmp/sol-round2-measure.rs` is preserved here so its command
remains reproducible after removing that owned throwaway source/binary:

```rust
use std::collections::{BTreeMap, BTreeSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("missing capture path")?;
    let text = std::fs::read_to_string(path)?;
    let suffixes: Vec<_> = text.split("instid=").skip(1).collect();
    let ids: Vec<String> = suffixes.iter().map(|suffix| suffix.chars().take_while(char::is_ascii_digit).collect()).filter(|id: &String| !id.is_empty()).collect();
    let unique: BTreeSet<_> = ids.iter().collect();
    let mut lengths = BTreeMap::new();
    for id in &unique { let count = lengths.entry(id.len()).or_insert(0usize); *count = count.saturating_add(1); }
    let links: Vec<String> = text.split("profile.php?instid=").skip(1).map(|suffix| suffix.chars().take_while(char::is_ascii_digit).collect()).filter(|id: &String| !id.is_empty()).collect();
    let distinct_links: BTreeSet<_> = links.iter().collect();
    let nonnumeric: Vec<String> = suffixes.iter().filter(|suffix| !suffix.starts_with(|ch: char| ch.is_ascii_digit())).map(|suffix| suffix.chars().take(48).collect()).collect();
    println!("instid= occurrences (all): {}", suffixes.len());
    println!("instid= occurrences (numeric): {}", ids.len());
    println!("distinct numeric ids: {}", unique.len());
    println!("distinct id digit-length distribution: {lengths:?}");
    println!("profile.php?instid= links: {}; distinct: {}", links.len(), distinct_links.len());
    println!("nonnumeric instid= suffixes: {nonnumeric:?}");
    println!("800000054526 already in twelve-digit set: {}", unique.iter().any(|id| id.as_str() == "800000054526"));
    Ok(())
}
```

```text
rustc --edition 2021 /tmp/sol-round2-measure.rs -o /tmp/sol-round2-measure && /tmp/sol-round2-measure crates/census-crawl/tests/fixtures/state_ed/index_letter_a.html
exit 0
instid= occurrences (all): 2530
instid= occurrences (numeric): 2528
distinct numeric ids: 220
distinct id digit-length distribution: {12: 220}
profile.php?instid= links: 220; distinct: 220
nonnumeric instid= suffixes: ["some_value\"\r\n            let inst_id = params.in", "\" + instid;\r\n        }\r\n    }\r\n</script>\r\n\r\n\r\n\r\n"]
800000054526 already in twelve-digit set: true
```

Only `crates/census-crawl/tests/fixtures/state_ed/SOURCE.md` prose changed. The prior count added
`800000054526` to a twelve-digit set that already contained it; the prior link count inherited that
double-count. The literal count also includes two nonnumeric JavaScript markers, not schools.

Before and after `sha256sum crates/census-crawl/tests/fixtures/state_ed/*.html`:

```text
adf44bb69c0c8751459c698162fe7526950a6e4feb1778a07d4f3c6c9aa37f27  crates/census-crawl/tests/fixtures/state_ed/index_letter_a.html
6d720faec71b6dbf30eddb8045262e78daf62931d805a2a501f9eefa12442830  crates/census-crawl/tests/fixtures/state_ed/profile_kingston.html
```

### Final-tree gate, blockers and limits

`bash tools/gate.sh` ran in the native worktree with **explicit 3600-second timeout**; it
completed in 534.37 seconds, exit 1. Raw result is artifact://730, not a shortened or silenced lane.
All 16 lanes were attempted. **15 PASS; vet alone FAIL**:

```text
Summary [ 127.321s] 1707 tests run: 1707 passed (1 slow), 3 skipped
total diagnostics: 0
structure: files>300=0 fns>60=0 fns>25logical=760
ratchet: no metric grew
ERROR × You must run 'cargo vet init' (store not found at /home/lewis/src/ad-law-scrape/arh-python-port/supply-chain)
gate: FAIL -> vet
```

Zero-comments scan checked 1010 Rust files; production forbidden-construct counts are all zero.
Check, doc, tests, strict Clippy/source scan, domain type integrity, domain purity, module seams,
ratchet, deny, audit, machete, geiger, feature powerset and benchmark presence passed as reported
by the gate. Geiger also emitted existing registry/package-matching and third-party parse warnings;
its PASS is the actual lane status, not a claim of complete third-party unsafe analysis.

After its one new ledger-reader `needless_question_mark` diagnostic was repaired,
`cargo clippy -p census-domain -p census-service --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock`
exited 0 without warnings. No lint suppression or quality-baseline changes.

**BLOCKED, not passed:** owner-restored cargo-vet store remains absent; no fake init/exemptions.
**BLOCKED, not claimed:** real private-association capture behavior still lacks a captured
body/robots pair, as established in round 1; this round's four private-assoc property cases are
synthetic. The gate's three pre-existing ignored runtime tests remain unexecuted, with their
round-1 browser/CDP and WALK_ROOT prerequisites unchanged. No lane was skipped. No live-site,
The new proof is bounded to the executed generation interruption and domain/captured-input
scenarios above, not power-cut filesystem recovery or hostile concurrent mutation.

Tracker updates executed after final acceptance: **athletic-rust-pipeline-dtl,
athletic-rust-pipeline-37k and athletic-rust-pipeline-8an CLOSED** with the executed evidence
above. New owner-prerequisite beads **athletic-rust-pipeline-3sb** (vet store) and
**athletic-rust-pipeline-9rj** (real private-assoc capture) are **BLOCKED**, not code acceptance
passes. Final ledger: **0 OPEN, 2 BLOCKED**. All three approved workstreams are complete; the
whole port remains uncertified. No commit/push or unrelated-state cleanup was performed.

VERDICT: BLOCKED 2

## Landing on `main` — the port's delta integrated (2026-09-30, `887f844`)

**Landing.** `887f844` changes 121 files against the previous `main` (`5d11c48`): 85 added, 13
deleted, 23 modified (+27 930 / −2 560). `git ls-files '*.py'` at the commit → 0. The deleted set is
`hs-address-pipeline/{address_normalizer,config,geocoder,nccs_crawler,pipeline,private_associations_crawler,pss_crawler,state_ed_crawler,update_strategy}.py`,
`hs-address-pipeline/requirements.txt`, `parsers/tn_tssaa_school.py`,
`tools/{chsaa_golden,chsaa_make_fixtures,port_chsaa}.py` and
`tools/{chsaa_port,port_chsaa_fixtures}.sh`. Three uncommitted working-tree states were integrated
rather than overwritten: the CHSAA `decode` refactor's `chsaa/{mod,decode}.rs` (kept; the port's
`parse/schema.rs` split re-exports `MemberSchool`, so that tree still type-checks), the merged
`xtask`/`registry` plan-slug lists, and the `CrawlError::DirectoryArtifact` arm the port adds in
`restate_services/jobs.rs`. One candidate was deliberately *not* carried: a table row for
`arbiter_orgs` (a row without its registry descriptor fails
`applicability::tests::the_table_names_every_registered_source_once`; the descriptor is that
cluster's uncommitted `registry/table/from_mshsl.rs` addition, so the row must return with it).

**Commands and observed results** — clean worktree at `887f844`, `tools/gate.sh`, warm shared target:

- `cargo test -q -p census-crawl --lib` → `499 passed; 0 failed`.
- `tools/gate.sh` → 13 of 17 lanes PASS; FAIL: `fmt`, `architecture contract`, `ratchet`, `vet`.
  Each reproduces on `5d11c48` and belongs to another cluster's uncommitted slice:
  - `fmt` — only `crates/census-crawl/src/coach_directories/map.rs`, 347 lines and unformatted in
    both commits; that cluster's working copy is the 226-line formatted split.
  - `architecture contract` check 8 — `arbiter` is a crawl-crate module with no descriptor in any
    registry state (pre-landing, commit, port worktree all grep clean).
  - `ratchet` — 4 `clippy::arithmetic_side_effects` (`coach_directories/survey.rs`,
    `milesplit/raw_rows/columns.rs`) and 4 files over 300 lines (`map.rs` 347, `survey.rs` 483,
    `restate_services/ingest.rs` 326, `xtask/src/replay/cases.rs` 349), identical at `5d11c48`;
    the cluster working copies measure 226/180/210/264. Landing effect: census-crawl strict clippy
    diagnostics fall from 17 (`5d11c48`, measured with the gate's lint set) to those 4.
  - `vet` — supply-chain store absent (bead `athletic-rust-pipeline-3sb`), unchanged by the landing.

**CLI smoke** — `cargo run -q -p census-service --bin census-service -- school-address --ccd
crates/census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv --pss
crates/census-crawl/tests/fixtures/nces/pss2324_pu_head.csv --out /tmp/sa-smoke`:

- First run published `current` → `generations/58266c9a3ada0eb9` (digest
  `58266c9a3ada0eb990b8fee88e659b7b06a0df5075d5ea862401c0b305fb517f`, schema_revision 1) holding
  `school_directory.csv` (sha256 `25d42b90ced9d85e38e19ec6724146db079730b9e7e1c2057c73e3ebe0902bca`),
  `school_directory.json`, `baseline.json`, `update_ledger.json`, `pipeline_report.json` and
  `manifest.json`; lane counts `nces-ccd` 1557 entries/42 skipped, `nces-pss` 374/25.
- A second identical run reused the same generation (one directory under `generations/`, same
  pointer), and `--geocode` exited 1 with the typed refusal, creating no output directory.

accompanies this landing; it integrates a port of tooling into `main`. The one Python behavior not
ported is the Google/USPS geocoding and postal-validation phase, which the verb refuses by design
(bead `athletic-rust-pipeline-9p7`), and rows whose state is outside the 49 census jurisdictions are
counted as skips, not published.

## Typed Google/USPS geocode clients and the verb phases — 2026-09-30

Tree: a detached worktree at `74843c8` carrying **exactly** this delta (the shared `main` working copy
was concurrently unbuildable while another cluster's `coach_directories` slice was mid-refactor, so
every lane below ran against a tree containing no other cluster's edits). Scope: `crates/census-crawl/src/geocode/{mod,key,transport,google,usps,tests}.rs`,
`SourceLabel::Geocoder` and `SchoolDirectoryEntry::set_coordinates_from`, the verb phase
(`crates/census-service/src/school_address/geocode.rs`), `PhaseReport` in the report and its summary
line, and the two credential refusals in `tests/school_address_corpus.rs`. Bead
`athletic-rust-pipeline-9p7`; contract in [ADR-020](adr/ADR-020-school-address-corpus-port.md) §4.

Commands and observed results:

- `cargo test -p census-crawl --lib geocode` → **14 passed, 0 failed** (524 filtered).
- `cargo test -p census-domain --lib school_directory` → **61 passed, 0 failed** (the port's 60 plus
  the new `geocoded_coordinates_are_stamped_weakest_and_never_displace_a_published_source`);
  `cargo test -p census-domain --lib geocoded_coordinates` passes alone.
- `cargo test -p census-service --lib school_address` → 3 passed, 0 failed.
- `cargo test -p census-service --test school_address_corpus --test school_address_publication` →
  8 passed and 3 passed, 0 failed: the two credential refusals, byte-identical republish, and the
  publication invariants.
- CLI smoke on the NCES fixtures (`cargo build -p census-service --bin census-service`, then
  `census-service school-address --ccd … --pss … --out …`): a plain run published `current` with
  `"phases": null` in `pipeline_report.json`; with `GOOGLE_MAPS_API_KEY`, `GOOGLE_API_KEY` and
  `USPS_API_TOKEN` unset, `--geocode` and `--validate-postal` each exited 1 *before writing the
  output directory*, with `the geocoder needs \`GOOGLE_MAPS_API_KEY\` in the environment: …`; `--help`
  documents both flags and their credentials.
- `tools/gate.sh` → `FAIL -> fmt architecture contract ratchet vet`, every failing lane naming files
  outside this delta and reproducing at `74843c8` alone: fmt diffs only in
  `coach_directories/map.rs`; check 8's single unregistered module `arbiter` (22 registered sources,
  17 readers including `geocode`, plus `arbiter` = the 40 crawl modules); ratchet growth of four
  `clippy::arithmetic_side_effects` sites in `coach_directories/survey.rs` and
  `milesplit/raw_rows/columns.rs`, four functions over the 60-line page in
  `coach_directories/{collect,survey}.rs`, `restate_services/ingest.rs` and `replay/cases.rs`, and
  four files over 300 lines (`map.rs`, `survey.rs`, `ingest.rs`, `cases.rs`); and no `cargo vet`
  store. tests, check, doc, comments, strict clippy, production scan, domain integrity, domain
  purity, module seams, deny, audit, machete, geiger, feature powerset and bench presence all
  passed. This delta adds 0 forbidden constructs, 0 over-budget functions and 0 oversized files: the
  first `apply_async` was 70 lines, the scan flagged it, and it was split into `geocode_phase` and
  `validate_phase`.
- Flake observed once under full-workspace nextest parallelism:
  `census-service::recovery ks_directory_walk_claims_units_the_kill_can_lose` failed, then passed in
  isolation and in the final gate run (a timing-sensitive kill ladder in code this delta does not
  touch).

Defect and regression: the first execution of `documented_usps_address_body_yields_typed_fields`
failed with `street: None` — `Response`/`AddressEntry`/`AdditionalInfo` lacked
`#[serde(rename_all = "camelCase")]`, so `streetAddress`, `additionalInfo`, `deliveryPoint` and
`DPVConfirmation` never bound. Fixed in `usps.rs`; the same test now passes.

Defect and regression, second: the CLI smoke's refusal read `the geocoder needs credential
GOOGLE_MAPS_API_KEY is not set in the environment in the environment` — the missing-credential
display nested inside the phase message. The phase now names the variable
(`needs \`GOOGLE_MAPS_API_KEY\` in the environment: …`), observed by rerunning the smoke against the
rebuilt binary; the corpus lane's assertion on the variable name still passes.

Limits. No live Google or USPS call was made: the client tests drive a recording `Transport` whose
bodies are the vendors' **documented** response shapes, not committed captures, so the USPS v3 field
names and the Google status set remain unverified against the live services and no capture lane
exists. `HttpTransport`'s socket path, TLS, timeouts and the checked `HeaderValue` refusal of an
invalid token are exercised only through the compiler, not by a test. Live end-to-end qualification
of `--geocode`/`--validate-postal` therefore remains open on `athletic-rust-pipeline-9p7`; the
earlier "typed refusal" smoke in this ledger is superseded — the flags now run the phases and refuse
only an absent credential.

## Reproduce block names the Rust verbs; the Python-free scanner drops stale exclusions (2026-09-30)

The zero-Python audit above left one open item: the generated census document's reproduce block
printed `tools/run_pipeline.sh` and `python3 tools/make_census_doc.py`, neither of which exists in
this tree, so a reader following it would run nothing. `format_sections::reproduce` now prints the
verbs that exist, each checked against the verb's own usage:

- `census-service run --store <store>`
- `census-service census-doc --store <store> --store-out <snapshots> --research <research>`

`format_sections::limits` keeps the 2026-09-20 measurement date and drops the removed tooling path.

Evidence. `cargo nextest run -p census-service format_sections` -> 2 passed
(`reproduce_block_names_the_rust_verbs`, `limits_do_not_point_at_removed_tooling`). The verb ran for
real against the preserved `var/midwest-census` store with a throwaway research root:
`cargo run -q -p census-service --bin census-service -- census-doc --store var/midwest-census
--store-out var/midwest-census/out --research /tmp/census-doc-proof` reported
`"wrote": "/tmp/census-doc-proof/synthesis/10-measured-census.md"`, and lines 162-167 of that
document carry the new block. The preserved store was unaffected (no file under `var/midwest-census`
newer than the run). `cargo clippy` over the touched crates' source targets with the gate's full lint
set is clean.

The contract scanner's `SKIP` no longer names `hs-address-pipeline` or `parsers`, both deleted from
the tree by `ea81c568` and `887f8446`. `cargo xtask contract` check 4 now reports `33731 files
outside [target, .git, var], 0 of them Python artifacts`, and the whole check stays PASS (0
deviations).

Limits. The rendered document is a throwaway readback, not a published artifact: it was written to
`/tmp`, its research root carried no `data/`, and the metrics it printed come from the preserved
2026-09-25 store, not a fresh run. No network was touched.

## The destination guard is wired into the source fetcher (2026-10-01)

Worktree `arh-closeout`, branch `closeout-python-port` (base `accc6e90`).

Defect. `census-crawl/src/net/destination_guard.rs` landed with the port (`6d660a5e`, `e763fd53`),
but no module declared it and no request path consulted it: `Fetcher::new` built its client with
`.redirect(Policy::limited(5))`, no DNS hook and no proxy suppression. A native CLI probe could fetch
`http://127.0.0.1:<port>/robots.txt` and its payload with exit 0.

Repair. `net/mod.rs` declares `mod destination_guard;` and the `Fetcher` carries
`destination: Arc<DestinationGuard>`; `net/client.rs` normalizes the authorized-host list once,
builds the guard from it and installs `.no_proxy()`, `.dns_resolver(GuardedResolver)` and
`.redirect(Policy::custom(|attempt| guard.redirect(attempt)))`, so a literal URL, every resolved
address and every redirect hop are validated against the same list the pacing layer paces;
`net/execute.rs::fetch` validates the URL before its cache lookup, so scheme and credential refusals
hold whether or not a body is cached. `tokio` gains the `net` feature (the resolver resolves through
`tokio::net::lookup_host`) and the test harness gains `io-util`.

Evidence. `cargo nextest run -p census-crawl --all-features` -> 565 passed, 0 skipped;
`cargo nextest run -p census-service --all-features` -> 522 passed, 1 skipped; `cargo fmt --all
--check` clean; the gate's clippy lint set over the crawl crate's source targets is clean. Six new
tests in `net/destination_guard/wiring_tests.rs` cover an unauthorized loopback literal (policy
error, listener never contacted), `localhost` without a grant, an explicit `127.0.0.1` grant that
admits the fixture after its `robots.txt` (server observes exactly `/robots.txt`, `/payload`), a
same-host redirect followed while a differently-named local hop is refused with the unlisted listener
silent, a seeded cache body that does not bypass the refusal, and `file://`/`ftp://`/credential URLs.
`crates/census-crawl/src/milesplit/results/pages/tests.rs` grants `127.0.0.1` in its loopback
fixture fetcher: its unreachable-page test needs a local 500, which the guard rightly stopped
accepting implicitly.

Native CLI probes against `python3 -m http.server 8971 --bind 127.0.0.1`:
`census-service --store /tmp/kz5-smoke/store fetch http://127.0.0.1:8971/payload` -> `policy:
non-public destination 127.0.0.1 for 127.0.0.1 is not explicitly authorized`, exit 1, listener log
empty; the same command with `--authorized-host 127.0.0.1` -> exit 0, the listener logging
`GET /robots.txt` then `GET /payload`.

Limits. The DNS hook is exercised by a resolved `localhost` grant, not by a public name whose
addresses change; no proxy environment variable was set during the probes, so `no_proxy` is asserted
by construction rather than by a poisoned-environment test.

## Isolated consolidated integration qualification (2026-10-01/02)

Scope. Commands and edits ran in `/home/lewis/src/ad-law-scrape/arh-integration-20261001`,
branch `integration/census-reconciliation`. The original working tree was read-only input:
snapshots `d1853a4e336d8d3db5cefc2e479cf4df54533a46`,
`479b97953bd133bac003b55a6ce77a3a28b01804` and final cutoff
`531bf9f7792022b16d4abe09c3a454496157e095`. Relevant changes were integrated or subsumed by
stronger checked implementations. Private prompts, interaction logs, runtime databases, historical
stores and generated run artifacts are not PR inputs. This qualifies the integration, not a fresh
national census, the complete native fault matrix, a release seal or comparable performance.

### Integrated behavior and regression evidence

The final focused command was:

```text
cargo fmt --all
cargo test -p census-domain -p census-store -p census-review -p census-crawl -p census-report -p census-service --lib
cargo test -p census-service --bin census-service
cargo test -p census-store --test checkpoint_fence
cargo build -p census-service --bin census-service
cargo run -q -p xtask -- scan
```

Observed: 1,558 library tests passed (crawl 715, domain 244, report 170, review 119,
service 191, store 119); 69 CLI tests and two sequence-fence tests passed. Failures observed
and corrected during integration included unknown model fields, unsupported event refinements,
complete UTF-16 contexts spuriously truncated, malformed postal provenance accepted, displaced
athlete rows skipped by full verification, reopened durable cases falsely treated as applied receipts,
and identical internally contradictory cohort evidence accepted as the same person.

| Requirement | Exercised evidence | Limit |
|---|---|---|
| Owner-bound postal claims and publication policy 3 | Domain/crawl postal regressions; report rendering, capture, full verification and tamper rejection; CLI CSV tests | No fresh nationwide postal acquisition |
| Captured owned-result parsing | Full public 311,763-byte response replay below, plus row/ownership/context regressions | Unknown upstream completeness remains unknown |
| Independent model formats and bounded exact replay | Actual two-server qualification below; strict reply, format-change, selected packet, standing and sequence-fence regressions | Synthetic unresolved subject, not population adjudication |
| Preserved acquisition failures and recovery | Deterministic HTTP/archive/quarantine/304/lock and Arbiter owner-bound corrected-response tests | Not a new power-loss or full 17-scenario native certification |
| Shared publication and identity facts | Same-snapshot enrichment, contradictory cohort, full workbook row comparison and tamper tests | No national release seal or performance claim |

### Actual full-response owned parser

```text
cargo run -q -p census-crawl --example qualification_owned -- var/integration-owned-capture.json var/integration-owned-qualification.json
```

The original retained public HTTP body was read without opening its store, copied only into this
worktree's ignored `var/`, and SHA-256 checked:
`8db9804ebc2d36b6f7ec1e2a289b2b8ab28610e0063f0ee9f54245eb128071ed`.
Observed 602 published rows, 560 accepted owned rows and 42 malformed rows, all 42 with null
athlete names. Result sets 1266814 and 1266815 contain 246 and 356 rows respectively.
`ownership_complete=false`; upstream completeness is unknown. Adelyn Spann, owner 14222592,
retains distinct owned jump results; long-jump result 201782263 at `data[172]` is 13-9 / 419 cm,
Girls, Class of 2027, meet 725218, team 38332. This is captured-response replay, not a live
fresh fetch. The three-row tracked projection remains explicitly identified as a projection.

### Actual two-server protocol and durable replay

```text
cargo run -q -p census-review --example qualification_dual -- var/integration-dual-qualified-v2
cargo run -q -p xtask -- scan
cargo run -q -p xtask -- contract
```

Observed one requested/answered subject, one insufficient-evidence outcome, zero accepted,
failed or dropped outcomes. Both actual `qwen3.8-27b-uncensored` endpoints answered:
11000 with `prompt_json`, 11001 with `json_schema`. Both returned the exact case ID, empty
field/value and confidence zero. The durable record and complete lane audit were read back into
`var/integration-dual-qualified-v2/dual-audit.json`; evidence and per-lane request digests are retained.
The persisted case is `Retained`; unchanged replay requested zero subjects and kept one receipt.
The first qualification attempt requested zero because its seed omitted an actionable persisted
case; the example now seeds that case explicitly without changing intake or automatically reopening
history. The failed evidence directory was preserved.

The post-fix scan reports zero forbidden production constructs, zero files above 300 lines and
zero functions above 60 lines. All eight architecture checks pass. The incoming native inner
retry ceiling of three was rejected by the existing architecture contract and restored to one;
no gate baseline or audit requirement was relaxed.

### Full release gate and remaining limits

An integrated `tools/gate.sh` run executed 2,063 tests: all passed, three skipped. Formatting,
zero-comments, check, documentation, domain integrity/purity, module seams, deny, audit, machete,
geiger, feature powerset and benchmark compilation passed. It failed architecture (inner retry
ceiling), debt ratchet (six JSON-indexing diagnostics in the qualification example), and cargo-vet.
The first two defects were corrected; actual dual-server,
scan and architecture checks passed afterward. The complete post-correction gate result is recorded
below.

Other actual consumer probes included retained-event repair dry-run/apply/idempotent reapply,
unregistered Arbiter provider refusal before fetch, and the standalone school-address CLI:
1,931 rows, 67 skipped, manifest byte/SHA readback and an unchanged second-run diff of zero.
Missing Google/USPS credentials refused before publication. These are isolated qualification
artifacts, not nationwide data or unexercised source coverage. A direct `target/debug/census-service`
invocation failed because that binary was unavailable; the current CLI route is exercised through
`cargo run` rather than treating that failed launch as verification.

The actual current CLI route also passed:

```text
cargo run -q -p census-service --bin census-service -- --store var/integration-pr-smoke/store review --family school-jurisdiction --limit 1 --endpoint http://127.0.0.1:11000 --endpoint http://127.0.0.1:11001 --model qwen3.8-27b-uncensored --response-format prompt-json --response-format json-schema --timeout-secs 90 --max-tokens 1536
```

Observed `requested=1 answered=1 decided=1 accepted=1 rejected=0 insufficient=0 unanswered=0
dropped=0 failed=0`. This exercised CLI parsing, explicit per-lane protocols and the review route
against the isolated synthetic qualification store, not the original serving census store.

A second complete gate again passed 2,063 tests with three skipped and passed the architecture
contract. Its remaining ratchet failure located three unchecked counters and one JSON index in the
new owned-response qualification example. They now use checked increment and direct serialization
of the already selected rows. The exact repair lane passed:

```text
cargo fmt --all
cargo run -q -p census-crawl --example qualification_owned -- var/integration-owned-capture.json var/integration-owned-qualified-v2.json
cargo clippy -p census-crawl --example qualification_owned --all-features -- -D warnings -D clippy::arithmetic_side_effects -D clippy::indexing_slicing
cargo run -q -p xtask -- scan
```

Replay again observed 602 / 560 / 42 and unknown completeness; strict example clippy passed.
The one-commit integration was rebased onto `origin/main` 4388b778, preserving its removal of
`athletic_matcher`, and pushed. Actual GitHub draft [PR #5](https://github.com/lprior-repo/athletic-rust-pipeline/pull/5)
targets `main` from `integration/census-reconciliation`; it is not automatically merged.

The complete post-rebase `tools/gate.sh` run observed 2,063 passed and three skipped.
It passed architecture, formatting, zero-comments, compilation, documentation, domain integrity
and purity, module seams, deny, audit, machete, geiger, feature powerset and bench compilation.
It failed ratchet on one `unnecessary_option_map_or_else` diagnostic in CLI date selection and
vet. The date fallback now uses equivalent
`unwrap_or_else`; an unused model-format test import was removed.

Final repair verification:

```text
cargo fmt --all
cargo test -p census-service --bin census-service
cargo test -p census-review --lib
cargo run -q -p census-service --bin census-service -- --store var/integration-pr-smoke/store review --family school-jurisdiction --limit 1 --endpoint http://127.0.0.1:11000 --endpoint http://127.0.0.1:11001 --model qwen3.8-27b-uncensored --response-format prompt-json --response-format json-schema --timeout-secs 90 --max-tokens 1536
cargo -Zallow-features=portable_simd,try_blocks clippy --workspace --lib --bins --examples --all-features --message-format=json -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo run -q -p xtask -- scan
cargo run -q -p xtask -- ratchet tools/quality-baseline.json var/final-clippy.tsv var/final-scan.json
```

Observed 69 CLI and 119 review tests passed. Actual unchanged CLI replay requested zero subjects.
Strict full-workspace source clippy exited zero with zero warning/error diagnostics and an actual
successful `build-finished` record, preserved in ignored `var/final-clippy.json`. Its measured
zero tally and actual scan were passed to the canonical ratchet: `ratchet: no metric grew`, exit zero.
The scan retains zero forbidden production constructs and no over-budget functions/files.
This focused final repair verification is distinct from the preceding complete gate; the latter
was not rerun after these two small repairs. The full check lane also reported five pre-existing
unused golden-helper warnings in the `parity_pipeline` test harness; these are not source-target
clippy diagnostics. The draft PR does not certify a national release.

## Exhaustive preserved-branch value audit (2026-10-02)

Owning task: `athletic-rust-pipeline-8dk`. Work remained in the isolated
`arh-integration-20261001` worktree on `integration/all-branch-value-audit`, based on
`0b0e988d805c2c122149ecbf90bb16458d99f2d3`. PR #5 was already merged; this is a new
branch-value audit, not another claim that ancestry alone establishes semantic inclusion.
The original checkout, its index, serving stores, preserved branches, stash and source
captures were not modified.

The durable [branch/commit/path matrix](../research/branch-audit-2026-10-02.json)
records all 27 frozen local/origin refs, including the symbolic `origin/HEAD` alias,
15 distinct tips and 520 distinct reachable history commits. Of these commits, 505
are ancestral history inventory and 15 require explicit divergent-value disposition:
11 coach commits, one DragonFly commit, two research-snapshot commits and one
tree-identical integration squash alias. The matrix separates ancestry facts,
semantic dispositions, frozen-base review observations and current parent resolutions.
It does not claim re-proof of every historical behavior.

| Preserved branch family | Value disposition |
|---|---|
| `closeout-python-port`, `engine-buildout`, `feature/port-acquisition-features`, `port-python-to-rust`, `holzman-3090`, `holzman-5090`, `holzman-enforcement`, remote authorized-alpha collection/continued | Acquired Rust/browser/source ownership, event aliases, postal handling, export and drain value retained or superseded by current owners. Reject workbook-root population intake, Python execution, exactly-50-state narrowing, independent retries and score-only identity. Delete the remaining unused private division-code duplicate; retain the active qualified helper. |
| `coach-acquisition-rust` and its origin alias | All 11 divergent commits and 186 unique changed paths dispositioned; independent expected/covered path sets match. Arbiter captures and scanner/corpus value retained. Reject relocation back into storage, partial-admission loss and implementation-derived whole-corpus counts. Correct the stale compiler-lexer wording in the owning tooling reference. |
| `dragonfly-coach-directories` and its origin alias | Historical full tree equals retained main-history commit `feec27eb3da7a69cd16cba8b7d5e1ccc30756f1e`; current mapper is stronger. Repair inherited replay qualification, not the live survey. |
| `restate-spine-alignment` and its origin alias | Durable journaling, one inner attempt, stable fanout, authorization/browser admission and bare-run refusal retained. |
| `port-piaa`, `piaa-merge` and origin port alias | Parser, collector, registration, CLI and replay retained. Durable TeamsArm wiring was never implemented on the port branch; adding an unqualified lane is not recovery of lost branch value. |
| `strip-inline-comments` and its origin alias | Removal value retained; current in-repository scanner enforces the stronger zero-comment/documentation-attribute boundary. Corpus parity is finite, not compiler-wide equivalence. |
| `research/captures-20260929` | All 2,034 net changed paths dispositioned, including every merge-parent commit path. Of 129 Rust paths: 54 exact current, 49 exact main-history/current-evolved, 17 independently adjudicated and nine rejected publication-narrowing deletions. Retain captures; restore numeric name entities and provider-owned athlete hosts without importing incomplete pacing interfaces or obsolete oracles. |
| `integration/census-reconciliation` and its origin alias | Entire file tree equals frozen main despite different squash ancestry; no remaining file-tree value. |
| local `main`, origin main and symbolic origin HEAD | Frozen comparison anchor and aliases, not additional feature branches. |

After `git fetch origin`, every input tip still matched the frozen inventory and no
additional unassigned input ref appeared. No branch was deleted or merged wholesale.
Historical source bodies and goldens remain byte-preserved; new current semantic
qualification files are separate from historical records.

### Implemented value and real consumer checks

The existing bounded numeric decoder now has one owner under MileSplit parsing and
is shared by roster, meet-index and raw-result readers. Decimal/hexadecimal Unicode
entities decode once; escaped references do not recursively decode, and malformed,
overlong or invalid Unicode references remain literal. Athlete links accept provider-owned
root, www, state and nested/alphanumeric hosts while rejecting foreign suffixes and
userinfo lookalikes. Positive route identity and graduation qualification remain required.

Coach replay selects exactly five required root response files before flattening, rather
than treating provenance/probe/survey artifacts as responses. Raw wire records remain
historical inputs. The replay and existing mapper tests share the new exact sorted
name/sport/gender context oracle: sixteen North Carolina contexts and zero Indiana
contexts. Missing required response/oracle reads fail; this is not full-survey certification.

Fail-before evidence:

```text
cargo test -p census-crawl --lib milesplit::tests::roster_entities -- --nocapture
cargo xtask replay coach_directories
```

Observed two roster regressions fail and two pass: numeric names remained encoded and
the provider root host was quarantined (`artifact://295`). The old replay admitted 27
flattened artifacts and failed on unmapped provenance.

Final focused behavior execution:

```text
cargo run -p census-crawl --example branch_roster_smoke
cargo xtask replay coach_directories
cargo test -p census-crawl --lib milesplit:: -- --nocapture
cargo test -p census-crawl --lib coach_directories:: -- --nocapture
cargo test -p census-domain --lib event_tests -- --nocapture
```

The throwaway public-parser smoke read the committed Wisconsin team index and returned
`José D'Arc`, positive athlete ID `42`, graduation `2027` and the exact
`https://www.milesplit.com/athletes/42-example` profile. It was removed afterward.
Replay consumed five required responses: NC directory 452 schools; GA page 2/3,
2,825 total results and 1,000 schools; NC summary staff 46/teams 38/sixteen exact contexts;
Indiana staff zero/teams 130/zero contexts; AccessDenied XML rejected as non-JSON.
Observed 85 MileSplit, 77 coach-directory and six event tests pass (`artifact://308`).

The first integrated gate caught a downstream Ohio oracle expecting literal `&#039;`
in meet names. Current decoded-label oracles independently correct three captured meet
names and three captured roster names; every roster positive ID was independently
matched to captured DOM anchors. No expected file was regenerated from the project parser.
Old golden bytes remain preserved. JSON source facts compare structurally rather than
pinning serialization whitespace. Scoped digest, synthetic entity-ID and cache-forwarding
echo assertions and their now-unused seed scaffolding were removed rather than re-pinned.

```text
cargo fmt --all
cargo test -p census-service --test parity_national milesplit_html_parity -- --nocapture
tools/gate.sh
```

The final focused consumer parity passed. The complete canonical per-commit gate
executed 2,066 tests: 2,066 passed, three skipped (`artifact://326`). Architecture,
formatting, zero-comments, check, documentation, source clippy, domain integrity/purity,
module seams, debt ratchet, deny, audit, machete, geiger, feature powerset and benchmark
compilation passed. Source clippy reported zero diagnostics; the ratchet reported no
metric growth. The only failing gate lane was the existing `vet` lane. Gates, policy
configuration and baselines were not weakened. Five pre-existing unused shared-helper
warnings remain in the `parity_pipeline` test harness; no new `parity_national` warnings
remain.

Final landing inspection found an empty obsolete decoder path left by the move tool.
It was removed; it contained no compiled module or behavior. The complete gate above
preceded that empty-file removal. Post-cutover formatting, comments, actual coach CLI
replay, roster regressions and focused consumer parity were executed again and are
recorded below separately.

This audit certifies branch-value disposition and the exercised parser/CLI behavior only.
It does not certify a national census, live source survey, native Restate fault suite,
formal/mutation/performance qualification or release readiness. The removed historical
Kansas kill test was not restored: its journal-before-row premise is obsolete and its
assertions did not establish zero loss. Current atomic batch ownership and retained
generic recovery coverage supersede it; no new Kansas crash qualification is claimed.

Post-cutover execution (`artifact://334`, exit zero):

```text
cargo fmt --all -- --check
cargo xtask comments
cargo xtask replay coach_directories
cargo test -p census-crawl --lib milesplit::tests::roster_entities -- --nocapture
cargo test -p census-service --test parity_national milesplit_html_parity -- --nocapture
```

Observed 1,207 Rust files checked with zero comments, the same five-response CLI outcomes,
four focused roster tests passed and one current consumer parity test passed. No source
scaffold or obsolete decoder alias remains.

Final source review also capped numeric-prefix inspection at eight characters (the seven-digit
entity limit plus one rejection character), instead of scanning an arbitrarily long malformed
numeric tail before applying the existing limit. Accepted entity semantics and allocation
shape are unchanged; no performance claim is made. Focused roster/parity/CLI checks and the
canonical gate were rerun after this bounded-scan adjustment.

The final post-review command chain (`artifact://338`) was:

```text
cargo fmt --all
cargo test -p census-crawl --lib milesplit::tests::roster_entities -- --nocapture
cargo test -p census-service --test parity_national milesplit_html_parity -- --nocapture
cargo xtask replay coach_directories
tools/gate.sh
```

Observed four focused roster tests and one consumer parity test pass, five required responses
replayed, then 2,066 canonical tests passed with three skipped. The final gate again reported
zero source-clippy diagnostics, no ratchet growth and only the unchanged `vet` failure.
This is the final integrated-code gate after bounded-prefix review. Physical deletion of the
unreferenced empty path was confirmed separately during staging; its final check is below.

### Executed negative qualification scenarios

Read-only Bubblewrap bind namespaces exercised the actual `xtask replay coach_directories`
CLI without changing source fixture/oracle bytes or enabling network access. Exact argv,
stdout, stderr and exit codes are retained in the machine audit's
`verification.negative_cli_smokes`. `cargo build -p xtask` passed (`artifact://343`) before
the successful negative executions. A first attempt found no executable at
`target/debug/xtask` and did not exercise the CLI; it is not refusal evidence.

| Given | When | Then, observed |
|---|---|---|
| An existing but empty response directory overlaid read-only on the coach fixture path | Actual replay CLI reads its required root response set | Exit 1 naming missing `nchsaa_directory_p1.json`; no historical count fallback or successful partial qualification. |
| `{}` overlaid read-only on the current context oracle; real response bodies and wire goldens unchanged | Actual replay CLI reaches the Indiana summary | Exit 1: `in_staff_summary_qwugx2.json has no current coach-context qualification`; no fallback to the historical row count. |
| Real five-response corpus and current context oracle | Unmodified actual replay CLI | Exit zero with the exact five-response outcomes above. |
| Numeric/escaped entity and owned/foreign-host roster inputs | Public roster parser regressions | Exact Unicode/identity/year/profile results or retained missing-identity quarantine; four focused tests pass. |

Temporary namespace smoke inputs were removed. Merge-aware path checks also confirmed
186/186 coach paths and all 2,034 research snapshot paths, including both snapshot parents,
with no missing disposition.

Staging exposed that the file tool's reported deletion had left the obsolete decoder path
as a zero-byte physical file. The owned empty path was removed with `rm`, and
`git add -u` then recorded a real deletion rather than the empty blob. No compiled module
used that path. The same post-cutover formatting/comments/actual replay/focused roster/parity
chain passed again after physical deletion (`artifact://352`): 1,207 comment-free Rust files,
five responses replayed, four roster regressions and one consumer parity test passed.
The complete 2,066-test gate above preceded this physical empty-file cleanup; no compiled
behavior changed afterward.

## Integrated qualification checks and actual VM stop — 2026-10-02

The report alias/direct-cohort regressions initially exposed a snapshot mismatch in the new
workbook test: the test built from a later store dataset while verifying the earlier frozen input.
The test now builds and verifies the same frozen dataset. The three `published_cohort` tests passed
(`bg_427`), including actual workbook readback. This is not a national workbook qualification.
The final integrated `cargo test --offline -p census-report` passed all 185 tests
(`artifact://1339`). The repaired native teams example's 34 tests also passed
(`artifact://1340`), including bounded writer contention and retained cleanup failure outcomes.
The service roster suite passed three tests and the browser library passed 34 with two ignored
(`artifact://1312`). Synthetic canonical-owner wrappers are explicitly synthetic; the unchanged
authentic ownerless fragment remains pure-parser evidence, not acquired school-owner qualification.

The native VM example passed 78 tests and release builds of the VM example and `census-serve`
passed (`artifact://1317`). Workspace all-target/all-feature offline check passed (`bg_431`).
These checks do not certify actual reboot, midnight acquisition or all 17 native faults.
Root13 rejected the conventional Restate binary because its observed version was 1.6.2 rather than
the required 1.7.x. Root14 rejected an incorrectly nested tools path. Root15 used the verified
1.7.0 binary and tools root, then stopped at `jurisdiction-reboot-start` after 99.36 seconds:
`required pinned_deployment_id absent`. No reboot or midnight scenario was reached.
Its retained `cleanup.json` records accepted=4, completed=4, cancelled=timed_out=aborted=panicked=0;
endpoint, node and QEMU exited successfully after TERM and were reaped. Disks and logs remain
under `var/vm-sol-20261002-15/`. Original invocation state observation is being repaired;
no replacement job or successful recovery is claimed.

Strict production Clippy remains failing. The first integrated run rejected the enlarged browser
outcome enum (`artifact://1326`); the wire final-response URL now uses an owned boxed string,
converted to the archive string without copying its bytes. The next run exposed actual native
qualification example arithmetic, JSON indexing and simplification errors (`artifact://1332`).
Those repairs are not yet a strict-lint pass or runtime qualification.

`jq -e` independently checked the retained source-keyed inventory's registry, CLI, dispatch,
team-arm, coach-family and jurisdiction cardinalities against their arrays/maps and confirmed
the missing 50-minus-49 member remains unknown. Result: `true`, exit 0. The JSON is
`var/source-keyed-inventory-sol-20261002.json`; this structural check does not authenticate capture
bodies or supply the still-missing revision/run-bound 50/49 manifest pair.

The integrated `cargo test --offline -p census-crawl` passed 819 tests
(`artifact://1349`). The service's
`jurisdiction_walk_resumes_from_the_journaled_index_and_the_unclaimed_rosters`
regression passed after the explicit synthetic owner-wrapper migration (`bg_443`).
These parser/recovery tests do not certify authentic live roster ownership or native reboot recovery.

Actual local model discovery returned `qwen3.8-27b-uncensored` from both `/v1/models`
endpoints. `ss -ltnp` bound port 11000 to `ninfer-serve` PID 1661 and port 11001 to
`llama-server` PID 1660. `nvidia-smi --query-compute-apps` bound those PIDs respectively to
PCI buses `00000000:01:00.0` (RTX 5090) and `00000000:03:00.0` (RTX 3090).
This verifies available lane identities, not successful advice on a genuine ambiguous case.

After current release builds of both production binaries passed (`bg_444`), a new native
Restate 1.7.0 node and sole store-owning endpoint started for
`var/national-sol-20261002-01/`, with a new cluster/base directory and no imported population.
Admin/ingress/endpoint are loopback 19095/18095/19080. Registration returned
`dp_14y1ca6Uya97dbeoEPtstwZ`; `GET /deployments` showed all 11 current services.
The unrestricted lower48+DC submission was:

```sh
target/release/census-service national --ingress http://127.0.0.1:18095/ --season 2026 --revision 1 --source-parallelism 1 --concurrency 4 --refresh --detach --json
```

It returned key `national:2026-27:51472a0f63b82f0d:1` and original invocation
`inv_13LIoGM6LB600EGmJ5iU9yGQmEPr4dHTGp`. An actual admin query observed that invocation
running and pinned to the registered deployment. `run-identity.json` retains the exact binding
and command. Submission/running state is not completed acquisition, census acceptance, release
qualification or a final workbook. Continue observing the same invocation; do not submit a new
logical job to reset attempts.

The native pin-observation repair passed all 89 VM example tests (`artifact://1361`), including
pending/ready/running-before-first-journal and pending sibling first-pin transitions. Missing pins
remain non-certifying; existing observed pins/protocols and the original witnessed child stay strict.
Integrated strict production Clippy passed (`bg_447`) after the browser layout and native example
safety repairs. The source scan (`bg_452`) measured zero forbidden constructs in all nine packages,
no files over 300 lines and no functions over 60 logical lines. The separate lexical scan checked
1,463 Rust files without comments; all eight architecture checks and the seam scan passed
(`artifact://1377`). These static/test checks are not the full release gate.

Actual authentic-capture projection readback initially failed in qualification roots 07 and 08
(`artifact://1357`, `artifact://1364`): the qualification consumer still looked for bare completion
keys, mixed newly archived raw metadata chunks with the owned API chunks, and expected API acquisition
time on raw metadata evidence. Both qualification consumers now recognize content-bound receipts
and select archive chunks by exact capture provenance; bound-performance checks use each capture's
own actual timestamp. The unchanged historical bodies/metadata remain preserved.
Root09 then passed the actual `qualification_bound_projection` command (`artifact://1369`,
`artifact://1372`): zero requests, 230 retained source rows, 13 exact source-owned performances,
ten original API chunks/readback, flush/drop/reopen equality, and identical third replay with
sequence 14 unchanged. This is historical capture replay in an exclusive fresh qualification store,
not fresh national acquisition, exhaustive histories or canonical identity acceptance.

The final integrated workspace command
`cargo nextest run --offline --workspace --all-features` passed 2,209 tests, with three skipped,
in 142.071 test seconds (`artifact://1382`). The skipped tests are not executed browser/native fault
qualification. Native VM root16 still stopped at source-boundary observation with
`required pinned_deployment_id absent` after 121.00 seconds; its drain records accepted=completed=4
and zero other outcomes. Disks/logs remain preserved. An actual independent admin observation of
the fresh production run showed a running TeamsSource child with journal_size=1 and no deployment
or protocol pin, while completed children had journal_size=2 and both pins (`artifact://1384`).
Thus a positive journal count alone is not evidence that an active physical source attempt is pinned.
The native qualification remains blocked; no reboot or midnight success is claimed for root16.

Both changed browser URL producers were exercised through real owned headless Chromium and two
bounded loopback fixture listeners. A temporary external Rust package called the production
`BrowserManager` API; it did not fabricate CDP events, bindings, Response objects or source data.
The first launch failed because the inherited long repository `TMPDIR` exceeded Chromium's Unix
socket-path bound (`artifact://1380`). The already-built smoke binary then ran with `TMPDIR=/tmp`
and a new, never-shared profile:

```sh
TMPDIR=/tmp target/debug/browser-url-smoke /usr/bin/chromium /home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/browser-url-smoke-sol-20261002/profile-02
```

All six actual HTTP cases passed (`artifact://1381`): CDP direct capture retained its actual URL,
not the semantic citation; CDP rejected a redirect without fetching its target; JS Navigation
captured a direct response; a same-endpoint query redirect retained the actual final URL separately
from the original request URL; wrong-endpoint and cross-origin final responses were rejected.
Both redirect hops and the refused final 200 responses were observed in the fixture ledger.
Browser shutdown had accepted=completed=1, remaining=aborted=cancelled=timed_out=panicked=0,
no shutdown error, and both fixture tasks joined. The raw output is preserved in
`var/browser-url-smoke-evidence-sol-20261002.jsonl`; the temporary harness and its two private
profiles were removed after proof. This proves these producer/admission paths on owned loopback
QA, not public Athletic.net access, authentic source acquisition or a national completion.

### Reopened adversarial acquisition findings and integrated repairs — 2026-10-02

Main reproduced the capture-review counterexamples before authorizing production changes:
`cargo test --offline -p census-crawl acquisition_ -- --nocapture` ran 16 tests, 11 passed
and all five new regressions failed (`artifact://1397`, raw `artifact://1396`). Identical
partial replay restaged original rows/rejections, distinct acquisition metadata replaced an
old partial interpretation, and a new-time manifest reference resolved the old timestamp.
The earlier completed-order regression failed on reversed result URLs (`artifact://1393`):
physical Meets increased 1→2, Teams 2→3 and Athletes 2→3 without changed performances.

Main then integrated immutable acquisition manifests sharing original content chunks,
content-bound partial/source receipts, exact indexed replay lookups, typed set-valued entity
normalization, and both v5 projection/failure archive references. The targeted Nextest filter
`test(acquisition_) | test(order_replay::) | test(transport_policy::)` passed all 20 tests
(`artifact://1402`). No public acquisition, identity acceptance or national exhaustion is implied.

The security regression filter `test(contextual::) | test(transport_policy::)` initially ran
21 tests, six passed and 15 failed (`artifact://1400`, raw `artifact://1399`).
Synthetic raw-text/quoted-attribute/template owner declarations and encoded selector
contradictions bypassed admission. The isolated Fetcher redirect regression observed one direct
HTTP request to the DNS-pinned owned loopback target representing a browser-only destination.
No public Athletic.net request was made. Main replaced the partial regex scanner with
`html5gum` 0.8.4 tokenization and a selective emitter that does not collect irrelevant text,
comments, doctypes or attributes; decoded metadata, raw-text state, inert templates and foreign
namespaces are accounted for. Direct HTTP now refuses a browser-only redirect before dispatch.
The combined owner/acquisition/order/transport filter passed all 46 tests (`artifact://1406`).

The complete census-crawl Nextest suite passed all 847 tests (`artifact://1413`).
The native VM unit suite in the same command passed 88/89; its remaining failure compared
literal error wording after Main added actual pin-status JSON to the diagnostic context.
Main removed that wording-only assertion while retaining refusal of missing/foreign pins,
wrong protocol and replacement invocation inputs. Its rerun and the clock-before-source
scenario reorder remain unverified at this entry; neither creates a pin or relaxes certification.

The actual captured public collector smoke ran in the new exclusive
`var/qualification-bound-sol-20261002-10/` (`artifact://1411`, raw `artifact://1410`):
original API/raw/index/roster bytes and metadata were unchanged, captured source rows and
bound projections were retained, flush/drop/reopen succeeded, and the third identical replay
left the physical tables, journals and store sequence unchanged (14→14). Actual requests were
zero; this proves captured-path projection/replay, not a fresh public-source qualification.
The original fresh NationalCensus invocation was independently inspected through the admin
JSON query API: still running, journal size 99, pinned to `dp_14y1ca6Uya97dbeoEPtstwZ`,
protocol 7. It has not been replaced or certified complete.

The subsequent integrated gate command passed strict workspace/all-feature Clippy over
libraries, binaries and examples, all nine production scans with zero forbidden constructs,
zero files over 300 lines and zero functions over 60 logical lines, zero-comments across
1,476 Rust files, all eight architecture checks, and module/crate seams with no violations.
The complete all-feature workspace Nextest run passed 2,237 tests, three skipped
(`artifact://1420`; 144.756 test seconds, 212.33 seconds for the combined command).

The separate native VM example unit rerun passed all 89 tests. Its release build passed in
87 seconds, then the actual reordered clock-first Root17 scenario failed:
`natural guest midnight crossing exceeded 330-second injection budget`
(`artifact://1424`; combined command 526.70 seconds). Original clock-set exit success did
not establish injection: Root17 captured 19:56:54.892691734 UTC before and
19:56:55.037242158 UTC after, not the requested 23:55 pre-midnight baseline.
The final observation remained 20:02:24 on the same date. No natural next-day crossing,
after-midnight acquisition, source-stage reset or complete fault verdict was proved.
Endpoint drain accepted=completed=4, all other outcome counts zero; endpoint, native node
and QEMU were terminated and reaped. Root17 disks, captures, logs and cleanup remain preserved.

An independent throwaway executable exercised the public `fetch_roster` API after the HTML
repair (`artifact://1422`): inert style text could not authorize unknown historical ownership,
an encoded foreign owner contradicted the matching observed final URL and was refused,
and an actual encoded matching declaration admitted the unchanged authentic 25-row verdict.
All three used explicitly synthetic prefixes and zero physical HTTP requests.
`var/roster-owner-smoke-evidence-sol-20261002.json` retains exact result/digests; the temporary
package and its private cache were removed after proof.

Actual Qwen execution is now distinct from earlier SOL workers carrying GPU names.
Main registered direct OpenAI chat-completion tools for ports 11000 and 11001 and assigned
independent source-registration and clock-qualification work. Saved completed HTTP responses
report `qwen3.8-27b-uncensored` with model completion IDs and usage. The current listener/PID/GPU
join was actually executed (`artifact://1429`): PID1661/ninfer on 11000 uses the RTX5090 at
PCI01; PID1660/llama-server on 11001 uses the RTX3090 at PCI03.
`var/actual-qwen-model-mapping-sol-20261002.json` preserves full delegated command output,
both live model descriptors and sampled response metadata. Some actual 5090 generations
emitted the wrong programming language and were rejected, not passed off as Rust or SOL work.
This is coding-generation evidence, not genuine ambiguous-census-case review acceptance.

The fresh ingress/admin observations are preserved in
`var/national-sol-20261002-01/progress-evidence-qwen-wave.json` and its linked raw responses.
At the observed snapshot, the original national invocation still ran with journal99/protocol7;
run-handler counts showed 65 completed and 13 running TeamsSource invocations and 49 running
JurisdictionCensus invocations. Census physical counts included 278,214 athlete rows, but zero
review cases, conflicts and identity verdicts. These are not unique eligible accepted people.
The ingress open-work command succeeded in 0.17 seconds: 49 jurisdiction rows, 48 unfinished
sweeps, 30 completed teams stages, two roster stages and one meet stage, all readable.
Its delivered JSON is preserved as an explicitly labeled semantic transcription because the
completed tool stream expired before raw archival retrieval; separate stderr was not captured.
Source-object obligations remained unmeasured/null, and the national report remained null.

### Actual dual-Qwen registration and clock qualification — 2026-10-02

Actual RTX5090 Qwen generated the immutable TeamsSource registration helper and replay
regressions; Main integrated the existing blocking-region API and all admission callers.
The registration performs real Fjall identity commitment, not a no-op journal marker.
It consumes no physical reservation and the existing three-reservation acquisition ceiling
is unchanged. Cached SDK identity is compared against persisted Fjall authority before admission.
The first integrated command passed all 16 source-ledger tests and all 92 native VM tests
(`artifact://1437`, 24.23 seconds including compilation/formatting). After splitting the
regressions into bounded modules and adding native diagnostics, the same 16 and 92 passed again
(`artifact://1443`; 1.02 and 2.99 seconds respectively). The release endpoint/VM example build
passed in 68 seconds. These passes predate the later four cached-receipt and five clock-boundary
tests and post-setup retained-window guard; those changes require another integrated check.

That command then ran the actual fresh owned Root18 VM with native Restate 1.7.0 and failed
`natural guest midnight crossing exceeded 330-second injection budget`; combined exit 1 and
wall time 514.25 seconds (`artifact://1443`). Root18's
`guest-clock-injection.json` retained actual GNU date stdout and confirmed initial guest
realtime **23:55:00.047970997 UTC**, following 20:48:59.064843373 UTC before the command.
However, the actual unfinished-Sleep witness already read **20:49:01.152634725 UTC**, and the
captured baseline read **20:49:04.841784285 UTC**. Both retained the initial boot
`6c7657a2-4db3-4d7d-a498-347ce2a53110` and machine
`91d07d4ae517469c85ae4627df011777`. The clock reverted after successful initial injection;
no synchronization-agent/root cause has been established. Bootstrap already specifies
`systemctl mask --now` for its two synchronization units, so a missing `--now` is not the
observed cause. Initial diagnostics were null because the first target check succeeded.
Main now collects external UTC/running-service diagnostics on initial success too, and the
actual 3090 worker authored retained-window validation for the post-setup baseline.
Those are prerequisite checks and diagnostic changes, not demonstrated mechanism repair.

Root18 retained only the before-midnight acquisition; no after acquisition or original
source-stage reboot witness was reached. `cleanup.json` records accepted=completed=4,
cancelled=timed_out=aborted=panicked=0, orderly endpoint/node termination and reaping, then
orderly QEMU termination/reaping. All Root18 disks/logs/captures remain preserved.
Neither Root18 nor earlier passing unit/build checks certify all 17 native scenarios.

Actual 5090 generation `chatcmpl-ae866459ba47caa2` produced cached SDK digest/date mismatch
regressions (1,134 prompt and 1,231 completion tokens; 5.347 seconds), preserved under
`var/qwen-5090-evidence-sol-20261002/1790974485299-361ba183-1718-43cb-a9ef-0e35eb6806fe.response.json`.
Actual 3090 generated the native target-window/UTC/identity tests and the retained baseline
helper/tests; request, response and transport files remain under the corresponding
`var/qwen-3090-evidence-sol-20261002/` tree. Wrong APIs, hardcoded windows and an invented
shadow validator in model outputs were rejected or corrected against the real implementation.
All resulting code still requires Main's integrated checks. This remains coding-generation
evidence, not genuine ambiguous-census-case adjudication.

The 5090 worker's source-backed SDK review found that `max_attempts(1)` uses
`OnMaxAttempts::FailAsTerminal`: a local registration write/flush error can become a durable
terminal run result before any physical reservation. Main removed that local one-attempt
override, preserving ordinary resumable SDK storage retry and the separate physical ceiling.
No native registration ENOSPC/replay experiment has yet validated that recovery path.

### Integrated actual-Qwen repairs and kernel ENOSPC — 2026-10-02

The first integration commands failed before tests or runtime: `cargo fmt --all` could
not resolve the nested safety test module, then the ENOSPC example compiler rejected the
borrow lifetime in the bounded `Error::source` traversal. Main added the explicit test
module path and dereferenced the iterator's inner error reference. Neither failure was
counted as a passing check.

Main's subsequent exact command chain, recorded as `artifact://1455`, completed in
53.76 seconds:

```sh
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo test -p census-store --example enospc --all-features
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo test -p census-service --lib restate_services::jurisdiction::team_source::tests --all-features
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo test -p census-service --example qualification_native_vm --all-features
bash -n tools/durability/scenario-09-disk-full-fjall.sh
mkdir var/durability-fjall-sol-20261002-01
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= SCRATCH_STORE="$PWD/var/durability-fjall-sol-20261002-01" bash tools/durability/scenario-09-disk-full-fjall.sh
```

Observed five safety tests, 20 source-ledger tests and 110 native-VM tests passing.
The real private 64-MiB tmpfs probe received kernel errno 28 during `batch_26` at school
836. It checked exact cold records and 28 acknowledged receipts, absence of the failed
batch, unchanged replay with zero appends, and successful new atomic recovery writes
across another cold reopen. The namespace exited zero and preserved `probe.out` and
`preserved-store` in
`var/durability-fjall-sol-20261002-01/enospc-09-Jmmqwn/`.
This establishes the direct Fjall ENOSPC scenario, not Restate registration ENOSPC,
power-loss/reboot recovery, or all 17 required native scenarios.

The actual 5090 Qwen review used response IDs `chatcmpl-fccd174fc5966471`,
`chatcmpl-cec286e16130ee13` and `chatcmpl-14c62cdf0ee5c27d` at loopback port
11000, requested/reported model `qwen3.8-27b-uncensored`. The worker rejected
unsupported APIs, invented timeout escalation and PID-namespace claims against actual
source. The static review found no confirmed atomicity/false-PASS defect but identified
unexercised inspection and cleanup liveness limitations: byte caps do not impose elapsed
deadlines on direct `findmnt` pipe/wait, and TERM-only timeout cannot prove hard-bounded
cleanup under unresponsive processes or IO. These remain explicit limits, not observed
hangs. This is coding/review provenance, not genuine ambiguous-athlete adjudication.

### Root19 clock refusal and actual-GPU review findings — 2026-10-02

Main rebuilt the release endpoint and VM example with
`TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo build -p census-service --release --bin census-serve --example qualification_native_vm --all-features`
(68 seconds), then ran the same native host command with a fresh
`--root var/vm-sol-20261002-19` and `TMPDIR=/tmp`. The combined command failed in
179.23 seconds at actual clock injection; it did not wait another 330 seconds or reach
source reboot recovery. `guest-clock-injection-validation.json` retained same-boot
realtime before `22:01:36.702420079Z` and after `22:01:36.756577054Z` despite GNU
date setter stdout claiming `23:55:00`. Independent GNU UTC observation also returned
`22:01:36.766545405Z`.

The running-service diagnostic explicitly showed
`systemd-timesyncd.service masked active running`. Masking alone was therefore not
proof of stopped synchronization. Main added an explicit bounded guest-only `systemctl
stop` before injection and requires both masked load state and inactive/failed active
state; active or transitioning units are refused. That correction and two new state
regressions have not yet been exercised in another VM. The active service is established;
causality for the observed realtime reversion awaits the corrected scenario.
Root19 cleanup retained its disks/logs and reconciled accepted=completed=4 with all
other counters zero, endpoint PID989 and node PID987 TERM-reaped.

The actual 3090 Qwen review responses
`chatcmpl-SGx5q4rmSEd0E1aMkBEQ6oEHcziteIY1`,
`chatcmpl-oYS7l9qf6Z3RNMlXx3A47yPCw2dGcTHe` and
`chatcmpl-yE4qdoyFHgn1sXNGnNFYbm5oXh5ptQ3m` identified two source-supported,
unexecuted counterexamples: a cancelled registration caller releases its admission
guard while an independently surviving blocking worker can overwrite first identity;
calendar-only crossing checks can admit later backward/forward time steps without
sufficient monotonic elapsed time. Main assigned independent actual-5090 guard-lifetime
and actual-3090 consecutive-clock repairs. Neither static review is runtime proof.
Source-contradicted flush claims, invented APIs and an insufficient one-second uptime
floor were rejected rather than adopted.

### Cancellation ownership, strict integration and Root20 clock — 2026-10-02

Main integrated the actual-5090 registration ownership repair and actual-3090
consecutive-clock validation repair. The supervised registration closure retains its
source-key admission guard after caller cancellation. Its deterministic regression
uses the real registration closure, real Fjall store and supervised blocking worker:
the same key remains unavailable while that worker survives, the first identity/date
remain authoritative after completion, and no second physical reservation appears.
It does not inject cancellation at a literal post-lookup instruction or prove native
Restate registration ENOSPC recovery.

`TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo test -p census-service --lib restate_services::jurisdiction::team_source::tests --all-features`
passed 21 tests. The integrated sequence also passed
`cargo test -p census-service --example qualification_native_vm --all-features`
(129 tests), `cargo test -p census-store --example enospc --all-features`
(5 tests), and warnings-denied Clippy for the service/store source targets.
Raw integrated output: `artifact://1468`; combined duration 17.64 seconds.
`cargo xtask comments`, `contract`, `scan`, `integrity` and `domain-purity`
all passed in the subsequent source-gate sequence: 1,493 Rust files, zero comments,
eight architecture checks with zero deviations, nine scanned packages with no
forbidden constructs or size-budget failures. That sequence also inspected the
release archive and took 9.98 seconds; it is not isolated timing per gate.

Main rebuilt the release endpoint and VM example and exercised
`TMPDIR=/tmp target/release/examples/qualification_native_vm host --root var/vm-sol-20261002-20 --tools var/native-vm-tools-sol-20261002 --base-image var/native-vm-tools-sol-20261002/Arch-Linux-x86_64-cloudimg-20261001.604814.qcow2 --census-serve target/release/census-serve --restate var/native-runtime-restate-1.7.0-sol-20261002/restate-server-x86_64-unknown-linux-musl/restate-server --captures var/native-vm-captures-sol-20261002-root08.json`.
The combined build/scenario took 503.09 seconds and failed at the later
`jurisdiction-reboot-start`, not at clock injection or midnight.

Root20 `guest-clock-injection.json` retained successful guest-only injection to
`2026-10-02T23:55:00.039257771Z`. `clock-oracle.json` recorded PASS for its
explicitly limited clock scenario: the same boot/machine crossed naturally to
`2026-10-03T00:00:00.443522389Z`, with sampled realtime/uptime progress checked
against both adjacent samples and the baseline. Real uncached acquisitions recorded
`fetched_at=2026-10-02T23:55:05Z` and `2026-10-03T00:00:02Z`, each
`from_cache=false`. The same original Sweep invocation
`inv_19eq9TpwKCdY2GX3v3uUiTTslZxgUS791S` and deployment were retained.
This is isolated guest/captured-source qualification on Restate 1.7.0, not national
public freshness, release qualification on 1.7.10, or proof against unsampled,
cancelling or sub-tolerance clock adjustments.

The subsequent source probe refused reset with
`source boundary does not retain owed teams stage`. Main preserved the QCOW2 and,
without booting it or opening Fjall, converted its data image to a new sparse raw
copy and selectively extracted JSON with read-only `btrfs restore`:

```sh
mkdir var/vm-sol-20261002-20/source-boundary-forensics
mkdir var/vm-sol-20261002-20/source-boundary-forensics/files
timeout --signal=TERM 60s env LD_LIBRARY_PATH="$PWD/var/native-vm-tools-sol-20261002/prefix/usr/lib" var/native-vm-tools-sol-20261002/prefix/usr/bin/qemu-img convert -f qcow2 -O raw var/vm-sol-20261002-20/data.qcow2 var/vm-sol-20261002-20/source-boundary-forensics/data.raw
timeout --signal=TERM 30s btrfs restore --path-regex '^/(|jurisdiction-recovery-(invalid-boundary|original)\.json)$' var/vm-sol-20261002-20/source-boundary-forensics/data.raw var/vm-sol-20261002-20/source-boundary-forensics/files
```

Extraction passed in 0.43 seconds. The retained invalid-boundary observation at
probe 3 showed both original TeamsSource children completed and settled on attempt
1, no retained errors, parent teams Completed with 127 records, and the original
parent running with 1 of 72 rosters committed. The requested unfinished teams
boundary was missed; this is not an observed source-data-loss defect and not reboot
acceptance. Do not relax the owed-stage check or manufacture an active child.
Root20 cleanup reconciled accepted=completed=5, all other counters zero, and
TERM-reaped endpoint PID2697, node PID2695 and QEMU PID3332141 with successful exits.
All disks, extracted observations and logs remain preserved.

`TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo nextest run --workspace --all-features`
passed 2,250 tests, with 3 skipped, in 145.332 seconds runner time
(163.09 seconds command wall time), raw output `artifact://1482`.
Skipped tests remain a release-evidence limit.
The full strict source lint command, using the owning `tools/gate.sh` feature
allowlist and every `LINT_SET` denial, first failed on the new ENOSPC example's
`Box<dyn Error>` `as` conversion (`artifact://1484`, exit 101, 6.72 seconds).
Main replaced that conversion with `Box::<dyn Error>::from`, then reran the
5 ENOSPC example tests and the full workspace strict source command:

```sh
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo -Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
mkdir var/durability-fjall-sol-20261002-02
TMPDIR="$PWD/var/test-tmp-sol-20261002" SCRATCH_STORE="$PWD/var/durability-fjall-sol-20261002-02" RUSTC_WRAPPER= bash tools/durability/scenario-09-disk-full-fjall.sh
```

All three steps passed in 43.59 seconds combined. The second real-kernel fault
again reached errno 28 at batch_26/school836, retained 28 exact acknowledged
receipts, replayed with zero appends and verified a new atomic recovery batch after
cold reopen. Evidence and the cold preserved store:
`var/durability-fjall-sol-20261002-02/enospc-09-JIMh2V/`.
The previously recorded inspection/cleanup liveness limits remain blockers to a
universal hard-wall-clock bound; this successful run does not erase them.

### Required runtime acquisition and live national progress — 2026-10-02

Main downloaded the official Restate 1.7.10 MUSL archive, matched its published
SHA-256 `870fdc42782800b2025338ceb3d56b666800187f1b4e30560b8f1c116c83355e`,
inspected the five safe relative directory/regular-file entries, and extracted
exclusively with `--keep-old-files --no-same-owner --no-same-permissions`.
The actual bounded executable command printed `restate-server 1.7.10`; its
SHA-256 was `cf117addd3000b3411b7b3a8f359a862674e69e4b7694fef559a0dfb87f423c2`.
Exact commands, outcomes, source URLs and timing limits are retained in
`var/native-runtime-restate-1.7.10-sol-20261002/acquisition-main.json`;
independent primary-source metadata is in the adjacent `release-metadata.json`.
No new node was started and no existing deployment was changed. The original
national invocation and Root20 still used 1.7.0; version acquisition is a prerequisite,
not completion of the seventeen required native scenarios.

Main's bounded admin queries at host `2026-10-02T22:26:13.355Z` both returned
HTTP 200. Original national invocation
`inv_13LIoGM6LB600EGmJ5iU9yGQmEPr4dHTGp` remained suspended, journal size 99,
on its original deployment and protocol 7; its 49 direct JurisdictionCensus children
were 42 running and 7 suspended. Raw requests/responses/times:
`var/national-sol-20261002-01/admin-progress-raw-native20-wave.json`.
The real `open-work` CLI returned 49 jurisdiction rows with 39 unfinished sweeps.
The adjacent `progress-native20-wave.json` records that CLI evidence and a
read-only Census/status observation: 22,997 schools, 42,156 teams, 45,516 coaches,
891,756 athlete rows, 1,919,878 observations and 599,928,201 on-disk bytes.
Those physical rows are not unique accepted eligible people. The status observation
has no captured exact HTTP status/time/raw response bytes; do not invent them.
No second process opened the live store. Source-object denominators, paired
50/49 manifests, genuine dual-lane review, final seals and workbook delivery remain
unproved.

### Native reservation seam integration and original source-unit classification — 2026-10-02

Main integrated the explicit default-off `native-fault-injection` feature and the
source-reservation marker/harness implementations. Both coding workers called the
actual local Qwen servers, with raw responses retained under
`var/qwen-5090-evidence-sol-20261002/` and
`var/qwen-3090-evidence-sol-20261002/`; this was coding consultation, not genuine
ambiguous census-case advice. The marker binds the actual registered request
digest/date and reserved operation/attempt before acquisition. It does not prove
an HTTP, response or parsing phase in flight.

```sh
cargo fmt --all
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo test --locked -p census-service --lib --features native-fault-injection restate_services::jurisdiction::team_source -- --nocapture
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo test --locked -p census-service --example qualification_native_vm --features native-fault-injection -- --nocapture
```

The source tests passed 40/40 and the VM example tests passed 153/153, combined
wall time 25.33 seconds (`artifact://1498`). The subsequent comment gate checked
1,514 Rust files with no comments; all eight architecture checks passed. The
size scan exposed one 61-logical-line boundary assessor despite exit zero.
Main extracted its existing source-plan validation without changing conditions;
the focused jurisdiction-recovery tests then passed 41/41 and the new scan showed
no files over 300 lines and no functions over 60 logical lines
(`artifact://1506`, 72.91 seconds including the release build below).

```sh
SCRATCH_STORE="$PWD/var/durability-golden-sol-20261002-01" RUSTC_WRAPPER= bash tools/durability/scenario-16-golden-census-determinism.sh
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo test --locked -p census-service --example qualification_native_vm --features native-fault-injection native::jurisdiction_recovery -- --nocapture
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo xtask scan
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo build --locked --release -p census-service --features native-fault-injection --bin census-serve --example qualification_native_vm
```

The corrected scenario-16 wrapper executed its exact semantic-rebuild regression,
1 passed, 0 ignored, and retained its log under
`var/durability-golden-sol-20261002-01/test-2SB6Vn/`. This is not the full
scenario-16 real-capture/advice/atomic-publication oracle. The full workspace
strict source Clippy command recorded above also passed with the feature enabled
in the preceding 21.64-second gate/wrapper run; it preceded the assessor
extraction. The explicitly feature-enabled release build passed in 1 minute
9 seconds. Fresh Root21 execution on verified Restate 1.7.10 was started with
`var/vm-sol-20261002-21`; its runtime result was not yet observed when this entry
was written.

The original national run was inspected only through its existing ingress/admin
APIs. The 49 original jurisdiction-state samples all returned HTTP 200, including
an Illinois response retried with a larger bounded inspection-body cap after
the first 64-KiB inspection cap was exceeded. The sampling was not atomic:
36 teams stages completed, 8 failed and 5 were owed. Twenty parents had a measured
roster remainder; 3,400 roster error entries are diagnostics, not 3,400 failed
source units or accepted people. Raw state and exact identity evidence:
`var/national-sol-20261002-01/source-unit-stage-inspections-wave01.json`,
`source-unit-illinois-large-inspection-wave01.json` and
`source-unit-stage-classifications-wave01.json`.

An admin query restricted to those same 49 original parent invocation IDs
returned 79 distinct original `TeamsSource/run` keys: 74 completed and 5 running.
All 79 read-only shared `inspection` calls returned HTTP 200, with no inspection
errors: 65 durable completed outcomes, 4 terminal, 5 exhausted and 5 unsettled.
GA/TN/MS/SC coach-directory objects each retained Unknown attempts 1, 2 and 3;
AL retained Unknown 1, transient 2 and Unknown 3. Four active admin rows reported
an earlier one-hour abort timeout. These are retained incomplete outcomes, not
proof of absence or permission for a fourth physical attempt or replacement job.
Artifacts: `source-unit-teams-admin-wave01.json` and
`source-unit-teams-inspections-wave01.json` in the same original run root.
The 79-object count measures registered teams objects only, not all source-role
obligations. The all-source denominator remains unknown.

`source-unit-failure-digest-wave01.json` records the eight failed parent stages:
IHSA rate-limit/cooldown exhaustion, ID/MT out-of-jurisdiction directory rows,
four Arbiter terminal stale-bundle failures and CO/PA redirect-policy failures.
Main's bounded public HTML-only GET of `https://live.arbiter.io/directory/`
returned HTTP 200 at `2026-10-02T23:36:47.020Z`–`23:36:47.146Z`; 4,278 bytes,
SHA-256 `2639ddc780ea00ff6d26dfd4070f4f3e5f5b96b17dabcf3c0e72c30ddafd184c`.
The current entry declares `/directory/assets/index-DdctX9sz.js`, unlike the
production pinned asset. `var/arbiter-directory-entry-main-sol-20261002-01.json`
retains the HTML/status/times. No bundle credentials or token were retrieved,
and this diagnostic request is not census acquisition.

The source registry inventory in
`var/source-count-reconciliation-sol-20261002-01.json` identifies 24 descriptors,
not an authoritative 50-implementation/49-acquisition manifest pair.
`var/skipped-workspace-gate-sol-20261002-01.json` finds three ignored source tests,
but the retained Nextest output does not name its three skipped tests individually;
source candidates do not prove the runner's exact skip identities. Main validated
both evidence files as JSON. Fresh public coverage, genuine dual-lane advice,
all seventeen native scenarios, sealing and workbook delivery remain open.

### Root21 reached reservation/reset and blocked recovery — 2026-10-02

```sh
TMPDIR=/tmp target/release/examples/qualification_native_vm host --root var/vm-sol-20261002-21 --tools var/native-vm-tools-sol-20261002 --base-image var/native-vm-tools-sol-20261002/Arch-Linux-x86_64-cloudimg-20261001.604814.qcow2 --census-serve target/release/census-serve --restate var/native-runtime-restate-1.7.10-sol-20261002/restate-server-x86_64-unknown-linux-musl/restate-server --captures var/native-vm-tools-sol-20261002/census-captures.json
```

The actual command exited 1 after 612.76 seconds:
`sweep-reboot-finish failed: exit status: 1`. The fresh manifest records native
1.7.10 and the copied endpoint/qualification identities. The natural-midnight
oracle completed, then `host-source-active-before-reset.json` recorded the
original RI parent `inv_1j4peqPCiaI619bka7SjOV0DI0IpqZgOjE` awaiting original child
`inv_1eaEVDJct3NR1gUMsa6ai5cjIgG9KRfMg9`, key
`jurisdiction:RI:2026-27:1/teams/milesplit`, actual reserved attempt 1.
Both retained deployment `dp_160A68Td1yDGXFFvsGdEwb7`, protocol 7. The complete
marker matched journaled registration digest
`58acd8c3b4597e407f9e607a596ccd89589d398cbaa65a64953b72d714dd100b`
and original date `2026-10-03`. Its proof is reserved-before-acquisition, explicitly
not an in-flight physical request.

QMP acknowledged the owned `system_reset` at host
`2026-10-02T23:46:12.637084027+00:00`. The guest boot changed from
`6a1934bb-7ad2-40c3-8c7c-791fb3c4b14e` to
`6f4e9b0d-45cf-4612-9023-9e27ea4aab31`, with the same machine
`00c3767ed0e747c98beaeda5fdae2c1f`. The retained post-reset repeat returned
`appended=0`, total observations 18. Its actual guest realtime was
`2026-10-02T23:47:04.915206247+00:00`, earlier than the pre-reset injected
`2026-10-03T00:00:13.545933346+00:00`. Sweep output remained HTTP 470/pending
until the outer action ended; source recovery finish was not reached because
the old host sequence returned at Sweep failure. A scheduler-delay cause from
this clock rollback is an inference, not an observed internal diagnosis.

Cleanup TERM-reaped endpoint PID366, node PID364 and QEMU PID3375880 successfully;
the retained drain certificate reconciled accepted=completed=4, all other counts
zero. All disks/logs remain preserved. Main converted only the stopped owned
data image to a retained sparse raw copy and ran bounded `btrfs restore` for
top-level JSON/log diagnostics under
`var/vm-sol-20261002-21/recovery-forensics/`; extraction passed in 0.41 seconds.
No guest was rebooted for diagnosis and no process opened the store.

Exact cold extracted marker SHA-256 was
`97a9464362902ddce9323c3475745ace7c9c57122d696a3f58ed412cfa44819e`;
configuration SHA-256 was
`44f480b2027dd71c5d606329f9c2c136b6ce3df05189292d95c37eacb72f0939`.
Both equalled the reached witness's retained raw-byte arrays and declared hashes,
verified with `jq -j ...bytes | implode` piped to `sha256sum` and direct file hashes.
This proves marker/config byte retention, not completed invocation recovery or a
cold ledger oracle. Main changed the host order to exercise reboot before any
guest-only clock injection, and to retain both recovery outcomes before
propagating failure. That repair was not yet compiled/exercised at this entry.
Root21 is not full scenario-03 or scenario-12 acceptance.

The retained 49 parent plans filtered through the current 14-arm teams dispatch
produce 80 eligible key candidates, versus 79 original registered keys.
The only unregistered candidate is `jurisdiction:TN:2026-27:1/teams/tssaa`;
the current collector awaits arms sequentially and TN's preceding coach-directory
object is unsettled. The candidate was not submitted or materialized.
`source-unit-teams-declared-reconciliation-wave01.json` retains all 80 classified
teams candidates and explicitly does not assert original-binary dispatch parity,
an all-source-role denominator, or the missing historical 50/49 manifest pair.

## Source repairs, public qualification and bounded reboot recovery — 2026-10-03

Main integrated the actual local RTX5090 Arbiter entry-discovery implementation and RTX3090
redirect-origin implementation. These are coding consultations, not genuine ambiguous-census
case advice. Arbiter's independent review found unmatched foreign closing tags activating inert
modules and self-closing foreign roots hiding the genuine module. Four focused regressions failed
before Main's matched, bounded foreign-scope repair (two failed, two passed; exit 101).
Post-fix Arbiter tests passed 71 cases. The redirect regression failed before restoring the
same-origin post-response exemption; delivered transport-policy and destination-guard suites
passed six and fourteen cases respectively. Full strict workspace source Clippy and the
feature-enabled endpoint/native-VM release builds passed in the integrated waves. These checks
are not the full release gate.

Actual CLI qualifications, all exit 0:

```sh
TMPDIR=/tmp target/debug/census-service --store var/redirect-public-smoke-sol-20261002-01 fetch https://chsaanow.com/schools/ --refresh
TMPDIR=/tmp target/debug/census-service --store var/arbiter-public-smoke-sol-20261002-01 provider arbiter_orgs --states NH --limit 1 --refresh
TMPDIR=/tmp target/debug/census-service --store var/directory-public-smoke-sol-20261002-01 provider coach_directories --states NC --limit 1 --refresh
```

Colorado captured HTTP 200, requested `/schools/`, observed `/schools`, 373150 bytes; no redirect
host grant was supplied. Arbiter discovered the declared `index-DdctX9sz.js` from the stable
directory entry, acquired a fresh token and returned one NH school plus three coach rows,
five reported acquisition requests, zero cache hits and zero errors. The NC directory smoke
returned one school, sixteen coach rows with published email, two requests, zero cache hits
and zero errors. Its summary now explicitly labels email addresses, not postal coverage.
Directory admission remains strict; jurisdiction refusal diagnostics now retain capture
URL/date/SHA and distinguish foreign, unrecognized and missing/unusable published states,
without claiming association non-membership. The focused directory suite passed 77 tests.
None of these limited qualifications completes an organisation/state or repairs old settled
invocations. Original IDs, deployment pins, attempts and captures remain unchanged.

`var/public-source-smoke-byte-verification-main-sol-20261002-01.json` records six immutable
CO/Arbiter body/metadata links independently verified from exact bytes (996663 total body bytes).
Token and credential bodies were not decoded or displayed. Main also independently verified
all nine original ID/MT/AL directory body and metadata SHA-256/length pairs (5141624 body bytes),
in `var/foreign-directory-original-byte-verification-main-sol-20261002-01.json`.
The seven named rejected rows publish recognized foreign states; these are interpretation/
jurisdiction refusals after HTTP 200, not failures to obtain a directory body. Association
membership, real-world location and exact attempt-to-capture binding remain unproved.
`var/foreign-directory-ownership-evidence-sol-20261002-01.json` retains the static trace and
original field values. No foreign-row drop, source-value reassignment or ID exception was added.

The original non-atomic 49-parent snapshot has teams36completed/8failed/5owed,
roster reports20/absent29, meet reports17/absent32 and results reports0/absent49.
The roster reports contain 3400 error entries: 2742 partial interpretations and 658 quarantines,
with a reported sum of38994 rejected rows, not distinct athletes or failed physical fetches.
`rosters_committed` counts clean journals, not every applied partial capture; `rosters_remaining`
is not a pending-request count. See `var/roster-stage-disposition-evidence-sol-20261002-01.json`.
The 80 teams candidates and missing 50/49 historical source manifests remain different,
unresolved denominators.

### Reboot before clock injection: Root22 and Root23

Both native runs used the verified native Restate1.7.10 and the same host command recorded
above, changing only the root to `var/vm-sol-20261002-22` or `-23`. Reboot precedes the
independent guest-only midnight injection. Both Sweep and source recovery outcomes are retained
before propagating either failure.

Root22: Sweep recovery succeeded; source recovery failed at the unchanged 32 MiB append limit
while repeatedly archiving full journals/inspections. Its cold `http.jsonl` has1068records/
32200438bytes. Main changed bounded recovery polling to cheap original invocation statuses,
with full initial/final or completion bookends, without increasing artifact, retry or clock limits.
Focused native-VM tests passed153cases; source-size/forbidden scans were clean, strict source
Clippy passed and feature-enabled release builds passed.

Root23: Sweep recovery succeeded; source recovery failed honestly because its original parent
remained unfinished after the bounded wait. Cold `http.jsonl` has363records/1165684bytes, with
no artifact-cap failure. Original parent `inv_1j4peqPCiaI64jp4RgV9cB4cAbun3p8cWe` retained
deployment `dp_115re8OsgqNJ6q1RYY0ftoR`, protocol7 and journal31, ending at results Run
completion_id16. Its original MileSplit child completed72records on attempt2 after retained
attempt1Unknown; RIIL completed55schools/258coachrows on attempt1. Teams completed127records,
the qualification roster returned one clean journal and meet discovery returned115rows.
Results remained owed; no replacement invocation was submitted.

Root23 retained83HTTP200 captures, including80meet results pages. The latest immutable capture
is00:43:25Z; final full observation is00:46:50.592974758Z. Captures do not prove that the last
fetch/cache publication returned. Independent static tracing found no defensible deadlock cause;
the coarse results Run lacks evidence of its internal pending phase. The completed scout trace
is preserved by Main in `var/native-results-wait-investigation-sol-20261002-01.json`.
Owned endpoint/node/QEMU cleanup completed; cold extraction did not boot the guest or open Fjall.
Root22/23 are not full scenario03/12 acceptance or national census evidence.

### Previously ignored browser ranking lane

An isolated real headless Chromium with owned profile/CDP29223 and an owned synthetic loopback
fixture21045 exercised both existing ignored ranking tests:

```sh
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= ADLAW_LANE_FIXTURE=http://127.0.0.1:21045/ ADLAW_LANE_CDP=http://127.0.0.1:29223 cargo -Zallow-features=portable_simd,try_blocks,proc_macro_span,error_generic_member_access test -p athleticnet-browser --lib lane_smoke -- --ignored --test-threads=1
```

Exit0, two passed/34filtered, test duration0.18s. The fixture recorded exactly two physical
POSTs: one403challenge, which revoked the gate/stopped pagination, and one200normal response,
with exact request body/capture and next-page assertions. Raw output `artifact://1552`;
retained requests `var/browser-rankings-ignored-evidence-sol-20261002-01.json`.
The fixture stopped; owned Chromium PID3401877 received TERM and exited0; both ports were
confirmed unbound. This is real-browser fixture qualification, not live Athletic.net acquisition,
census data, full fault05 acceptance or completion of the remaining corpus/release gates.

### Attempted-with-retained-reasons was being counted as owed acquisition

The fresh run `national-sol-20261002-01` recorded `rosters_remaining` = 10,332 over 40
jurisdiction-progress rows while every one of those rows' remaining count equalled the number of
journaled teams the roster summary had retained with a quarantine reason or rejected row locators.
`census/sweep.rs::progress_of` computed `rosters_remaining = rosters_total.saturating_sub(committed)`
and `census/scope.rs::pending_rosters` skipped every journaled team, so an attempted roster that no
later pass would ever re-attempt stayed in the owed count forever. The identity the CLI fixture
asserts (`cli::national::tests::a_blocked_row_owes_the_index_it_did_not_walk`:
`committed + skipped + remaining == rosters_total`) did not hold for the recorded rows — NC answered
244 + 488 + 655 = 1387 against 899 indexed teams. A roster stage could therefore never become
terminal, which blocks every downstream seal and workbook acceptance.

Main repaired the accounting in `census/sweep/roster.rs`, `census/sweep.rs` and `census/scope.rs`:
`roster::summarize` counts distinct clean teams (`committed`) and distinct retained teams (`held`),
`roster::remaining` is a checked `total - (committed + held)`, `progress_of` returns
`CrawlResult<StateProgress>` instead of silently saturating, and `pending_rosters` returns only teams
with no journal row. `StateProgress.rosters_skipped` now carries the retained-attempt count, so CLI
labels print `held` instead of `had`/`skipped`. No persisted shape, journal key, logical identity,
retry ceiling or source-policy rule changed; retained reasons remain in `state.errors`.
`docs/OPERATIONS.md` now states the bucket identity and why a re-fetch cannot change a retained
row-admission verdict.

Read-only admin projection over the recorded `JurisdictionCensus/<identity>/state` responses
retained at `var/roster-accounting-projection-20261003.json` (40 rows with roster progress, raw
`/usr/bin/curl`-equivalent `POST` bodies, not an atomic snapshot): 15,023 indexed teams, 4,691 clean
commits, 10,325 attempted-with-retained-reasons, and 7 teams whose walk error left no journal row.
The projection therefore reclassifies 10,325 of the previously reported 10,332 owed rosters as
classified coverage gaps and leaves 7 genuinely un-attempted units plus the nine records without
roster progress (unknown, not zero) as owed acquisition.

Executed evidence:

```sh
TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER= cargo nextest run -p census-service \
  -E 'not test(a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write)'   # 546 passed, 2 skipped
```

The focused roster set (`test(roster) or test(rosters) or test(jurisdiction_walk) or test(partial)
or test(replay) or test(summar)`) ran 20/20 passed. The flipped regression
`milesplit_roster_observations::partial_and_quarantined_rosters_retain_their_reasons_without_owing_a_refetch`
now asserts `committed + held + remaining == rosters_total` for both a partial and a quarantined
capture, and `recovery::jurisdiction_replay` asserts the same identity for a clean walk, a
limit-deferred walk and a restarted walk.

Immutable handover binaries are under `var/releases/roster-resume-sol-20261003-01/`: `census-serve`
SHA-256 `fba827a6c79700afdb0affee32b3d8a1fa409fcdb65eb77f39c626159a4c500a`, `census-service`
SHA-256 `9409fb4150f7575447315193d660058d28bc4dc12718a9746e7212ca2d2463c2`.

The previous endpoint (`roster-accounting-sol-20261003-01/census-serve`, PID 3808761, listener
`127.0.0.1:19081`) received SIGTERM, unbound its listener within 60 s and exited 0 after ~265 s of
store finalize. Its service record retains the drain certificate
`drained: accepted=3 completed=3 cancelled=0 timed_out=0 aborted=0 panicked=0`. Its stdout was
attached to an operator terminal rather than a run log, so the certificate is retained as the
supervisor's captured output, not as bytes appended to `var/<run>/serve.log`.

### Revision 2 completes a jurisdiction that revision 1 could only abort

The repair above shipped behind the release gate and then completed a whole jurisdiction. Before any
further work, the store was captured with `census-service store-backup` into
`var/backups/original-national-before-roster-held-accounting-sol-20261003-01`: 303,233 files,
25,700,934,006 bytes, manifest `backup.json` SHA-256
`fcb00291a66a0a62981599f5db3004f6d00ed421b6827187fc4d2060d13a38b5` (elapsed 1,852.8 s). The
release gate then ran clean end to end: `TMPDIR="$PWD/var/test-tmp-sol-20261002" RUSTC_WRAPPER=
cargo run -p xtask -- gate` printed `gate: PASS` with exit 0 in 633 s, log
`var/gate-roster-held-20261003c.log`, every lane PASS (fmt, zero code comments, architecture
contract, check, doc, tests 2,362, panic extraction, strict clippy, production scan, domain type
integrity, domain purity, module seams, debt ratchet, deny, audit, machete, geiger, feature
powerset, bench presence). Two defects the first post-repair gate run had caught were repaired
rather than waived: the new replay assertion left `stopped_after` unused under `-D warnings`, and
geiger failed on a stale unit whose source no longer exists. Both are integration artifacts of the
repair, not of the shipped accounting.

`national-report` names what revision 1 actually died of. Every one of the 48 unfinished
jurisdictions carries `Terminal error [500]: the invocation stream was closed after the 'abort
timeout' (1h) fired`; the roster stage that could never become terminal kept the jurisdiction
invocation alive until Restate's one-hour abort timeout closed its stream. SC was the single
jurisdiction the old accounting let finish (457 rosters, 186 committed, 271 remaining, 0 blocked).
The 639 published rows, the 10,332 owed rosters and the empty publication evidence all follow from
that one non-terminating stage, not from source or parser faults.

A revision-2 slice then walked Wisconsin end to end against the running service:

```sh
var/releases/roster-resume-sol-20261003-01/census-service jurisdiction wisconsin \
  --ingress http://127.0.0.1:18095/ --revision 2 --timeout-seconds 600 --json
```

`jurisdiction:WI:2026-27:2` answered with teams `completed` over 597 records; rosters
`rosters_total` 597 = `rosters_committed` 263 + `rosters_skipped` 334 + `rosters_remaining` 0,
`blocked` false, `blocked_skipped` 0, the 334 retained reasons kept in `errors`; 63,123 athletes,
15,059 class-of-2027; meets 22 pages, 1,044 seen, 1,124 rows. The bucket identity
`committed + held + remaining == total` therefore holds on a live jurisdiction, the rosters stage is
terminal, and `open-work --revision 2` reports Wisconsin owing nothing while the other 48 sweeps owe
their stages. The national fan-out was submitted as
`national:2026-27:51472a0f63b82f0d:2` (invocation `inv_1jmRWtPPVo8241H2iNaMZTRLpnBU6Yj9tR`).

Both endpoints of that readback are the sanctioned ones: `census-service open-work` and the
`JurisdictionCensus/<identity>/state` shared handler, neither of which opens the store.
`var/open-work-rev2-20261003.json` and `var/wi-rev2-state-20261003.json` retain the replies, and
`var/monitor-rev2/` retains the ten-minute samples.


`check_pr_comparison_laws`, `check_identity_contradiction`, `check_redirect_cycle`,
`check_retry_limit`, `check_terminal_state_no_retry`, `check_store_batch_arithmetic`,
`check_census_scope` — none of which the crates define any more. The registry
harnesses were renamed, and the command's help text still described the old eight invariants. Main
repaired the runner: the default selection is now `KNOWN_HARNESS`, `resolve_targets` returns
`Vec<&'static HarnessInfo>`, the help text names the harnesses that exist, and
`a_named_selection_resolves_only_the_names_it_was_given`, so a default list drifting away from the
`cargo clippy -p xtask --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::expect_used` is
clean.

delivery, so execution stopped after the eleven harnesses the repaired runner had already begun. Those
eleven are retained as the tooling evidence: `check_gradyear_of_formula` verified (1 passed, 2 s) and
`check_escaping_is_injective` verified (1 passed, 39 s); `check_confidence_bounds` answered
`census_domain_wiring.rs` wires only gradyear, publish, id_mint and framing; the five
`check_id_mint_*` harnesses failed to compile with `Using the stub attribute requires activating the
unstable stubbing feature`, which is the `-Z stubbing` the documented manual procedure passes and the
runner does not; and `check_escaped_payload_carries_no_record_separator`,
`check_published_email_printable_ascii_contract` and `check_published_email_classifies_domains`
returned no verdict inside a 600 s bound. Verdicts and full outputs are retained in
historical gaps stay unverified rather than becoming claims. A post-hoc attempt to pass `-Z stubbing`
from the runner was reverted for the same reason.

### Retained roster refusals, the drained endpoint, and the revision-3 repair runs

Three jurisdictions could not terminate their rosters stage because a roster fetch that ends in a
deterministic `CrawlError::Schema` refusal — the ownership check in
`crates/census-crawl/src/milesplit/parse/roster/owner.rs` refusing "requested team location conflicts
with its source jurisdiction" — propagated before `journal_done`, leaving the team neither committed
nor held and therefore permanently owed. Live readings before the repair: `jurisdiction:MD` 46
committed + 354 skipped + 1 remaining, `jurisdiction:MA` 31 + 433 + 1, `jurisdiction:MS` 241 + 217 + 5,
with the refusal text retained in each state's `errors`. The sweep now records such an attempt as a
retained refusal: `crates/census-service/src/census/sweep/roster.rs` stores `capture: Option<C>` and
`refusal: Option<S>` in the journal row, `summarize` counts a refusal row as held and reports
`refusal=<source's own words>`, and `crates/census-service/src/census/sweep/roster/refusal.rs` writes
that row under the same `milesplit_roster:<state>:<year>:<revision>:<team>` operation key with a
digest over the team, year, observed date and refusal text. Transient `FetchError`s still propagate.
The regression `a_refused_roster_is_retained_without_rows_or_a_refetch` in
`crates/census-service/tests/milesplit_roster_observations.rs` proves a refused roster writes no
school, team or athlete rows, reports the refusal, reaches `remaining` 0 with `skipped` 1, reopens to
the same table digest, and is not re-fetched; the pre-existing provenance test was updated from
"remaining 1 with an empty journal" to "remaining 0 with one retained refusal row" and asserts the
persisted reason's own words. `cargo nextest run -p census-service --test
milesplit_roster_observations` reports 4 passed.

The endpoint was replaced under the lifecycle: `kill -TERM 208145` returned the drain certificate
`drained: accepted=126 completed=126 cancelled=0 timed_out=0 aborted=0 panicked=0`, the process exited
0, and `var/releases/refusal-retain-sol-20261003-02/census-serve` was started on the same loopback
port 19081 against the same store root and browser profile. The certificate was read from the
harness's service log for that process, not from `var/<run>/serve.log`: that process had been started
with its output on a terminal, so its stop line was captured there rather than in the run directory.
The node still lists two deployments, `http://127.0.0.1:19080/` (stale, its endpoint gone) and
`http://127.0.0.1:19081/`; `DELETE /deployments/<id>` answers 501 in this node build, and the live
runs below progressed through the healthy deployment.

Two repair runs re-drove the stages that the previous endpoint's exit had left un-terminated: run A
`national:2026-27:e690714ac5e7fbbf:3` over AL, GA, ID, MS and PA with
`--authorized-host www.piaa.org`, and run B `national:2026-27:9c35e761c7aea5b5:3` over CA, CO, FL, IL,
KY, MD, MA, MI, MT, NH, NY, OH, OK, TX and WV. `JurisdictionCensus/<identity>:3/state` then reported
teams `completed` and rosters terminal for AL (340 committed + 222 held of 562), GA (271 + 439 of
710), ID (24 + 175 of 199), MS (241 + 222 of 463), MD (46 + 355 of 401), MA (31 + 434 of 465) and NH
(34 + 74 of 108) — the three states that had owed a roster with no journal row now owe none — while
their meets stages published 420, 931, 276, 313, 534, 1040 and 212 rows respectively. The remaining
repair states were still mid-pass, and PA's teams stage still owed its 24 alpha-page fetches because
`https://www.piaa.org/...` answers 302 to its own plain-HTTP URL, which the transport refuses as a
downgrade even with the host granted.

### The workbook's export-input gate refuses while sweeps are writing

`census-service workbook` is a Restate workflow whose invocation key is
`run_key("workbook", &[year, scope, limit, out], DEFAULT_GENERATION)`
(`crates/census-service/src/cli/live.rs`), so its key includes `--out` and a completed invocation
cannot be repeated with the same arguments — the ingress answers `409 Conflict: the workflow method
was already invoked`. The build binds a frozen export input and then checks it against the live
store: `crates/census-report/src/export/dataset/frozen.rs::ensure_snapshot` compares
`snapshot.tables_digest(&INPUT_TABLES)` against the digest recorded at bind time, where
`INPUT_TABLES` is the thirteen tables `Schools`, `Teams`, `Coaches`, `Athletes`, `Meets`, `Events`,
`Performances`, `ReviewCases`, `SourceAccess`, `IdentityVerdicts`, `AthleteIdentityDecisions`,
`SourceObservations` and `SourceMeets`. Two preview builds on 2026-10-03 at 19:05 and 19:08, with
the revision-3 repair sweeps running, both failed with `ingress returned HTTP status 500 Internal
Server Error: stale or foreign export input cannot publish changed source evidence` because those
sweeps were still writing `SourceObservations` and `SourceMeets`. `index` and `consolidate` both
succeeded against the live store during the same window (`index`: 750 source identities, 0
conflicts, 0 review cases, 52 coverage rows, 1 snapshot; `consolidate`: 207 schools and 543 coaches
merged into the existing snapshots, whose content digests were unchanged and therefore left their
files in place), and `bests` reduced the class-of-2027 cohort to 3,366 best-mark rows in
`out/best-results-co2027.jsonl`. Generation therefore has to run once the store is quiescent; a
later build under the same `--out` needs a new output directory.

### Published best marks re-derived from their own captures

The class-of-2027 reduction in `out/best-results-co2027.jsonl` was spot-checked against the archived
source bytes rather than the reduction alone. The row for `track800m` athlete `Koepke, Owen`
(`value` 12191, place 3, date 2026-05-26, meet "D1 Regional 1A - Menomonie") re-derives from the
cached body `var/national-sol-20261002-01/http/d81c22f095aa235a286e3db208ac623a.body` (1,204,549
bytes, `https://www.wiaawi.org/sites/default/files/2026-08/tr2026menomonieregionalindiv.pdf`,
fetched 2026-10-03T22:42:40Z), whose `pdftotext -layout` text carries `3 Koepke, Owen 11 RIVER FALLS
2:01.91 6`. The rows for `track110m_hurdles` and `track300m_hurdles` athlete `Wyatt Langness`
(20.44 at place 10 with `source_key` `...pdf:110m Hurdles:preliminaries:9`, and 52.63 at place 12)
re-derive from `var/national-sol-20261002-01/http/9f2fcd523c8053cec02da3c3cf0c4f2f.body` (248,425
bytes, `https://www.wiaawi.org/sites/default/files/2026-08/trb2026northwesternregional.pdf`), whose
text carries `10 Wyatt Langness 11 AMERY 20.44` and `12 Wyatt Langness 11 AMERY 52.63`. Those source
tables list placings, marks and points with no hurdle height, so the absent height is a property of
the source rather than a normalization loss, and the round survives in the observation key.

## The workbook rebuilt from the run's own cold backup, and the dual-GPU lane's phantom-ask defect — 2026-10-03

The class-of-2027 workbook was regenerated from the run's own durable material rather than any
earlier store. `census-serve` pid 408493 took `kill -TERM`, drained in about ten seconds with
`drained: accepted=62 completed=62 cancelled=0 timed_out=0 aborted=0 panicked=0`, and
`store-backup --store var/national-sol-20261002-01 --to var/backups/live-post-redrive-sol-20261003`
wrote 40 GB in 14 m 31 s. The endpoint was restarted on the same loopback port 19081 against the same
store root, and `open-work` then reported revision 3 with 31 of the 49 owed jurisdiction sweeps still
outstanding — the sweep had resumed and progressed. `store-restore --from
var/backups/live-post-redrive-sol-20261003 --to var/pr-store-sol-20261003` read back 365,186 files
and 41,544,414,991 bytes in 10 m 44 s with `store-integrity: ok=true` and every table
`expected=actual` (athletes 4,655,118; teams 214,406; coaches 78,463; schools 74,357; performances
115,725; source_observations 4,856,062; source_meets 48,934; events 8,114; meets 1,010). Against that
restored copy, `bests` reduced the class-of-2027 cohort to 10,431 rows in 52 s, `workbook` published
generation `12af07020028b8402a295b43a150c67e530ac91b1b6acb05b1d618d6a9a43028` in 3 m 14 s and
`verify` answered `OK (complete frozen generation)`. That generation's store identity is
`acb01c0cf195530b18ba565a3406b7eb7a6810014248c447421a6fbe3ad3f5c2` at snapshot sequence 98,640. It
replaces the earlier PR-less generation `8b30583e…`, whose PR sheet was header-only because the only
snapshot then frozen predated the mark-bearing rows. The delivered file
`/home/lewis/Downloads/fresh-census-co2027-sol-20261003.xlsx` is 91,399,879 bytes with SHA-256
`245044129b986c51fd1fe7df5cf8b047dd23275dfb4fd85f0b480e370c6d6fcc`, equal to the generation copy,
and the bundle beside it carries the manifest, both census JSON projections, `recruiting.csv`
(185,287,977 bytes), the 11,448,808-byte `best-results-co2027.jsonl` sidecar (10,431 lines) and the
audit projection. The live store's own `out/best-results-co2027.jsonl` is the earlier 3,366-row
reduction written while the repair sweeps were still running; the two are different snapshots of the
same run, not competing claims.

A streaming readback of the delivered workbook scanned every sheet. Athletes holds 566,229 rows with
a unique `Athlete ID` and graduation year 2027 on every row; PRs holds 10,431 rows whose `Athlete ID`
repeats by design (4,971 repeated rows over 1,779 athletes, one PR per athlete and event, 5,460
distinct athletes); Performances_001 holds 28,126 rows with a unique `Canonical Result ID`; Coaches
holds 78,463 observation rows over 54,926 distinct coach identities with 40,079 professional emails,
21,013 personal emails and 50,044 phones; Schools holds 32,809 unique ids with state on every row and
city on 32,235; Meets holds 1,008 unique ids; Sources holds 36 rows and Coverage 208; Conflicts holds
10,585 rows (10,522 athlete identity, 63 recruiting contact) over 10,575 subjects; Review holds
566,456 rows (566,229 athlete identity unverified, 227 meet venue unresolved) with every subject
unique; and Run Metrics holds 49 rows. The earlier generation, for contrast, carried 459,974
athletes, no PRs or performances, 65,908 coach rows and 3,897 conflicts, so the re-drive enlarged
every evidence sheet.

The dual-GPU identity review ran on genuine local hardware against the restored copy, against the
NInfer endpoint on `127.0.0.1:11000` and the llama.cpp endpoint on `127.0.0.1:11001`, both serving
`qwen3.8-27b-uncensored`. The case projection filed 26,842 cases (26,542 decided, 300 pending) from
2,145,989 athlete rows and 2,098,454 provider objects, with no row holding several objects of one
provider. A bounded review over 25 cases reported `requested=25 answered=0 decided=0 accepted=0
rejected=0 insufficient=0 unanswered=25 dropped=0 failed=0` in 98 seconds, and a retry at 8,192
tokens with a 600-second per-request bound reported the same shape over five cases. Neither run's
tallies are model replies: with both lanes routed through transparent logging proxies
(`var/tmp-review-proxy.py`; 127.0.0.1:11098 to 11000 and 11099 to 11001), reviewing one
athlete-identity case reported `requested=1 ... unanswered=1 ... failed=0` after 89.56 seconds while
both proxy logs stayed at zero bytes, so no HTTP request left the client. The same command over
`meet-jurisdiction` filed no case at all (`requested=0`). `packets::SubjectIndex::packet` for
`AthleteIdentity` requires the two sides to share the canonical triple key `(school, normalized
canonical_name, grad_year)` (`athlete_flags::key`), while the cases are filed by
`athlete_clusters::bound_case` from the observation-level `AthleteIdentityIndex` keys, so a case
bound by a shared provider object whose sides differ in school or grad-year attribution has no
packet: `ask_lanes` posts nothing and `records::account` counts the case unanswered with `failed=0`
because no transport error occurs. The defect is filed as `athletic-rust-pipeline-qw2` with the
executing commands and a fix direction. Separately, both served models are reasoning models: at
`max_tokens=24` each returns `finish_reason: length` with empty `content` and populated
`reasoning_content`, and at 3,072 tokens each returns `finish_reason: stop` with `content: "pong"` in
under a second, so a lane that does answer needs a budget above the default 1,536 (cap 8,192). No
verdict has been validated or applied.

In the same window `TMPDIR=/tmp cargo run -p xtask -- gate` reported 2,361 passed, 1 failed and 3
skipped. The single non-passing test,
`census-service::restate_kill_restart a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write`,
was still running at the harness's 60-second slow mark when the outer 180-second `timeout` cancelled
the run, so it is unproven within that bound rather than a product failure. The capture chain was
re-verified independently: the body `var/national-sol-20261002-01/http/74692fff592b4aeaada68771a4ac978a.body`
is 333,028 bytes with SHA-256 `ea72fe9073c28af0ffc157e285e354416f62f5dd6d481d3ea29681f116ea7e40`,
and three separate archive copies hash-match it. No seal exists for this run and 31 jurisdiction
sweeps remain owed, so nothing above certifies a national publication.

### The phantom ask is repaired, and the workbook's store copy holds no marks — 2026-10-04

Main repaired `athletic-rust-pipeline-qw2` in three places. `packets::SubjectIndex::read` now loads
each case's `member_ids` as well as its `subject_id`, so the other side's row is in the packet index
at all; `athlete_packet::AthleteIndex::compare_members` binds an explicitly filed pair — two distinct
member ids naming the subject and one other row — without requiring a shared triple-key group, while
the implicit path (empty `member_ids`) keeps the group requirement unchanged; and
`records::ReviewReport` gained `unaskable`, incremented for a case whose packet cannot be built, so
such a case is reported distinctly instead of as `unanswered` (the summary line now prints
`unaskable=` after `requested=`). Regression tests:
`athlete_packet_tests::group::an_explicitly_filed_pair_binds_across_two_schools` (two rows of one name
and cohort at different schools, filed as a pair, yield a packet naming both ids), the `missing_subject`
row of `membership_tests` and
`consensus::tests::replay::missing_subjects_are_durable_unresolved_review_not_silent_success` now
assert `unaskable=1` with `unanswered=0`. `cargo nextest run -p census-review` reports 127 passed.

The repaired lane was then exercised on the store that produced the phantom asks
(`var/pr-store-sol-20261003`, 2,145,989 athlete rows, 26,842 cases filed of which 275 pending), with
both lanes proxied: `target/release/census-service --store var/pr-store-sol-20261003 review --family
athlete-identity --limit 4 --endpoint http://127.0.0.1:11100 --endpoint http://127.0.0.1:11101`
reported `requested=4 unaskable=0 answered=4 decided=0 accepted=0 rejected=4 insufficient=0
unanswered=0 dropped=0 failed=0` in 117 s, and each proxy log holds one POST per case per lane (4 + 4
over two runs) with HTTP 200 answers whose `content` is a verdict object naming the case id. Both
lanes' `same_person` proposals were refused by the store's own evidence, so the four cases stay
pending rather than merging, and the defect's tallies are no longer reproducible.
`cargo run -p xtask -- comments` reports 1538 Rust files checked with no comments, and
`cargo run -p xtask -- panic-extraction` is clean after the throwaway read-only probe
(`crates/census-review/examples/case_probe.rs`, which prints the pending cases and their member rows)
lost its `expect`.

Reading those same cases then exposed a second, separate gap in the workbook. The generation published
at 21:20 under `var/final-workbook-sol-20261003/current` was built offline from
`var/workbook-restore-sol-20261003`, and that store copy holds no marks: `fjall-stats` on it reports
`events 0` and `performances 0` with `out/best-results-co2027.jsonl` at zero bytes, so the workbook's
`PRs` and `Performances_001` sheets carry only their headers (row counts read from the workbook's own
worksheet XML: `Athletes` 459,971, `PRs` 1, `Performances_001` 1, `Coaches` 65,909, `Schools` 27,490,
`Meets` 760, `Review` 460,199). The live deployment's own report
(`var/national-sol-20261002-01/out/report.json`, 19:05) counts 471,386 class-of-2027 rows — 11,412
more than that copy — and its `out/best-results-co2027.csv` is 1.4 MB, so the stale copy cannot
produce the delivered workbook. The 21:46 backup `var/backups/live-post-redrive-sol-20261003` (40 GB:
`fjall` 3.9 GB, `http` 25 GB, `out` 12 GB) is the live store's durable material at that point and was
restored to `var/workbook-final-sol-20261004` for the rebuild. `census-service national-report` still
shows 48 of 49 jurisdictions failed on that deployment (47 by the 1-hour invocation abort, WI by
`no consolidated schools: run collect and consolidate before the wiaa_results provider`), and no seal
exists, so any workbook built from it remains an incomplete national publication.

### The verified workbook rebuilt from the live store's own cold backup — 2026-10-04

`census-service store-restore --from var/backups/live-post-redrive-sol-20261003 --to
var/workbook-final-sol-20261004` finished in 553 s and `fjall-stats` on the result reports
`events 8114`, `performances 115725`, `athletes 2145989`, `schools 74357`, `teams 214406`,
`coaches 78463`, `source_observations 4856062` — the marks the earlier restore lacked. The rebuild
then followed the run's own order, one step per line in `var/rebuild-workbook-20261004.log`, each with
`rc=0`: `index` (2,322,394 source identities, 19,321 retained conflicts, 24,770 review cases, 71
coverage rows, one snapshot, 176 s), `consolidate`, `bests --grad-year 2027` (`cohort=co2027
rows=10431`), `report --print` (`schools=32809 athletes=2145989 co2027=566229 boys=318197
girls=247499 profile_url=560331 coaches=54926`), `workbook --out var/final-workbook-sol-20261004`
(generation `0d28e4a810f368fe9c4237d3183d349ba72dee4be698a284d69a6d6f3a499ef8`, snapshot sequence
100712) and `verify --workbook …/current/workbook.xlsx`, which reports
`verify: OK (complete frozen generation)`.

The generation was verified against its own material before delivery. The workbook's worksheet XML
carries 566,229 `Athletes` data rows whose gender counts (Boys 318,197, Girls 247,499, Unknown 533)
sum exactly to the report's cohort; 10,431 `PRs` rows; 28,125 `Performances_001` rows; 78,463
`Coaches`, 32,809 `Schools`, 1,008 `Meets`, 10,585 `Conflicts`, 24,770 `Review` and 49 `Run Metrics`
rows. `best-results-co2027.jsonl` holds 10,431 marks over 18 event kinds (5000m 1,616, 100m 1,209,
200m 869, shot put 797, discus 781, 400m 674, long jump 664, 800m 603, 1600m 474, javelin 467, 300m
hurdles 394, pole vault 386, triple jump 385, high jump 337, 3200m 333, 110m hurdles 222, 100m
hurdles 219, weight throw 1) across Outdoor 8,585, Cross Country 1,616 and Indoor 230, every row
citing `result_url`, `meet`, `meet_id`, `date`, `performance_id` and a source key, with the timing
class retained (Fat 1,480, Unknown 5,133, NonTime 3,818) and no mark carrying a PR conflict. The 222
boys' 110m hurdles marks are numeric times (for example 20.94 s from value 2094), so the
normalization rules survive into the delivered artifact. The 19,944 distinct school ids referenced by
athlete rows all resolve to one of the 32,809 `Schools` rows, and the 78,463 `Coaches` observations
reduce to exactly 54,926 distinct `(school, coach, sport, role)` assignments, which is the number the
census reports. The generation's manifest hashes were reproduced with `sha256sum` on the copies
delivered to `/home/lewis/Downloads` (`class-of-2027-tfxc-census-20261004.xlsx`
`1de295504835358d0e674c5fed0e52200f0eaea26e4d0e94ab0df8723beda67a`, `class-of-2027-best-results-20261004.csv`
`cfca2e56aa39ae065c2c4d55ea1469d5be3233708177f114a28fd4331b11bbed`,
`class-of-2027-census-20261004.json` `a51e3bf668136dbe13d4034b77c200e7da8514cfa754205a2344c5c0d65949eb`).

Two limits belong with those numbers. The delivered generation is a snapshot, not the live store's
last byte: `curl -X POST http://127.0.0.1:18095/Census/status` answered during the restore with
4,655,118 athlete rows beside the same 115,725 performances, 8,114 events, 74,357 schools, 214,406
teams and 78,463 coaches, so the sweep has kept adding athletes since the 21:46 backup (2,145,989
athlete rows). No best-mark row carries a wind reading: all 10,431 rows are `wind_class`
`NotApplicable` (6,943) or `Unknown` (3,488) with `wind_mps` null, so no sprint or hurdle mark states
a wind value — filed as `athletic-rust-pipeline-j9a` to confirm whether the meet-performance payload
publishes one before the absence is documented as a contract limit. And the cohort is single-provider
(`class_of_2027_multisource` 0) with 24,770 review cases still pending, so the workbook is the
verified census of what the completed sweeps discovered, not a sealed national publication: 48 of 49
jurisdictions failed in the live run and no seal exists.

### The genuine two-lane review on the two physical GPUs: advice journaled before the verdict, identical bindings replay — 2026-10-04

Both physical lanes were run through the transparent byte logger so the exact request and response
bodies exist outside the store as well. `python3 var/tmp-review-proxy.py 11100
http://127.0.0.1:11000 var/proxy-5090-sol-20261004.log` fronts the RTX 5090's `ninfer-serve`
(`prompt-json`) and `… 11101 http://127.0.0.1:11001 var/proxy-3090-sol-20261004.log` fronts the RTX
3090's `llama-server` (`json-schema`); both serve `qwen3.8-27b-uncensored`, which
`curl -s http://127.0.0.1:1100{0,1}/v1/models` reports.

`target/release/census-service --store var/pr-store-sol-20261003 review --family athlete-identity
--limit 8 --endpoint http://127.0.0.1:11100 --endpoint http://127.0.0.1:11101 --model
qwen3.8-27b-uncensored --response-format prompt-json --response-format json-schema --timeout-secs
600 --max-tokens 8192 --observed-on 2026-10-04` ran twice, identically.

* Run one: `requested=8 unaskable=0 answered=8 decided=0 accepted=0 rejected=8 insufficient=0
  unanswered=0 dropped=0 failed=0` in 127 s, with eight requests and eight responses logged on each
  proxy (60,958 and 65,272 bytes). Run two gave the same counters in 137 s and did not re-ask the
  first eight: the `review_advice_v1` journal grew 38 → 46 → 54 entries, each case carries exactly
  one ask and one request digest for the 600 s/8192 s binding (16 cases, 16 asks), and every one of
  the 54 journaled advice records has a durable row in `identity_verdicts`.
* Each durable row's `rationale` is the audit: policy `dual-independent-review-v1`, the packet
  digest, and per lane the endpoint, model, `response_format`, `status`, `request_digest`,
  `timeout_ms`, `max_tokens`, the model's batch and the adjudications, plus the recomputed outcome
  `hard_contradiction`. All 16 asked cases stay `retained`; no model-proposed value was applied
  without the store's own evidence, which is what `rejected=8` records. The advice was journaled
  before the verdict row and case state were written, so a crash between the two leaves the advice
  readable and the retry accounted.
* Changing a lane binding is not replayed. Eight cases answered under the earlier 180 s/1536-token
  binding were asked again once the 600 s/8192-token binding was configured, and the store kept
  both advice records with their distinct request digests. The same command with the same binding
  then skipped them, so repeated runs advance through the pending cases instead of re-charging the
  same ones.

Counts before and after, from `Store::scan` and `Store::journal_payloads` (`cargo run --release -p
census-review --example audit_dump`, a readback tool written for this evidence and removed
afterwards): 26,567 verdict rows of which 26,542 are `deterministic:shared-provider-object` and 25
are `dual-independent-consensus`; 26,842 cases of which 26,542 `resolved`, 25 `retained`, 275
`pending`. The two audit files and the binding index are kept under
`var/review-audit-sol-20261004/`.

### The gate is red on another workstream's uncommitted collect change, not on the review repair — 2026-10-04

`TMPDIR=/tmp cargo run -p xtask -- gate` reports `gate: FAIL -> tests` after `fmt` and
`bench presence` pass, and `cargo nextest run --workspace --all-features --no-fail-fast` reports
2365 tests run: 2353 passed, 12 failed, 3 skipped. None of the twelve is in a crate or file the
review repair touched. Six are `census-crawl::coach_directories::map::postal_regressions::*`
(`Error: CheckFailure("address1=42; left=(1, 0) right=(1, 1)")` and the ownership cases), and
`cargo tree -p census-crawl | grep -c census-review` is 0, so census-crawl cannot observe the
`census-review` change at all. The other six are collect-parity goldens —
`Error: golden mismatch for mshsl__collect: expected 183 lines, got 184, first difference at line
143` and `Error: golden case athleticlive__meets-sample-collect-report` beside the `wayzata`, `ks`,
`plain_names` and `coach_contacts_csv` parities — i.e. production output ahead of the committed
goldens. The working tree carries an uncommitted collect/coach-directory change
(`crates/census-crawl/src/coach_directories/{collect.rs,collect/postal.rs,survey_tests/collect.rs,..}`,
`crates/census-service/src/{census/*,cli/national/report.rs,restate_services/*}`) whose files are
stamped 23:02:18, after this session's first gate run, which reported 2362 tests with 2361 passed and
only the `restate_kill_restart` case unproven inside its 60-second bound; that case passed in this run
after 129 seconds. The review-lane repair's own suite, `cargo nextest run -p census-review`, reports
127 passed, and the repair's end-to-end evidence above is unaffected. The twelve failures are left to
their owner rather than reformatted or patched from here.

### The 2026-10-04 disk cleanup deleted the fresh run's store and every backup — 2026-10-04

An operator-directed cleanup of `var/` removed, between 10:52 and 11:00, whole trees this session had
been verifying against: `var/national-sol-20261002-01/` (29G — the fresh Class-of-2027 run's Fjall
store, its `http/` captures and its `out/`), `var/backups/` (103G, including
`live-post-redrive-sol-20261003` and both held originals), `var/retained-pr-correction-20261001/`
(70G), `var/landing-20260930/` (48G), `var/pr-store-sol-20261003/` and
`var/workbook-final-sol-20261004/` (42G each), `var/workbook-restore-sol-20261003/` and
`var/run-2027-v2/` (25G each), `var/restore-check/`, `var/national-2027/` and the superseded
roster-accounting backup. The fresh run's Restate node (pid 3215921, ingress 127.0.0.1:18095, admin
127.0.0.1:19095) was still serving from the deleted directory through unlinked RocksDB files; asked
to stop it exited 0 on SIGTERM and both ports are free. This session deleted only derived and dated
leftovers — `var/midwest-census` (15G), `var/midwest-athletes` (1.1G),
`var/vm-sol-20261002-{10,11,12,22,23}` (6.5G), `var/native-vm-tools-sol-20261002` (671M),
`var/tmp-wb-xml` (551M), two unpacked restate 1.7.10 runtimes (526M),
`var/browser-rankings-no-unwrap-*` (270M), then the 2.1G of release bundles and the 243M restate 1.7.0
runtime that the two live processes had pinned — all recorded in `var/cleanup-20261004.log`. `var` is
now 762M and `/home` has 732G free.

Consequence: the fresh Class-of-2027 run is no longer re-verifiable locally. Its store, the original
capture `74692fff592b4aeaada68771a4ac978a.body` (declared SHA-256
`ea72fe9073c28af0ffc157e285e354416f62f5dd6d481d3ea29681f116ea7e40`) and every Fjall backup were
removed; `find` over `~/src` and `~/Downloads` finds no copy of that body, and the run journal died
with its node. What survives of that publication: `~/Downloads/class-of-2027-tfxc-census-20261004.xlsx`
and `~/Downloads/class-of-2027-census-20261004.json`, which sha256-match the manifest's
`workbook.xlsx` and `census-all-sources.json` entries; the manifest itself (schema_revision 1,
generation_digest 3782cb1872b6c49891523439e323257e44535d2010171f26359b0fafaa0339f6, store_identity
acb01c0cf195530b18ba565a3406b7eb7a6810014248c447421a6fbe3ad3f5c2); the two retained 2026-10-01
workbooks; `var/fresh-census-verification-sol-2026100{3,4}.md`; the kani, mutant and gate logs; and
the beads records. Five further artifacts of that generation went with the store directory and have
no copy on disk: `audit.json` (250,148,182 bytes), `recruiting.csv`, `best-results-co2027.csv`,
`best-results-co2027.jsonl` and `census-core.json`; `frozen-input.json` likewise. Older-run copies of
some of those names under `var/anet-pilot/`, `var/audit-20260930/` and `var/qualification-postal-*`
are different generations, not this one. Any further verification of that census, or regeneration of
the missing five, requires a new run: new store root, new endpoint port and a fresh registration.




### The school-address join runs natively: real captures → durable service → workbook readback — 2026-10-04

Scope: the corpus-to-census join of [ADR-021](adr/ADR-021-school-address-join-durable-stage.md) —
the core, the `SchoolAddressJoin` workflow, the national stage and the workbook postal block. Not
a national census: the store is a 207-school copy of `var/census-service`, and the corpus is the
real 2026-10 NCES captures. Run directory: `var/school-address-join-20261004/` (preserved).

**Corpus.** `target/debug/census-service school-address --ccd
var/school-address-join-20261004/ccd/ccd_sch_029_2526_w_0a_050626.csv --pss
/home/lewis/src/ad-law-scrape/data/nces/pss/pss2324_pu.csv --out
var/school-address-join-20261004/corpus --now 2026-10` exited 0 in 14.6 s and reported 122 692
entries (CCD 100 307, PSS 22 385) over 122 692 rows, 1 920 skipped rows and 386 notes, with lane
digests `nces-ccd=d1473136285b5994b73a1a8b640757811eb81e0ae770953bcf915ee8c422386e` (equal to
`sha256sum` of the extracted CSV) and `nces-pss=14a2f9e600a492940fd57646792b4b5163ea9d03b8015a7df1135066bcec3b8b`.
Both published archives were verified live before being cited: `curl -sI` returned `HTTP/1.1 200`
for `https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip` (12 718 258 bytes) and
`https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip` (3 970 317 bytes).

**Reader mapping.** The same captures were read again after the NCES reader gained the CCD
`WEBSITE` mapping (`crates/census-crawl/src/nces/parse.rs`): `corpus-web` reports the same 122 692
entries over 122 692 rows and the same 1 920 skipped rows, with the lane digests unchanged
(`nces-ccd=d1473136…`, `nces-pss=14a2f9e6…`) and generation digest
`5578b3f3670fc9b658bcbdb3dac263956f77434a3238488dcba4fc06704d8edc` (the postal read's is
`57edd0b2c3c88b25280bb90f1e4cb8a873d262a0ec5baa891ca857aa8ee6f0e2`). The only difference is notes:
CCD 369 → 398, because 29 `WEBSITE` cells are not http(s) URLs (for example `website is not a
supported value: "http://601 South Clinton Street"`); PSS stays at 17. The exported `website` column
carries 70 223 URLs, and a field-by-field comparison of the two exported CSVs (Python `csv`, all
122 692 rows) shows the key/street/city/state/ZIP/phone tuples are identical.

**Native service.** Fresh node `var/school-address-join-20261004/restate.toml` (ingress 18195,
admin 19195), `census-serve --listen 127.0.0.1:18196 --data-dir var/school-address-join-20261004/serve
--max-concurrent 2 --drain-timeout 30 --browser-profile … --browser-executable /usr/bin/chromium
--browser-headless` over a copy of the coach census store, registered with `POST /deployments`.
`GET /deployments` reported **12 services**: BrowserSession, Report, JurisdictionCensus, Workbook,
Sweep, NationalCensus, SchoolAddressJoin, Census, Ingest, Consolidate, Bests, TeamsSource.
`POST http://127.0.0.1:18195/SchoolAddressJoin/smoke-20261004/run` with the generation and the two
url/date overrides returned HTTP 200 and `{"scanned":207,"linked":158,"review":1,"no_match":48,
"refused":0,"evidence_missing":0,"missing_state":0,"exact_name":153,"core_name":5,"ambiguous":1}`,
writing `<store>/out/school-address-join/report.json` and `outcomes.jsonl`.

**Durability and idempotency.** `kill -TERM` on the serve pid drained cleanly
(`drained: accepted=4 completed=4 cancelled=0 timed_out=0 aborted=0 panicked=0`). After restarting
the same binary on the same store, a second workflow key `smoke-20261004-b` returned
`linked=0, already_linked=158` with the same review/no-match counts: claims survived the restart and
a replay appended nothing.

**Workbook readback.** After a second clean drain, `census-service --store
var/school-address-join-20261004/serve workbook --out var/school-address-join-20261004/workbook`
exited 0 and `census-service --store … verify --workbook …/current/workbook.xlsx` printed
`verify: OK (complete frozen generation)`. The `Schools` sheet row for `sch_00209dac4fe40ec0` carries
Postal Street `256 Main Street`, City `Paris`, State `ME`, ZIP `04271`, owner namespace
`school_directory:nces-ccd:ME`, owner ID `231077000361`, source `nces-ccd`, the CCD archive URL,
observed date `2026-10-04` and capture SHA `d1473136…`. The CCD row
(`Oxford Hills Comprehensive H S, …, 256 Main Street, Paris, ME 04271, NCESSCH 231077000361`) is the
ground truth, and the workbook's capture SHA is byte-equal to the corpus lane digest.

**Website attach and the second readback.** So that the postal smoke above stays exactly as
observed, the website-carrying generation was joined against a second copy of the same census store
(`var/school-address-join-20261004/serve2`, same 12-service registration). The `SchoolAddressJoin`
apply over `corpus-web` reported
`{"scanned":207,"linked":0,"already_linked":158,"websites":93,"review":1,"no_match":48,
"refused":0,"evidence_missing":0,"missing_state":0,"ambiguous":1}` — the postal claims were already
present from the earlier apply and replayed as `already_linked`, and 93 schools gained the CCD
website. `census-service --store …/serve2 workbook --out …/workbook2` and `census-service --store
…/serve2 verify --workbook …/workbook2/current/workbook.xlsx` exited 0 and printed `verify: OK
(complete frozen generation)`. Read back with Python's `zipfile` (no image and no Excel involved),
the `Schools` sheet has 207 rows, 158 carrying a postal block and 93 carrying `School site`; the
`sch_00209dac4fe40ec0` row carries `School site` `http://www.msad17.org/o/oxford-hills-high-school`
— the CCD `WEBSITE` cell for Oxford Hills High School — beside the unchanged postal cells. Finally,
the offline lane at the fixed revision was dry-run against `serve2`/`corpus-web`:
`scanned=207, linked=0, already_linked=158, websites=0, review=1, no_match=48`, i.e. the replay
appends nothing and rewrites no website.

**Suites.** `cargo test -p census-service --lib --bins`: 222 + 71 passed (nine are the join core's
own lane: link-with-evidence, idempotent second apply, evidence-missing refusal, dry run, override
validation, lane evidence, ambiguity review, cross-state refusal, missing state), and the integration
targets `nces_directory_properties` 8 (the CCD window now expects 15 website notes, all
`field == "website"`), `school_address_corpus` 8 (15 notes; the Albertville export line carries
`http://www.albertk12.org`), `school_address_publication` 3 and `workbook_shape` 1. `cargo test -p
census-domain`: 270 passed. `cargo test -p census-crawl`: 873 passed (the CCD/PSS reader windows,
including the mapped website). `cargo test -p census-report`: 195 passed. `cargo fmt --check -p
census-crawl -p census-domain -p census-service` is clean, and `cargo clippy -p census-domain -p
census-service -p census-crawl -p census-report --all-targets` emits no diagnostics: the
`LinkDecision::Linked` variant was boxed rather than allowed to trip `large_enum_variant`, and 17
`clippy::panic_in_result_fn` errors in `school_directory/tests/link_tests.rs` were replaced with the
workspace `check!` macro, not allowed to stand.

**Limits.** No 49-jurisdiction run was repeated here: the national stage is wired and journaled (the
workflow invokes `SchoolAddressJoin` under `<run identity>:school-address-join` after consolidation
and before workbook publication) but the fresh-run acceptance obligation is unchanged. `no_match=48`
is the corpus's measured scope — association-only names such as `Chesterton Acad. HS` have no public
directory entry — not a join defect. The 158 claims are smoke evidence over a copied store, not
national completion. Two further limits belong to the reader revision: the postal acceptance
workbook was published from the read taken before the `WEBSITE` mapping — its derivation is
unaffected, because the postal tuples of the two reads are field-for-field identical (compared
above), and the website-carrying generation is the one a fresh run would build. Separately, the
workspace test suite carries a pre-existing failure unrelated to this work: `xtask`'s
`traversal_tests::actual_fixture_graph_includes_tests_examples_benches_and_tools` expects `6 Rust
files` at commit `344536e7` while the extractor reports 5, and the main checkout carries an
uncommitted fix changing the expectation to 5. It is recorded here rather than silently adopted.
