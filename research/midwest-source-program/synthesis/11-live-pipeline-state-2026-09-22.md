# Live pipeline state — 2026-09-22

Measured, not projected. Every number below comes from a command run today against
`/home/lewis/src/ad-law-scrape/athletic-rust-pipeline`, store `var/census-service`.

## 1. Census result (the acceptance question)

Two scopes were published after the durable index pass (`index`, 177 s):

| scope | canonical athletes | Class of 2027 | boys | girls | athletes with an Athletic.net profile URL | corroborated by a second source |
|---|---|---|---|---|---|---|
| `all_sources` | 879,587 | **213,198** | 119,144 | 93,688 | 199,996 | 59,269 |
| `core` (Athletic.net + its AthleticLIVE derivative excluded) | 743,739 | **169,969** | 94,285 | 75,405 | 166,027 | 0 (by construction) |

Read the pair, not either alone:

- **79.7 % of the Class-of-2027 cohort (169,969 of 213,198) is discoverable without Athletic.net.**
  The remaining ~43,229 athletes are discovered or enriched only by Athletic.net itself.
- 93.7 % of the Class-of-2027 athletes already carry a public Athletic.net profile URL
  (199,996 of 213,198) — the census knows *where* to spend Athletic.net requests rather than
  searching for them.
- 27.8 % (59,269) already have independent corroboration from a non-Athletic.net source.
- 13,483 schools and 27,580 coaches are attached to the same graph.

Commands: `census-service report` and `census-service report --core`
(write `out/report.json`, `out/report-core.json`, `out/census-by-state.csv`,
`out/census-by-state-core.csv`).

## 2. Store inventory

`census-service fjall-stats`, before → after the index pass:

| table | before | after |
|---|---|---|
| schools | 62,063 | 62,063 |
| teams | 138,995 | 138,995 |
| coaches | 32,264 | 32,264 |
| athletes | 1,565,219 | 1,565,219 |
| meets | 12,209 | 12,209 |
| events | 29,154 | 29,154 |
| performances | 289,142 | 289,142 |
| observations | 2,129,046 | 2,129,051 |
| source_identities | 0 | **1,116,961** |
| conflicts | 0 | **1,361** |
| review_cases | 0 | **27,967** |
| coverage rows | 0 | **210** |
| snapshots | 0 | 1 |

`index` had never been run in this store before today; the derived plane (identities,
retained conflicts, review queue, coverage, snapshot) is now materialised, which is what turns
"rows in a store" into an auditable census. On-disk: 7.6 GB store / 1.8 GB of entity bytes.

Note the shape: 1.12 M source identities against 1.57 M athlete observations means most athlete
rows still carry a single provider identity — that is the §32 identity work the review queue now
owns (27,967 pending cases).

## 3. Source access: MileSplit is hard-blocked as of today

Observed, in this order:

1. 07:09–07:11 UTC — a national MileSplit walk (`collect --all-states --state-concurrency 6`)
   fetched ~45 state team indexes successfully (HTTP 200) and wrote 2,454 cache artifacts.
2. 07:11:19 UTC onward — **every roster request returned HTTP 403**: 582 of 582 non-success
   responses in that run, across `al/ar/az/ca/co/ct/...`, each in ~1 s, with one request per host
   per second and at most one request in flight per host.
3. Afterwards, from a plain `curl` with a desktop browser User-Agent:
   `ny.milesplit.com/teams` → 403, `ny.milesplit.com/teams/<id>/roster` → 403,
   `wi.milesplit.com/teams/<id>/roster` → 403.

So the block is at the `*.milesplit.com` family level for this IP, not a robots rule (their
robots.txt carries no `Crawl-delay` and does not disallow `/teams`) and not a per-path rule.
Earlier, smaller runs in the same session (2 requests to `ny`, 6 to `wy/vt`) succeeded — the block
followed sustained cross-host volume, not a single large burst.

**Operational rule (this is §69 of the mission brief):** a 403/429 must be persisted, not
rediscovered. Implemented and verified in code:

- The fetcher records one `SourceAccessCondition` per `(kind, host)` the moment a 403/429 is
  seen (`census_domain::model::SourceAccessCondition`, slug key `forbidden:<host>`,
  `rate_limited:<host>`), captures the published `Retry-After` when present, and applies a
  six-hour cooldown default.
- The conditions are persisted to the `source_access` store table (`Table::SourceAccess`, file
  `source_access`) and reported as `CollectReport.blocked_hosts`; the roster walk stops issuing
  requests once a blocking condition is seen — later rosters are counted as `blocked_skipped` and
  stay owed, so the rest of the state costs the host nothing.
- The classifier itself is tested (`census/sweep/access.rs::tests`): 403 and 429 end a walk, and
  404/500/robots/oversize/schema failures do not — stopping a 40,000-request walk on a transient
  500 would trade one blocked run for months of uncollected work.

**Correction, observed later the same day:** the block has *cleared*. `provider milesplit
--states WI --limit 20` skipped all 597 WI rosters as already journaled (correct resume
behaviour, zero requests), and `provider milesplit --states AL --limit 3 --refresh` then fetched
the AL team index and 3 rosters — 4 requests, 0 errors — so `al.milesplit.com` served this IP
again. Treat the 07:11 UTC block as a several-hour rate/abuse response, not a ban: the lane can
resume, at a lower cross-host rate, and a re-block now costs one request instead of 40,000.

That probe is visible in the store: `consolidate` afterwards reports schools 13,483 → 13,486,
teams 75,940 → 75,946, athletes 879,587 → 879,664 — four requests, three rosters, no Athletic.net
traffic.

