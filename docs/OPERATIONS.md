# Operations runbook

`restate-server` owns invocation journals/ingress/admin; `census-serve` owns the Fjall store and
endpoint; `census-service` is the CLI. [Lifecycle](deployment-lifecycle.md) owns deployment and
handoff. This runbook owns project handler/retry procedures; [durable execution](restate/durable-execution.md)
is external vendor background. [CLI reference](../crates/census-service/README.md) owns command groups.

Repository developer commands use only `tools/moon-local`; Cargo and shell gate/fault wrappers are
internal implementations. Build with `env -u CI tools/moon-local run pipeline:build-portable`,
then use the real binaries under `target/moon-portable/x86_64-unknown-linux-gnu/release/`.
Runtime ingress/admin commands remain actual census/Restate operations.

## Run selection and inspection

For a fresh census, bind a new named store to an unused durable namespace/revision under
[ADR-013](adr/ADR-013-fresh-national-source-census.md). Preserved historical stores, receipts and seals
do not certify it. A new directory with old Restate keys can replay old work; a changed revision must
not be used merely to reset an exhausted attempt budget.

The default store root is `var/census-service`; historical campaign evidence includes
`var/midwest-census`. Do not assume either is the fresh run. Choose and record the actual root,
namespace, scope, season and revision before acquisition. Never expose private admissions data.

```sh
env -u CI tools/moon-local run pipeline:xtask -- census-status
env -u CI tools/moon-local run pipeline:xtask -- coverage
census-service open-work
census-service national-report --help
```

Serving commands default to loopback ingress `http://127.0.0.1:18095/` and open no second store.
The ingress client rewrites the SDK's synchronous `/restate/call/...` and `/restate/invoke/...`
routes onto the served `/<service>[/<key>]/<handler>` path, and the SDK's asynchronous
`/restate/send/...` route onto the served `/<service>[/<key>]/<handler>/send` suffix, so `national`
and `jurisdiction` submit through the CLI. The SDK's invocation-handle routes have no ingress
equivalent on this server version, so a submission that returns a handle can be submitted with
`--detach` and inspected through `Census/open_work`, `JurisdictionCensus/<key>/state` or the admin
API; the same run can also be submitted on the workflow route itself:

```sh
curl -X POST http://127.0.0.1:18095/NationalCensus/national:<year>:<plan>:<revision>/run \
  -H 'content-type: application/json' -d '{"source_parallelism":4}'
```
Explicit `--store <dir>` selects an offline route where supported and requires the endpoint owner
stopped. Service-only commands reject it. The endpoint itself is loopback HTTP/2, conventionally
9080; the Restate admin API is conventionally 19095. Do not expose an unauthenticated endpoint.

Inspecting a run's server-side state needs the admin API's JSON accept header; without it the query
returns a binary body. With the admin API at `http://127.0.0.1:19095/`:

```sh
curl -s http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -H 'accept: application/json' \
  -d '{"query":"SELECT status, COUNT(*) as n FROM sys_invocation GROUP BY status"}'
```

`open-work` is the owed-work view: `jurisdiction sweeps owed` counts sweeps a run still owes, and
`source objects owed` reads `unmeasured` when the run cannot enumerate them. A client-side
observation timeout is not a job failure: `national` observes for `--timeout-seconds` (default
172800) and the run continues either way, so a submission that printed `Terminal error [500]: the
invocation stream was closed after the 'abort timeout' (1h) fired` may have completed server-side —
confirm through the admin API before concluding a jurisdiction failed. A submission whose run
identity already exists is deduplicated, not restarted: reattaching returns the existing invocation,
and starting a different run needs a revision bump, which remains the documented way to invalidate
completed work and must not be used merely to reset an exhausted attempt budget.

Roster progress is not roster acquisition. Every indexed team is in exactly one roster bucket and
`committed + skipped + remaining == rosters_total`. `committed` counts teams whose captured roster
page admitted every published row; `skipped` counts teams whose row was attempted and retained with
an explicit quarantine reason or rejected row locators (a coverage gap that a re-fetch cannot change,
because row admission is a deterministic function of the retained capture); `remaining` counts teams
with no journaled attempt. A recorded roster stage remains owed while `rosters_remaining` or
`blocked_skipped` is positive, or the stage is blocked. The exposed `owed_rosters` is the greater of
remaining and blocked-skipped counts, not their sum: blocked skips are normally included in remaining
work. Previously journalled/skipped rosters are not subtracted again from remaining work, and their
retained reasons stay in `errors`. An absent progress record leaves the stage owed without inventing a
roster count. These accounting rules also feed census sealing; they do not reset retry budgets,
restart terminal invocations or prove the supplied source-object list is complete.

`teams`, `meets` and `collect` without `--states`/`--all-states` default to Wisconsin qualification.
`--all-states` selects the 49-jurisdiction `CENSUS_SCOPE`, not all 51 modeled locations. Provider
restriction defaults differ; consult that command's help. `collect --school-year` is the academic
starting year, not the graduation cohort: 2026 denotes 2026–2027. Workbook/bests use `--grad-year`.

## Acquisition and incremental work

Use the serving jurisdiction/national path for durable acquisition. Registry applicability, planned
source units and actually wired stage handlers are different things; inspect refused/unfinished
units rather than infer coverage from a registered name. A roster or meet page is shared evidence,
not per-athlete work. Respect source policy and shared physical admission, not a per-workflow budget.

The national workflow runs school-site contact acquisition for every requested jurisdiction after
the school-address join has attached source-backed official websites, and before consolidation.
Its durable `TeamsSource.contacts` key is `<jurisdiction identity>/contacts/sidearm_staff`, distinct
from the earlier team-discovery phase. `open-work` includes this obligation even where the registry
has no singleton SIDEARM seed. Old completed team receipts cannot complete this contact phase.
Only source-owned, qualified staff rows bind coach appointments; unsupported pages, missing websites,
ambiguous owners/categories and access stops remain owed. A published listing binds currentness to
the run school year under ADR-024; it does not independently establish freshness or corroboration.
School office/athletics mailboxes remain separate school-owned claims.

Inspect the settled phase with `national-report --contacts` using the original season, revision and
jurisdiction set. Its `source_rows` measures adapter rows, not distinct schools or nationwide recall;
unknown acquisition counts are null. An absent report means the contact fan-out is not yet measured,
not successful-empty. Use `open-work` for individual active/unfinished obligations.

