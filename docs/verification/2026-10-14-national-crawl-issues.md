# National crawl issue dossier, 2026-10-14

Scope: the live census at `var/census-national-20261013` served by `census-serve` pid 3515375 on
`127.0.0.1:9085`, Restate node `census-national-20261013` (admin 19099, ingress 18099).
Every item below is an **executed observation** on that run, with the command and its output. Where
a cause is not yet established it is marked as a lead with the check that would settle it, not as a
finding. Line numbers refer to the revision at which the observation was taken.

## A. The teams stage never settles for 35 of 49 jurisdictions

Evidence (`census-service open-work --ingress http://127.0.0.1:18099/ --season 2026 --revision 1 --json`,
aggregated over all 49 jurisdiction entries):

```
teams    complete  14
teams    unknown   35
```

The work is nonetheless happening — the endpoint's own log shows index collection completing state
by state:

```
collect_state_teams{jurisdiction=Colorado refresh=false}: team index collected state="CO" teams=429
collect_state_teams{jurisdiction=NorthCarolina refresh=false}: team index collected state="NC" teams=899
collect_state_teams{jurisdiction=Ohio refresh=false}: team index collected state="OH" teams=977
```

So collection succeeds while the durable stage stays `unknown`. The observer counts exactly that
field, which is why it reports a frozen `teams 14/49`:

`crates/census-service/src/cli/national/observe.rs:122` — `if state.teams.is_completed() { … }`,
printed by the `progress:` line at `observe.rs:139`.

Impact: any completion logic keyed on the stage state (including an operator watching the progress
line) sees no progress. Patch point: whatever transitions `TeamsStage` from `Owed`/in-progress to
`Completed` never runs when collection finishes. This is the same family as the `ekqb` finding
(fault-injection seam cannot hold or settle a stage).

## B. Captures are not being archived

Evidence:

```
find var/census-national-20261013/http -newermt '-30 minutes' -type f | wc -l   →  0
find var/census-national-20261013/http -type f | wc -l                          →  33297
```

and the per-object read `open-work --source-object milesplit_wi --json`:

```
"source_object_inventory": { "objects": 1, "accepted_observations": 0, "accepted_nothing": 1, "unreadable": 0 }
```

So the archive is frozen at 33,297 bodies while `TeamsSource` completed 276 invocations in the same
23 minutes. Lead (not yet established): archive admission capacity. The crawler has an explicit
capacity knob, `CENSUS_ARCHIVE_CAPACITY_BYTES`
(`crates/census-crawl/src/net/cache/archive/admission.rs:11`), and admission is refused wholesale
when a capture does not fit. Check: compare the store's archive bytes against the effective
capacity (`archive_bytes_used` / `archive_capacity` in
`crates/census-crawl/src/net/cache/archive_tests*`), then decide whether the default is simply too
small for a national corpus.

## C. Every jurisdiction has refused sources

Evidence (same `open-work --json`):

```
refused_sources > 0   True   49     (all 49)
AL refused_sources 4 · AZ 3 · AR 2 · …
```

Impact: refused sources cannot contribute captures, so their athlete/meet/result evidence is absent
by construction. The refusal *reason* is not yet extracted — the endpoint log at INFO carries no
refusal line, and `silent_sources` is empty. Patch point: surface the refusal record (source,
status, reason) through `open-work`; today the count is available and the cause is not.

## D. meets / results / contacts have never started in any jurisdiction

Evidence (same `open-work --json`):

```
meets    unknown  49
results  unknown  49
contacts unknown  49
publication complete 49
```

and the published sidecars from the last cycle:

```
0 var/census-national-20261013/out/source_access.jsonl
0 var/census-national-20261013/out/source_meets.jsonl
```

This is the hard proof behind the workbook's zero `Meets` and zero performances: those stages are
not reached, so the results-lane beads (`niz5`, `6qkk`, `616`, `3bo`, `ead`) are the correct
owners of that gap, not the crawl's runtime.

## E. The cycle's `index` stage is skipped whenever it runs against a live endpoint

The cycle run through the ingress printed:

```
index	skipped (offline stage: `index --store <dir>` with census-serve stopped)
```

`crates/census-service/src/cli/cycle.rs:231` emits exactly that. So a cycle driven by
`national --drive` / `run --ingress` publishes a generation whose `source_access.jsonl` and
`source_meets.jsonl` are **0 bytes** (observed above), and whose Sources/Coverage evidence is
therefore empty. Patch point: either run the derivation through the serving endpoint, or make the
skip explicit in the publication manifest so a reader cannot mistake it for measured zero.

## F. The browser lane has never been engaged

Evidence:

```
curl … -d '{"query":"SELECT count(*) FROM sys_invocation WHERE target_service_name = '"'"'BrowserSession'"'"'"}'
{"rows":[{"count(*)":0}]}
```

Zero `BrowserSession` invocations on a run that includes browser-transport sources
(`athleticnet` declares `transport: browser` and `origin: www.athletic.net` in the published
Source table). The only chromium processes on the host are the operator's desktop browser
(`--ozone-platform=wayland` with user extensions), not the headless lane. Impact: browser-transport
sources cannot produce captures, which matches the empty athlete/PR sheets for them.

## G. Operational traps (resolved or documented this session)

1. **Deduplicated submissions silently ignore changed parameters.** Restate dedupes a repeat
   submission of the same run identity, so `--source-parallelism`, `--delay-ms`, `--states`,
   `--refresh`, `--limit` and `--concurrency` had no effect; the CLI said so only in a quiet
   `note:`. Fixed in `59a80d45`: the message now states that every changed parameter was ignored
   and what to run instead, and `national --drive` re-drives under a fresh `--resume` ordinal.
2. **Draining pauses the parent and pins it to the replaced deployment.** Observed:
   `sys_invocation` row for the parent showed `status = paused` with
   `pinned_deployment_id = dp_17xTeOaYmaPGzXGeuwaTdTj` (the retired 9084 deployment). `cancel`
   returned `202` and did nothing; `kill` returned `200` and cleared it
   (`docs/restate/invocation.md:79`: "If the deployment is not reachable, use kill instead").
   While paused it blocked all later submissions and froze the fan-out. Documented in `AGENTS.md`
   under "Reloading a deployed census after new code lands".
3. **A store carries exactly one run.** `--revision 2` against a store bound to `2026-1` is
   refused: "a fresh census needs its own store root". A from-scratch crawl needs a new store root
   (and can warm from the existing 107 MB HTTP cache).

## H. Not storage

Fjall is healthy and writing: `fjall/1.jnl` measured 67,108,864 bytes with an mtime one second
before the check, the lock is held by the serving process, the store is 138 MB, and the volume has
848 GB free. The journal rolls at exactly 64 MiB. Note for scale planning: `keyspaces/2` is 33 MB
against ~165 MB of journal.

One correction recorded for honesty: an earlier statement in this session that Fjall had written
nothing for ten minutes was wrong — it compared local-time file mtimes against a UTC `date`
(local is CDT, UTC−5).

## What would falsify each item

- A: re-running `open-work --json` later and finding `teams complete` advancing; or the endpoint log
  showing a settlement for a state whose stage stays `unknown`.
- B: new files appearing under `http/` while the crawl runs; or `accepted_nothing` dropping for a
  named source object.
- C: `refused_sources` decreasing after a rate or grant change.
- D: `meets`/`results` leaving `unknown`.
- F: any `BrowserSession` invocation appearing.