## 4. Which lanes can actually run today

From `cli/provider.rs`'s dispatch match (the CLI help text is *not* the source of truth):

- **Live crawl:** `ks`, `wiaa`, `wiaa_results`, `ihsa`, `ihsa_tournament`, `ohsaa`, `mshsl`,
  `wayzata`, `athleticlive_athletes`, `athleticnet` (operator-authorized host), `milesplit`
  (currently blocked), `plain_names`, `coach_contacts`.
- **Capture import — issues no requests:** `athleticlive` (needs an operator-supplied
  `athleticlive-*.csv` harvest) and the result plane in `sources/athleticlive/results.rs`
  (`athleticlive_results_v1`, folds captured event/summary/standings documents).
- **Documented but not routed:** `athleticlive_results` and `hytek`, `raceday`, `tfrrs` appear in
  the CLI help text or in the sources tree, but `athleticlive_results` has no arm in the provider
  dispatch match — invoking it reports "unknown adapter". Worth fixing in the help text or the
  match, not in the data.

Athletic.net remains the workhorse and is the one host the operator explicitly authorizes
(`--authorized-host athletic.net`, 2 rps ceiling enforced): whole-meet acquisition is two requests
per meet, and 199,996 cohort athletes already carry a profile URL, so the profiling route is a
targeted lookup rather than a search.

## 5. Local model server: running, and the review lane has run against it

- Upstream `ec5a12b85` was built CPU-only at `/tmp/llama-master/build-cpu/bin/llama-server`
  (CUDA stays unavailable: this checkout's `ggml-cuda` uses `cuda::make_strided_iterator` /
  `cuda::make_counting_iterator`, removed in CUDA 13, so the 5090 build fails to compile; the
  35B-A3B Q4_K_XL model fits the 123 GB of RAM and A3B MoE keeps ~3B parameters active per token).
- It loads the model in ~64 s and serves `http://127.0.0.1:52080` (`--jinja`), verified with
  `/v1/models` and a real completion. Two details matter for any client:
  - the readiness line is `listening on http://…`, not `server is listening` — a supervisor
    watching for the older string times out against a perfectly healthy server;
  - the model is a reasoning model, so an unconstrained request spends its whole output budget in
    `reasoning_content` and returns empty `content`. The client sends
    `chat_template_kwargs: {"enable_thinking": false}` and a `response_format` JSON schema, which
    is what makes the answers parseable.
- Measured throughput: ~16 s per case (a packet plus a short verdict), ~13 tokens/s, one request
  in flight.
- The only two models on disk are `Qwen3.6-35B-A3B-UD-Q{4,5}_K_XL.gguf`; `Qwen3-Coder-Next-GGUF`
  is an empty directory.

## 6. The review lane: implemented, run against the store, and what it found

`census-service review` (`crates/census-service/src/identity/`) asks the local model about the
cases the index retained, validates every answer against the store's own evidence, and records the
verdict either way in the new `identity_verdicts` plane. What the index actually retains, measured
today:

| Retained family | Rows | Lane asks? |
| --- | --- | --- |
| Class-of-2027 identity confidence | 26,608 | no — needs cohort evidence this store does not hold |
| Coach mailbox withheld | 917 | no — withheld on purpose by the collection contract |
| Meet venue unresolved | 442 | yes — the missing field is the meet's jurisdiction (`state`) |
| School jurisdiction unresolved | 0 | yes — the same question for schools |
| *(conflicts)* Athlete identity | 1,249 | retained conflicts, operator's queue |
| *(conflicts)* School identity | 112 | retained conflicts, operator's queue |

Live passes, as run today (`review --family venue --limit 5`, `--observed-on 2026-09-22`):

```
asked=5 accepted=2 rejected=0 insufficient=3 unanswered=0 dropped=0 failed=0   (jurisdiction rule, as run)
asked=3 accepted=0 rejected=3 insufficient=0 unanswered=0 dropped=0 failed=0   (first pass: the venue-era rule refused all three)
```

The accepted verdicts are real placements, e.g. `meet_08d087631573abba` → `state=MN`
(confidence 100, "the evidence identifies the venue as St. Anthony Village HS, which is in
Minnesota") and `meet_094d5574d8756fd0` → `state=WI` (95). Design rules the code enforces, not
just documents: the request is schema-constrained; `VerdictBatch::sanitize` drops any verdict
naming a case the packet did not ask about and demotes a "proposal" with no value; `validate`
admits only a real jurisdiction (`UsJurisdiction::parse`, so `WI` and `Wisconsin` both work) and
only when the subject does not already carry one; and a verdict is *evidence, not an edit* — the
pass writes `identity_verdicts` and moves the case to `resolved`/`retained`, and never rewrites a
canonical row (the merge owns those tables).

## 7. What is still missing for end-to-end

1. **26,608 cohort-confidence + 917 mailbox cases** stay with an operator: a local model cannot
   add the missing evidence, and the mailbox rule is a contract.
2. **~429 meet placements** remain in the queue; each costs ~16 s of CPU, so the lane is run in
   batches (`review --limit N`). Applying an accepted verdict to canonical data is deliberately
   not built: it would mean feeding the jurisdiction back as a source observation, which the merge
   owns.
3. **MileSplit can resume** (block cleared, §3) — at a lower cross-host rate than the walk that
   tripped it, with the block policy now in place.
4. **Derived planes are exported** beside this document (`pipeline/derived/`): `review_cases`
   (27,967), `conflicts` (1,361), `coverage` (210), `snapshots`, `source_access`,
   `identity_verdicts`, plus the refreshed workbook and both census scopes.