Public coach CSV import remains a separate operator-controlled surface, not population discovery.
Weekly staging units are operational legacy, not a second canonical census or proof of a fresh
source-to-output path. Do not run old offline collection chains against the serving store or copy
their output into a new run as its population.

An index rebuild and review application can change review-case populations. Retained verdicts and
unresolved candidates must reconcile; never choose a smaller intermediate case table to obtain a
seal. Reusing cached bytes does not advance the actual acquisition timestamp or establish freshness.

MileSplit result projection uses `milesplit_result_sets_v5`, with completion identities bound to
owned/raw capture provenance and projected context. Historical phases remain retained, not
completion authority for changed captures. Reapplication can commit new evidence without
duplicating identical content-bound row effects. Inspect the immutable capture archive and receipt,
not a bare meet/result-set key, when distinguishing a resumed projection from changed source bytes.
Owned acquisition manifests use `acquisition/<meet>/<stable-capture-sha256>/manifest` within
`milesplit_owned_capture_v1`. The digest binds observed requested/final URLs, method, status,
content digest, byte count, actual `fetched_at` and content type, excluding transient `from_cache`.
The manifest references the preserved `<meet>/<body-sha256>/manifest` and chunk prefix.
Interpretations use `parsed|partial/acquisition/<meet>/<stable-capture-sha256>`; rows and
rejections retain content-bound payload identities. Existing captures and interpretations are
not overwritten. Exact receipt lookups replace whole historical-phase key scans on replay.
Result URL order alone cannot create additional physical entity effects; application witnesses
and persisted rows share the same typed set normalization.

An explicit index rebuild rederives mutable projections even when its input receipt already exists.
The receipt prevents duplicate durable input commitment, not repair of missing derived rows.
Unsupported graduation evidence remains a pending `Unsupported graduation inference` review family;
it appears in the workbook's global pending review queue even without a canonical athlete. It is
not a recruiting/cohort admission and does not prove provider identities equivalent.

Coach-directory collection uses the `coach_directories_schools_v2` receipt phase for corrected
eligibility ordering. Old receipts are preserved, but do not skip the corrected parser. A failed
summary retains valid directory facts without a completion receipt; a successful summary commits
school facts, public coach rows and its receipt through the same row sink. Recollecting cached
bytes changes neither acquisition freshness nor immutable Restate run identity/attempt budgets.
School, coach and postal evidence retains the actual capture `fetched_at`; the adapter observation
instant remains a separate execution input. CSV/XLSX postal evidence dates must agree with stored
claims, not with a later replay date.

NET whole-meet collection preserves the original meet-response and result-response capture
instants separately. Meet provenance uses the former; result-derived participant/performance
provenance uses the latter. Published meet dates and season evidence still determine the school
year and grade/cohort context; a later replay or decision date replaces neither acquisition clock.

WIAA result interpretation revision 8 retains published participants from year-only artifacts as
raw `SourceObservations`, including source-owned row/relay-member keys, literal names/schools,
published grades and archive-supported school years. Unknown exact performance dates remain
explicit unfinished debt: no date, dated canonical event or performance is manufactured.
Compatible observations of the same canonical meet use the shared lossless entity merge so
independent result-file URLs and their acquisition evidence survive. Replay preserves physical
observations and acknowledgement receipts rather than relying on later canonical deduplication.

Request pacing has two independent dials, and both default to the polite setting:

- Per-source spacing. Each host (or, for a single-lane source family, the family as a whole) leaves
  its configured delay between turns. `--delay-ms` overrides the default host delay; a source's own
  robots `crawl-delay` raises it, an authorized host never drops below 500 ms, and a family budget in
  `default_family_delays` holds across that family's hosts. Raising concurrency never shortens this.
- `--source-parallelism <N>` accepts 1–64. Above the default of 1, a source family admits N requests
  in flight and each host retains its own spacing slot. At 1 the family keeps one slot and one turn.
  Hosts outside a registered family retain their own slot. This raises a family's ceiling, not its
  rate; it never overrides robots or an access condition. A serving owner fixes the normalized
  budget on its first fetcher request. A different budget for later acquisition is a terminal policy
  refusal, not another independent family budget. Settled logical operations remain replayable
  without resetting their acquisition authority.

Every fetching process also serializes per origin across processes. Before its first request to a
host, a run takes an advisory `flock` on `<workspace>/var/locks/<origin>.lock` — the origin is
scheme, host and any explicit port, sanitized to a file name by
`census_crawl::net::origin_lock_file_name` — and holds it for
the run's life; a second census-service process (CLI command or serving owner) that reaches a held
origin is refused before that request with `origin <origin> is held by another census-service process
(holder: {...})`, naming the holder's recorded pid, command and start time, and exits nonzero rather
than doubling the origin's request rate. Cached responses need no lock, and one process re-entering
an origin it already holds proceeds. A held origin is a refusal, not a queue: wait or stop the
holder.

The native origin-budget qualification uses
`env -u CI SCRATCH_STORE=<new-root> ADMIN_PORT=<unused-port> SERVICE_PORT=<unused-port> tools/moon-local run pipeline:durability -- scenario-08-global-budget`.
Use a current portable build. It executes three independent native CLI processes with separate
stores, one common working directory/production lock root and a reached physical target handshake.
Its fresh certificate binds binary, regression, ledger and capture hashes; it verifies contender
refusals with the exact holder PID, no contender traffic, actual200 pacing/peak/burst, lock release
and traffic-free exact cache replay. This one-host CLI qualification does not certify distributed
locks, multiple serving endpoints, turnover pacing or sustained-load behavior. Keep each run's
certificate and ledger; a historical passing certificate is not evidence for a new build.

Aggregate throughput also follows the endpoint's `--max-concurrent` and per-request `--concurrency`.
TeamsSource acquisition shares the endpoint's Jobs admission budget and serializes active work for
the same source key. All jurisdiction fetchers share one serving-owner pacing state, including
fetchers with different authorized-host sets; a new cache key cannot create another family budget.
Count actual physical admissions separately from cache hits and retained entity populations.
In the acquisition layer `FetchStats.requests` counts logical attempts, including attempts served
from the cache, `FetchStats::physical_requests()` is that count minus the cache hits, and every
`AdapterReport.requests` projects the physical count with its cache hits reported separately as
`from_cache`. A cache-served acquisition therefore reports zero requests, and per-fetch counters
increment `from_cache` alone when the cache answered.

### Sweep watchdog and recovery

Every census handler declares an inactivity timeout plus an abort timeout
(`crates/census-service/src/restate_services/limits.rs`). When both expire with no journal progress
Restate aborts the invocation, and it completes as a failure — it is never retried, so a sweep that
stalls silently is permanently incomplete and its jurisdiction keeps owing the unfinished stages.
Both are 12 h, i.e. 24 h of tolerated silence: a multi-hour single invocation is normal for a
national run (2026-10-07: sweeps that completed ran 9.03–13.27 h; 42 states whose stream went silent
were killed under the previous 1 h + 1 h watchdog).

A failed sweep is re-driven on its own key; never mint a job because an attempt failed.
`JurisdictionCensus` is a virtual object, so re-invoking `JurisdictionCensus/<key>/run` resumes from
the stages its durable state records. `NationalCensus` is a workflow (run-once per identity): once
the cause is fixed, restart its failed run through the admin API
(`PATCH /invocations/<id>/restart-as-new`), or submit a new revision. Read the terminal account
first:

```sh
curl -sS -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -H 'accept: application/json' \
  -d '{"query":"SELECT target, status, completion_failure FROM sys_invocation WHERE completion_failure IS NOT NULL"}'
```

## School athletics site crawl (long tail)

`census-service school-sites <queue.jsonl>` crawls the school websites that the directories and
association lanes do not cover. The queue is JSONL with `state`, `name` and `website` per school —
the NCES-recovered website queues under `var/school-site-wave4-*` and the school-address corpus are
its inputs — and `state` accepts a code or a full jurisdiction name.

The verb opens the store for its HTTP cache only (no Fjall writes) and writes four artifacts under
`<store>/out/school-sites`: one `<STATE>__<school-slug>.json` per site holding the raw signals plus
an evidence record (`url`, content digest, `fetched_at`, status) for every fetched page, one
`site-rows/<site-key>.csv` holding that site's contact-lane rows, one `fragments/<STATE>.csv` in the
twelve-column contact lane shape, and `report.json`. A site is resumed only when its artifact *and*
its recovered row file exist; an artifact without its rows is crawled again rather than skipped,
because a crash between the two writes would otherwise lose the site from the projection forever.
`--refresh` refetches and rewrites both. `fragments/<STATE>.csv` is republished as the sorted union
of every per-site row file under `site-rows/`, so resuming a queue — including one that gained a
school in a state an earlier run already covered — never drops or replaces rows captured earlier.
`--sample` and `--limit` narrow the run, and `--state` supplies a jurisdiction for queue records
that carry none.

School sites redirect across origins constantly (`http` to `https`, apex to `www`, a school host to
its district or platform host), so a run that intends to follow them passes
`--authorize-queue-hosts`: it authorizes the base domain of every host in the queue for this
collection, exactly as `verify-coaches --authorize-cited-hosts` does for cited hosts. Without the
flag a cross-origin redirect is refused and the site is reported under `failed` with its fetch
error. `--delay-ms`, robots and the per-host origin locks still bound every request.

The crawl is a static read that executes no JavaScript. A staff page that builds its directory
client-side therefore yields no coach rows, and the gate labels any candidate row from such a page
`render-required` instead of shipping it.

Fragments are candidates, not contacts: `verified_proof_digest` stays empty until `verify-coaches`
re-fetches each row's `source_url` and writes the proof. Nothing the crawl writes is accepted into
the census on its own. A coach name is only verified on a span that carries its program, its school
name and its state; an athletic-director name is verified under the same institution and
jurisdiction binding, so a director certified for one school cannot be proven from a page naming
another school or state. A span that marks the appointment `former`, `not current` or `no longer`
— for a coach or a director — contributes no claim at all and marks the row contradicted, which
withholds it from the union. `--union <dir>` stages the verified rows as one `<ST>.csv` per state
beside its `<ST>.csv.evidence.jsonl` claim sidecar; `merge-coaches --fragments <dir>` consumes exactly that
staging shape and publishes `<out>` with `<out>.evidence.jsonl` for the rows it kept, carrying their
claims verbatim (the proof digest covers the claim sequence, so a repeated claim is preserved rather
than deduplicated). Every kept row is validated before publication against the claims that bind to it
and against its recorded digest, so an absent, empty, truncated or mismatched proof — and any edited
school, role, sport, name, mailbox, source URL, observation date or digest cell — rejects that row;
the report names each rejection and its reason. A fragment whose sidecar is missing is refused
rather than merged, and
`verify-coaches --reconcile <csv>` recomputes each published row's proof from that sidecar, so a
merged product is only accepted when
its digests still match the verified claims.

```text
target/debug/census-service --store var/<run> school-sites var/school-site-wave4-*/TN.jsonl \
  --authorize-queue-hosts --out var/<run>/out/school-sites
target/debug/census-service verify-coaches --fragments var/<run>/out/school-sites/fragments \
  --cache-dir var/<run>/http --authorize-cited-hosts --out var/<run>/verify --union var/<run>/union
target/debug/census-service merge-coaches --fragments var/<run>/union \
  --out var/<run>/coach-contacts.csv --report var/<run>/merge.md
target/debug/census-service verify-coaches --fragments var/<run>/union \
  --cache-dir var/<run>/http --authorize-cited-hosts --out var/<run>/reconcile \
  --reconcile var/<run>/coach-contacts.csv
```

## Browser lane

Start the endpoint with a dedicated `--browser-profile <dir>` and an allowed browser executable.
Use an absolute path: the lane's settings validation rejects a relative profile directory or
executable, and binaries built before this tree report that rejection as an invalid tab count.
Use the headed profile required for Athletic.net; a headless flag's existence is not an authorization
to bypass that source policy. Then use ingress:

```sh
census-service browser-session start
census-service browser-session status
census-service browser-session stop
census-service browser-session fetch --url <url> --semantic-url <citation>
```

The endpoint owns the profile and default object key `profile-0`. A missing lane is an explicit
source refusal, not an empty page. `HumanRequired` stops affected admission until the operator
resolves access in that profile. Do not rotate egress, spoof identity, replay challenge cookies or
switch transport to clear it. Other permitted sources may continue.

Lane presence is part of the current plan fingerprint. Adding/removing it under a previously
recorded plan can be refused; preserve the existing run's obligations and make any capability/run
transition explicit. Do not silently resubmit every failed operation under new keys.
[Architecture §6](../ARCHITECTURE.md#6-admission-and-browser-state-10-26-28) owns admission requirements.

## Shutdown and diagnostics

SIGTERM first stops the endpoint listener and waits for SDK connection shutdown, then closes region
admission, drains owned work and flushes storage. `--drain-timeout` is the async-task grace period,
not a wall-clock bound on shutdown. The shipped unit uses `TimeoutStopSec=infinity` and
`SendSIGKILL=no`: already-started blocking effects must finish before the store can be finalized.
Retain the certificate:

```text
drained: accepted=<n> completed=<n> cancelled=<n> timed_out=<n> aborted=<n> panicked=<n>
```

A stop request is broadcast to cooperative region tasks before the drain begins. A non-zero
`timed_out` counts tasks still owned at the grace deadline; it overlaps terminal outcomes and is not
an additional completion bucket. Async survivors are aborted and reaped; started blocking effects
are awaited even beyond the deadline. Successful drain reports satisfy
`accepted = completed + cancelled + aborted + panicked`, with no remaining owned tasks.
Cancelled drain callers retain the region for a subsequent drain; admission never reopens.

The endpoint's own shutdown is the SDK's: `restate-sdk` waits up to ten seconds for open connections
to close before region admission closes. This ordering prevents shutdown refusal from becoming a
terminal result of an invocation's unstarted effect. Reconcile persisted unfinished work; a
deadline, abort or panic is not normal success to hide. A blocking effect that never returns prevents
clean shutdown; diagnose it rather than killing it and claiming a drain certificate. Confirm process
exit/store lock release before another owner starts. `RUST_LOG` controls structured diagnostics.

Use Restate admin queries for invocation state without opening Fjall:

```sh
curl -sS -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -H 'accept: application/json' -d '{"query":"SELECT id, status, target FROM sys_invocation"}'
curl -sS -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
  -H 'accept: application/json' -d '{"query":"SELECT service_key FROM state WHERE service_name = '\''Ingest'\'' ORDER BY service_key"}'
```

Paused invocations can retain object ownership and block later submissions. Identify the actual
failure, deployment and durable key before operator cancellation; do not mass-kill or erase journals.
`JurisdictionCensus` currently kills on retry exhaustion, while other definitions may pause.

## Offline independent review

With the store owner stopped, `review` requires exactly two independently bound loopback model
endpoints before deterministic reconciliation may write anything:

```sh
census-service --store <restored-store> review \
  --endpoint http://127.0.0.1:11000 --endpoint http://127.0.0.1:11001 \
  --model <served-model-name> \
  --response-format prompt-json --response-format json-schema --family athlete-identity
```

One explicit model name applies to both endpoints; supply one per endpoint when names differ.
`--response-format` also accepts one value for both endpoints or one per endpoint, in order.
Defaults target `qwen3.8-27b-uncensored`, NInfer port 11000 with `prompt-json`, and llama.cpp port
11001 with `json-schema`. Prompt JSON requests OpenAI text responses and still requires an exact
JSON verdict object; prose, fences, unknown fields, missing/null string fields and confidence outside
integer 0–100 refuse. There is no automatic protocol fallback. Listing a model is not qualification.
`--dry-run` asks and validates without committing advice. Response format participates in exact
per-lane bindings: a changed lane is reasked while independently valid unchanged peer advice can
be reused. Changing configuration does not automatically reopen resolved/superseded history.
Each case retains both endpoint/model/request bindings, structured answers or failure
classifications, adjudications and the evidence digest. Both lanes must agree on an admissible
value; unknown JSON fields, malformed/duplicate/ghost verdicts, unsupported values, incomplete
membership, missing positive identity evidence and SamePerson contradictions cannot accept.
DifferentPerson remains admissible when Rust's existing identity rules permit it. A case filed with
an explicit member pair is asked on that filed pair: both member rows are loaded and compared even
when their canonical keys disagree, so a pair whose two rows differ in school still reaches both
models. A case no packet can be built for — no filed pair of two distinct rows, or a member row the
store does not hold — counts toward `requested`, issues no request, holds no advice and is reported
as `unaskable`, never as `unanswered`.

Unresolved retries reuse only independently valid advice with exact current bindings, including
primary ownership, linked identities, profile/evidence/grade-source URLs and retained conflicts.
A failed lane does not discard the other's valid answer. Verdicts, case transitions and receipts
commit together at checkpoints of at most 256 cases. Selected checkpoint input, retained advice,
current evidence and output consume a conservative cumulative 8 MiB serialized-byte budget;
overflow refuses before that checkpoint is published, without truncation or smaller silent batches.
Zero model-case budget validates the lanes before reading census/advice. This is not a measured
process-RSS bound: store iteration can transiently decode a row that is not selected.

Current subjects and cases share one post-advice snapshot; its sequence fences the commit.
Cancellation cannot acknowledge an incomplete checkpoint. Repeating a receipted payload verifies
its durable verdict and case state. Explicitly reopening the same case at the same `observed_at`
refuses if its old resolved receipt is no longer applied; it does not silently close Pending or
mint a new retry identity. This applies to model-assisted and deterministic reconciliation.
Resolved/superseded history is not automatically reopened, and legacy single-lane records cannot
establish independent advice for new review. Equal internally contradictory cohort sets still
refuse SamePerson; admissible DifferentPerson advice remains distinct. This offline command is not
yet the national workflow's durable review stage. Failed raw HTTP/content bytes are not retained
by the current model-client error contract.

## Preserved HTTP captures and incomplete acquisition

HTTP cache publication preserves captured bodies and metadata immutably before replacing eligible
mutable successes. Real non-success bodies reaching the body reader are archived without becoming
reusable success-cache entries. Early refusals with no obtained body remain explicitly uncaptured.
The current optional String representation refuses unrepresentable response headers rather than
inventing their values. Bodies are bounded at 32 MiB and encoded capture metadata at 64 KiB.
304 reuse verifies the planned metadata and bounded body under the same publication lock; it cannot
overwrite a newer refresh. Contending downloads retain their own archive before lock refusal.
Quarantine and failed staging directories are retained and synchronized; preserve them for operator
inspection rather than deleting a potentially unique capture after an IO/publication failure.

Arbiter incomplete-response recovery uses `arbiter_coaches_incomplete_v2:<state>:<org>:<public-id>`
with version-2 owner-bound payloads. An unchanged incomplete cached response triggers an ordinary
admitted refresh; partial valid facts survive and only a complete owned acquisition receives its
completion receipt. Historical v1 markers are preserved, not adopted as another owner's authority.
Native teams acquisition uses mandatory TeamsSource virtual objects, keyed by the parent
`WorkflowIdentity::jurisdiction(jurisdiction, season, revision)` plus `/teams/<source>`. A failed
attempt never receives a fresh source key. Each object retains immutable identity, reservation and
outcome slots in `teams_source_attempts_v1`, with at most three reservations. Before admission,
an SDK run journals the real immutable Fjall identity registration. Registration commits with
`SyncData` and completes the store flush before returning; it does not reserve a physical attempt.
Local storage errors use the SDK's resumable run retry policy, not a one-attempt terminal override.
Admission compares the cached SDK identity against authoritative Fjall identity; missing, corrupt
or contradictory authority is refused rather than recreated from the cached result.
The endpoint retains its source-key admission guard in the supervised blocking registration
worker as well as the caller. Cancelling the caller cannot reopen that key while queued or running
registration still owns the guard. A subsequent registration must retain the first committed
identity and observation date; process-local admission is not proof of a multi-endpoint budget.
No SDK run encloses the physical source operation; the child handler owns its bounded physical
retries. Completed, terminal, three known transient failures and unacknowledged acquisition are
distinct outcomes.
An abandoned third reservation is Interrupted, not three proven failures and not permission for a
fourth attempt. The parent retains structured SourceFailures and continues independent stages.

TeamsSource `state` inspects settlement; `progress` inspects the coherent reservation/outcome
history, including Unknown reservations. An active third reservation is not itself a settlement.
`inspection` is the atomic parent-recovery surface: either Settled with its supported outcome and
history, or Unsettled with the coherent history. If the native child call fails, the parent prefers
that authoritative settlement; otherwise it retains the actual reservations/reports as Interrupted.
An unavailable inspection is explicit, not a fabricated empty known-failure history. Do not combine
independent `state` and `progress` calls into a supposed atomic recovery snapshot.
Attempt progress preserves actual adapter reports and failure reasons separately. Reported row
counts are adapter processing counts for that attempt, not cumulative or unique school/contact
populations; missing reports remain unknown. Never sum overlapping retry reports to certify census
coverage. Unique retained populations and relationships require entity/source receipt reconciliation.
These mechanisms do not authorize robots, access-refusal or challenge bypasses.

MileSplit whole-meet evidence uses `milesplit_owned_capture_v1` for original byte chunks and
`milesplit_owned_meet_v3` for owner-bound interpretation with qualified hurdle labels and combined
scores. The v2 interpretation remains historical evidence; the current parser records a new
interpretation without changing original capture bytes. TeamRelay rows are explicitly retained
as recognized team observations rather than treated as malformed individual owners.
`milesplit_result_sets_v5` is the separate canonical projection receipt, exported as
`RESULT_SET_PHASE`: it requires exact provider school binding, published cohort and matching
raw-document meet/season metadata, and binds owned/raw capture provenance and projected context.
It retains direct published graduation years as typed claims rather than invented grades.
Historical v2/v3/v4 projection receipts remain preserved and do not suppress changed v5 captures.
The row sink commits projected entities, observations and the content-bound v5 receipt together.
A completed receipt suppresses duplicate physical
effects while replay rebuilds the current accumulator; it is not a meet-exhaustion, accepted
identity, lifetime-PR or census seal certificate. Inspect partial receipts and unresolved bindings
rather than replacing missing owners with name matches.
An empty canonical-school input does not block source-owned retention. Unresolved provider rows
and their partial obligations survive until a genuine exact school binding permits projection;
no placeholder school, imported population or name-only binding is introduced.
New partial projections pair physical contents with `milesplit_result_set_effects_v1` application
witnesses in the same row-sink commit. Witness lookup is indexed by table/canonical-content digest;
the collector does not load an ever-growing witness set or emit one recording batch per row.
An unchanged partial replay stays partial, produces no duplicate physical rows and does not rewrite
identical retained/projection receipt payloads. A new exact provider school binding still permits
the unresolved projection to proceed.
Result-set requests are grouped by their shared meet: one meet is acquired, parsed and committed as
its own canonical/effect/receipt batch, then released before the next meet is read. A later meet's
interruption therefore leaves an already completed meet's projection durable and resumable, while
the interrupted meet leaves no half-visible rows; re-running the same selection resumes from the
retained captures, journals and receipts instead of re-projecting them.
This interpretation revision does not migrate completed historical canonical projections. A fresh
census uses a fresh store; existing projected stores still require an explicit source-bound
correction/migration before they can claim the new parser semantics. Do not append newly minted
event IDs beside stale typed canonical rows or present a historical receipt as current acceptance.
Older partial stores without these witnesses are preserved historical evidence, not retroactively
certified idempotent.

## School-address corpus and join

The school-directory corpus is one verified generation over operator-supplied directory artifacts;
the join turns that corpus into owned postal claims on canonical schools. The generation has no
store and no clock: `--now YYYY-MM` is the caller's month, and the lane digest it records is the
capture digest every claim will carry. Nothing here invents a capture URL or an observation date —
without both, a matching school stays unlinked and is reported `evidence_missing`.

Build the corpus from the real captures (the CCD reader documents its own fetch because a `.zip`
passed straight to the verb is refused):

```sh
curl -o var/<run>/ccd_sch_029_2526_w_0a_050626.zip \
  https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip
mkdir -p var/<run>/ccd && unzip -o var/<run>/ccd_sch_029_2526_w_0a_050626.zip -d var/<run>/ccd
target/moon-portable/x86_64-unknown-linux-gnu/release/census-service school-address \
  --ccd var/<run>/ccd/ccd_sch_029_2526_w_0a_050626.csv \
  --pss /home/lewis/src/ad-law-scrape/data/nces/pss/pss2324_pu.csv \
  --out var/<run>/corpus --now 2026-10
```

(The PSS public-use CSV is the local capture of the verified
`https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip` archive; the verb reads the CSV, not the
archive.)

The national CCD/PSS CSV readers admit at most 128 MiB of input, 200,000 source rows and
200,000 retained rows per vector, with a shared 512 MiB retained-byte accounting limit.
These national-file limits do not enlarge the ordinary directory-page limits. Any decoder stop
or unfinished frontier refuses generation publication; a parsed prefix is not a complete input.

CCD school jurisdiction prefers the source's physical `LSTATE` to its administrative `ST`;
`ST` and then `MSTATE` remain fallbacks when the location state is absent. Agency code `BI`
is not a geographic exclusion when the same row publishes an in-scope physical state.
NCES school keys are unchanged. Address selection still retains one owned physical or mailing
tuple and its explicit kind; jurisdiction selection does not splice address components.

The 2026-10-08 code-only replay of these retained files produced 122,518 all-grade school records
across the 49 approved jurisdictions: 100,133 CCD physical-address records and 22,385 PSS
mailing-address records. Seventeen PSS ZIP codes remain unknown, not reconstructed.
There are 70,056 website claims; ID, IN, KY, LA, NE, RI and SC have no website links in this input.
This is an observed NCES frame, not a measured nationwide school/program denominator or a coach
mailbox census. The files' source-year lag and missing fields remain material limitations.

Historical 2026-10-04 execution produced 122,692 entries (CCD 100,307, PSS 22,385) over 122,692
rows with 1,920 skipped rows and 386 notes, a `current/` publication with `manifest.json`,
`school_directory.json`, `school_directory.csv`, `pipeline_report.json`, `baseline.json` and
`update_ledger.json`, and lane digests `nces-ccd=d1473136…`, `nces-pss=14a2f9e6…`.

Historical re-reading of those captures with the CCD `WEBSITE` column mapped changed nothing about the
entries — same 122 692 over 122 692 rows, same 1 920 skipped rows — and adds only website notes:
the CCD lane goes from 369 to 398 notes because 29 `WEBSITE` cells are not http(s) URLs (for
example `website is not a supported value: "http://601 South Clinton Street"`), and the exported
`website` column then carries 70 223 URLs. The two reads' postal tuples (key, street, city, state,
ZIP, phone) are field-for-field equal; a generation digest differs (`57edd0b2…` without the
mapping, `5578b3f3…` with it), so a claim can never be mistaken for one built on the other.

Join it offline against a stopped store owner (dry run first; `--apply` appends):

```sh
target/moon-portable/x86_64-unknown-linux-gnu/release/census-service --store var/<store> school-address-join \
  --generation var/<run>/corpus \
  --evidence-url nces-ccd=https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip \
  --evidence-url nces-pss=https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip \
  --evidence-date nces-ccd=YYYY-MM-DD --evidence-date nces-pss=YYYY-MM-DD
```

A national run validates the school-directory generation and every lane's URL/acquisition-date
attribution through `Census.school_address_preflight` before starting jurisdiction fan-out.
Missing `current/`, malformed artifacts, digest mismatches and unattributed lanes are refused early.
The verified manifest digest is journalled into the address-join request; a different generation is
refused before applying any join effects. A request without `--school-directory` resolves
`<data-dir>/school-address`. Build that input with the same binary that will serve the run, then
submit, or pass `--school-directory <root>` explicitly. The historical 2026-10-06 run lost about
13 hours because its missing input was checked only after sweeping; that is not the current order.

```sh
target/moon-portable/x86_64-unknown-linux-gnu/release/census-service school-address \
  --ccd <run>/ccd/ccd_sch_029_2526_w_0a_050626.csv \
  --pss /home/lewis/src/ad-law-scrape/data/nces/pss/pss2324_pu.csv \
  --out <data-dir>/school-address --now 2026-10
```

Check the exact input without opening a store or starting acquisition:

```sh
target/moon-portable/x86_64-unknown-linux-gnu/release/census-service school-address-preflight \
  --generation <data-dir>/school-address \
  --evidence-url nces-ccd=https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip \
  --evidence-url nces-pss=https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip \
  --evidence-date nces-ccd=YYYY-MM-DD --evidence-date nces-pss=YYYY-MM-DD
```

Use the actual retained capture dates, not the replay/build date. Passing preflight verifies these
declared input bytes and attribution, not school-population recall, contact availability or freshness.

A provider read from more than one capture needs per-capture attribution, and every generation
records it: each lane in `pipeline_report.json` carries the source keys (`captured`) that its own
capture holds, and a claim is credited only to the capture holding the matched key. Give each of a
provider's captures its own URL with `--evidence-url <provider>@<capture path>=<URL>` (the lane's
path as recorded in the report; `<provider>` alone still applies one URL to every capture of that
provider, and `--evidence-date` stays provider-wide). A selector naming no capture in the
generation is refused before anything is written. A school whose key several captures hold is
`refused` — the row names every candidate path, and the remedy is one capture per school — and a
school no capture holds is `evidence_missing`; neither case names a capture that does not carry the
school, so a generation published before `captured` existed refuses a multi-capture provider
instead of guessing, and rebuilding it from the same captures restores the links. The keys add
6.8 MB to the 122 692-entry report above (0.3 MB → 6.8 MB; the corpus itself is 83.8 MB), because
per-capture keys, not whole entries, are what naming a claim's bytes costs.

The durable apply lane remains the `SchoolAddressJoin` workflow (Restate service `SchoolAddressJoin`,
key `<run identity>:school-address-join`). National execution orders preflight, jurisdiction census,
digest-fenced address join, school-site contacts and consolidation. Workbook publication remains a
separate existing publication surface; a national report alone is not an exported or sealed workbook.
The workflow carries `--evidence-*` equivalents and the preflight digest in its request; invoke it
directly through the ingress only when repeating a failed stage:

```sh
curl -s -X POST "http://127.0.0.1:<ingress>/SchoolAddressJoin/<key>/run" \
  -H 'content-type: application/json' \
  -d '{"generation":"var/<run>/corpus","urls":{"nces-ccd":"…","nces-pss":"…"},"dates":{"nces-ccd":"YYYY-MM-DD","nces-pss":"YYYY-MM-DD"}}'
```

Both lanes write `<store>/out/school-address-join/report.json` (mode, generation path, manifest
digest, per-lane evidence, and the counters `scanned`, `linked`, `already_linked`, `backfilled`,
`websites`, `review`, `no_match`, `refused`, `evidence_missing`, `missing_state`, `review_filed`,
`review_present`, `co_op_members`, `co_op_declined` plus the per-rule counters) and
`outcomes.jsonl` (one row per school: outcome, rule or reason, candidate labels). Under `--apply`
an ambiguous tie files one `ReviewCase` under the `School identity` family carrying both candidate
labels and the lane providers (`review_filed`); a
replay that finds it already durable counts `review_present` and appends nothing, and a dry run
files nothing. An identity the school already owns is `already_linked`, and the matched capture is
still applied: an already-owned record that lacks the claim gains it, a claim for a different
address or a different capture appends beside the earlier one, and a record that gains any of those
distinct observations (claim, website or lane evidence) counts `backfilled`. An identical replay of
the same capture appends nothing (`already_linked`, `backfilled` 0), so a replayed apply over the
same generation is safe after a restart, while rebuilding the corpus from the same captures — a
different generation digest — re-presents each school as a new observation and is retained. A matched CCD entry whose website
is an http(s) URL and whose school publishes none attaches it to `CanonicalSchool.school_website`
(counted as `websites`, rendered by the workbook's `School site` column) with the same lane evidence
note; a school that already publishes a website is not rewritten, and an unusable `WEBSITE` cell
stays a corpus note.
Two equally plausible schools are `review`, never a silent pick; a name matching only in another
state is `no_match`. A school marked `co_op` links each published member name (`aliases`)
independently: every member name runs the ladder and guards alone, an accepted member key stamps its
own identity and evidence on the co-op school without creating or merging a member school, a member
tie files its own case, and a member name with no qualifying key counts `co_op_declined` while the
school-level `no_match` keeps its meaning. A trailing parenthetical that is only a campus designation
(`East`, `West Campus`, `Main`) never falls back to the undesignated campus through the parenthetical
head, so two campuses sharing one base name keep their own keys; the CSV's `identity_count` carries
how many keys a school owns and the link block publishes the first in namespace/id order. Read claims
back through the workbook's `Schools` postal block and `School
site` column plus `census-service verify --workbook`; a pending ambiguous join is read back in the
workbook's `Review` sheet under the `School identity` family, whose rows `verify` recomputes from
the store alongside the verdict rows a reviewer has answered. The postal cells carry the owner
namespace/id, source lane, capture URL, observed date and capture SHA, so a corpus rebuild with
different bytes cannot masquerade as the same evidence.

## Export, verification and sealing

Workbook publication captures one bounded, immutable input generation for its logical job.
Workbook, censuses, best-results, recruiting and audit products derive from that input, not
independent live-store reads. Review decisions must be applied before capture. Retrying the same
job reuses its archived input; source advancement refuses that input rather than silently
recapturing it. A changed input requires a new logical export, for example a new publication root.
Returning an existing capture also synchronizes its archive directory, so retry success cannot
skip a previously failed directory-durability step. A fresh publication root, a fresh
`<store>/out/export-inputs` archive and any fresh output directory create every missing ancestor and
fsync each newly created directory's parent before the verb acknowledges success, so a reboot cannot
drop the new directory entry; [FJALL_BACKUP.md](FJALL_BACKUP.md) states the filesystem requirement
for directory fsync.

An explicit offline `index --store <dir> --school-year <yyyy>` still re-derives mutable projections;
those separately replaced tables are not the workbook's publication boundary and are not a
prerequisite for retained-data export. The index verb requires the run's contact school year
explicitly, like `workbook`: the retained conflict and review queues assess coach tenure and contact
claims against that season, so a queue row always reflects the run being indexed and never the
calendar date the index happens to run on.

Retained mark normalization is a separate, explicit offline operation, never a new scrape. Stop the
store owner, cold-back up and restore into a new root under [FJALL_BACKUP.md](FJALL_BACKUP.md), then:

```sh
census-service --store <restored-store> repair-retained-marks
census-service --store <restored-store> repair-retained-marks --apply
census-service --store <restored-store> repair-retained-marks
```

The first command is dry-run. Apply appends at most 100 corrections per write from one bounded
snapshot. Only MileSplit result-row raw times with explicit `a/A` or `h/H` qualifiers and metric
marks with explicit `m/M` units exactly representable as centimetres are eligible. Timing
qualifiers on field events, undeclared/non-numeric tokens, missing event/evidence and contradictory
known timing are not guessed. Metric normalization preserves timing and does not infer an event
identity from the unit. IDs, source keys/owners, result affiliation, capture dates and original
observations remain intact; a derived evidence note carries the original token and fixed correction
revision.
The final dry-run must report zero eligible corrections. Retain the backup as rollback evidence,
restart a sole endpoint owner on the restored root and submit Workbook with a new publication root.
No PR is invented for an athlete/event lacking a retained comparable result.

The same stopped-owner/backup/restored-root sequence applies to event-kind refinement:

```sh
census-service --store <restored-store> repair-retained-events
census-service --store <restored-store> repair-retained-events --apply
census-service --store <restored-store> repair-retained-events
```

Only previously unmapped, recognized source labels with matching parsed provenance qualify.
Disagreeing or unbound labels and retained conflicts refuse refinement, including on the typed
side of a merge. Corrections append in batches of at most 100; event identity, meet, sex,
division, round, original labels and observations survive. The final dry-run reports no eligible
corrections. Unsupported labels remain unresolved rather than becoming guessed PR categories.

Export schema revision remains 1. Policy revision 4 requires event-compatible PR marks, combined-event
scores and same-context conflict detection. It retains revision 3's accepted-alias projection,
source-owned postal fields and complete summary-cell admission, and revision 2's declared timing
normalization and exact omitted-zero imperial-inch notation (`19-.25`). Revision-1/2/3 frozen bundles
remain preserved historical evidence but are refused by the revision-4 verifier. Existing logical
export inputs remain immutable. Publish an explicitly new logical export/root for the changed
projection policy; a completed Restate workbook identity must not replay an old reply as new output.

`--out` names a publication **directory**, not an XLSX file. A writer holds its publication lock,
renders into owned staging, hashes the exact artifact inventory, independently reconciles every
workbook cell and sidecar record, and checks the source fence while switching `current` atomically.
Completed generations live under `generations/<generation-digest>/`; failed staging and temporary
pointers are removed without deleting completed or historical generations. Frozen inputs and
individual artifacts are capped at 8 GiB; the complete artifact inventory is capped at 16 GiB.

```sh
census-service workbook --ingress http://127.0.0.1:18095/ --out <publication-root> --grad-year 2027
census-service verify --workbook <publication-root>/current/workbook.xlsx
census-service seal --ingress http://127.0.0.1:18095/ --workbook <publication-root>/current/workbook.xlsx --write
```

One measured limit: a full-size publication currently cannot seal through the ingress route. The
handler re-materializes the frozen input inside the endpoint, past `census-serve`'s memory budget,
so the endpoint drains itself mid-seal and Restate pauses the invocation — two clean `MemoryBudget`
drains on `var/national-fresh-20261009-01` (2026-10-09), filed as `athletic-rust-pipeline-q41j`.
Until that is repaired, read the verdict from the offline route (`census-service seal --store <run>`
with the store's owner stopped) and record the run's own open work as unknown: only the ingress
route measures it.

Standalone `verify` reopens the immutable bundle's frozen input, verifies lineage, hashes and exact
inventory, and compares every workbook row/cell and JSON/JSONL/CSV sidecar record. It does not open
the serving store, sample, stride or impose the former 5,000-sample ceiling. It can therefore run
beside the store owner. Historical loose workbooks without a complete manifest are not certified;
preserve them and publish a new generation instead of editing old evidence.

A store admits exactly one census run. National submission binds a run manifest — store identity,
season, revision, the Class-of-2027 cohort and the admitted jurisdiction scope — through the
`Census.bind_run` handler before any jurisdiction fans out, and a store already bound to another
run refuses terminally instead of adopting it. The binding lives in the store and survives restart.
The publication's lineage carries that manifest, so a seal measures the same run it certifies: a
journal measured for one season/revision cannot certify a publication of another, and a route that
measured no journal names the run binding as an unmet item instead of defaulting it. The contact
school year must be the run's own. Only the Class-of-2027 cohort seals at all; any other
`--grad-year` is refused at the boundary before a journal or a workbook is read, whether the request
arrives through the CLI or the ingress.

Seal uses the same complete bundle oracle, refuses limited/non-Class-of-2027 or stale-source
publications, and fences the live source before atomically recording `out/seal.json`. Supply the
actual `--season`, `--revision` and all applicable repeated `--source-object <key>` values: defaults
or an incomplete object list are not the run's denominator. Current open-work heuristics remain
narrower than the complete source-obligation contract. Missing offline measurements remain unknown
and must refuse rather than default to zero. Full bundle readback is not national coverage,
fresh-source qualification or all-native-fault certification.

If an old seal shape cannot decode, preserve it and diagnose an explicit version/migration
decision; do not edit away evidence or quietly ignore it. Old seals do not certify changed input.
Acceptance requirements live in [the active plan](NATIONAL-CENSUS-PLAN.md), not exit status alone.

## Backup, deployment and release

Use [cold backup/restore](FJALL_BACKUP.md); preserve referenced raw captures and the separately owned
Restate durable directory. No live directory copy, cache-only backup or removed `import-legacy`
command can substitute. Deployment unit/config ownership is in [lifecycle](deployment-lifecycle.md).

A store records its own schema identity. `census-service --store <root> fjall-stats` prints
`schema_version`, `key_format`, `created_by`, `created_at` and both generation counters:
`evidence_generation` moves when canonical observations, entities or review inputs are written, and
`derived_generation` names the derived generation readers currently see. Opening refuses a store it
cannot interpret: `StoreError::FormatNewer` (written by a newer schema), `SchemaUnknown` (missing or
inconsistent schema rows) and `MigrationRequired` (older, or a migration that did not finish), each
naming the root and the detail rather than reinterpreting bytes.

Bring an older store to the current schema with the store stopped, after a cold backup:

```sh
census-service --store var/<store> store-backup --to var/backups/<name>
census-service --store var/<store> store-migrate
census-service --store var/<store> store-integrity
census-service --store var/<store> fjall-stats
```

`store-migrate` rewrites derived tables under generation partitions, collapses map tables to one row
per id and leaves observation-log bytes unchanged; it writes its resume marker before rewriting, so
an interrupted migration resumes on the next call, and a store already at the current schema is
reported with `already_current` and not rewritten. The command reports `rewritten_rows`,
`dropped_rows` and the migrated store's integrity, and it opens the store itself: the owner must be
stopped, exactly as for cold backup. Derived generations are reclaimed with a budget on each
publication; a repeat index pass reclaims the stage it abandons, and
`Store::reclaim_derived_generations` exposes the same sweep to an offline caller.

[Moon developer tasks](../xtask/README.md) are the only gate entrypoint; `tools/gate.sh` remains their internal gate implementation. [VERIFICATION-EVIDENCE.md](VERIFICATION-EVIDENCE.md) owns dated
incidents and executed results. Report only the declared run/scope and observed verification, with
terminal access gaps, unresolved review and unfinished discovery visible.

## Exact source time precision

Timed marks use `ExactSeconds`, a positive checked `i64` nanosecond value with the source's
fractional-digit count (zero through nine). The current JSON shape is
`{"TimeSeconds":{"nanoseconds":10941000000,"precision":3}}` for `10.941` seconds.
Equivalent spellings compare and hash by nanoseconds; precision remains independent output
provenance. Decimal seconds, `minutes:seconds` and `hours:minutes:seconds` are accumulated with
checked integer arithmetic, not floating-point addition. Clock seconds must be below 60; minutes
in an hours-prefixed clock must be below 60. Nonfinite, nonpositive, malformed, over-precision and
overflowing times are refused rather than rounded into a usable mark.

PR ordering uses nanoseconds for time, micrometres for distance and hundredths of a point for
combined-event scores. `best-results-*.csv` declares these integer scales in `best_value_unit`
(`ns`, `um`, `centipoints`). Its `best_mark` and the XLSX `Calculated PR` preserve source
precision, including trailing zeroes. XLSX `Mark Value` remains a presentation number in seconds,
with `Unit` equal to `s`; it is never the comparison authority. The JSONL mark retains exact
nanoseconds and precision even when a floating-point presentation cannot retain all nine digits.

Bare legacy integers remain centiseconds under their historical schema, never nanoseconds.
Only an explicit versioned migration may convert them with checked multiplication by
10,000,000 and precision two. Legacy floating-point seconds require the named historical
schema's interpretation, with overflow and invalid values refused. Do not relabel an old
frozen export or completed workflow reply as current exact-time data. Publish under the new
schema/policy authority; captured fixtures, historical bundles and other runs remain immutable.

The exact parser accepts borrowed text without creating a component vector. The time display
uses scalar decimal formatting rather than building an intermediate fractional suffix.
Serialized canonical meet/performance records independently own their identifier/date fields;
their ownership-boundary copies are not a claim of free cloning or measured performance.
No latency, throughput, allocation-count or memory improvement is certified by these source changes.

