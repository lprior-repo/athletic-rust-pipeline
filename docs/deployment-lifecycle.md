# Deployment lifecycle

A national census executes for days on a Restate cluster. The journal lives in the node, not in
the endpoint process: a crash of `census-serve` resumes at the last recorded step. Rolling a new
build is not a restart — it is a new registration, and the old registration stays alive until every
in-flight invocation finishes or is cancelled. Nothing in the repo encodes this lifecycle today;
the following describes how it must work and why.

## The two safety rules

1. **A version is immutable once registered.** A binary that has been registered with Restate must
   never be rebuilt in place. Rebuilding a registered version's binary and re-registering it causes
   RT0016 journal mismatches because the journal still references the old invocation chain and the
   new binary's type layout, error paths, and retry semantics may differ. The only safe path is to
   build into a *new* path and register that path.

2. **No version is removed while it has active invocations.** A deployment that still has workflows
   in progress must not be shut down or unregistered. In-flight invocations are pinned to the
   deployment that accepted them; removing the deployment while work remains causes those invocations
   to fail with unrecoverable errors, corrupting the journal and potentially losing data.

These rules exist because of the replay hazards that Restate workflows exhibit across builds.

## Invocation timeouts

Restate asks an invocation to suspend after `inactivity_timeout` without journal progress, then aborts
it `abort_timeout` later. Both default to one minute and ten minutes, and both are read from the
manifest the endpoint publishes — per service — so an endpoint that declares neither gets the defaults
whatever the node's own configuration says.

The census needs longer than the defaults, because **queueing is in-flight time**. Every handler runs
its work on the blocking pool (`--max-concurrent` slots), and the national fan-out submits every
jurisdiction at once, so most handlers spend their first minutes waiting for a slot with no journal
entry to show for it. Restate read that wait as a stall: on 2026-09-24 the revision-8 nationwide run
lost 47 of its 49 jurisdictions to a ten-minute abort while they were queued, and finished with two.

`build_endpoint` therefore binds the nine store-backed services with `ServiceOptions` declaring an
hour for each timer (`CENSUS_INACTIVITY_TIMEOUT`/`CENSUS_ABORT_TIMEOUT` in `restate_services/mod.rs`),
and `fjall_restate_e2e` asserts both numbers appear in the manifest that every one of them advertises.
`BrowserSession` keeps the defaults on purpose: the lane answers one request at a time, and a lane
that hangs should be aborted quickly rather than held for an hour.

The hour is also the backstop for a genuinely stuck handler. An abort discards whatever the blocking
job had done in memory — the journal holds only what landed — so the invocation's next attempt
replays that work. The retry policy decides what happens after an abort; the timer decides only that
waiting forever is not one of the options.

## What breaking rule 1 looks like

The failure is silent, which is why it is worth recognising. A client that submits work against a
stale registration prints `submitted as invocation inv_…` and then blocks until its own observation
bound expires; re-running the same command prints the same and blocks again. The endpoint's log says
nothing, the store gains no rows, and the HTTP cache gains no bodies, so the run looks like a crawl
that is thinking rather than one that is not running at all.

The invocations are visible from the node's admin API:

```bash
curl -s -X POST http://127.0.0.1:19095/query \
  -H 'content-type: application/json' -H 'accept: application/json' \
  -d '{"query":"SELECT id, status, target FROM sys_invocation"}'
```

`status: "paused"` is the signature. Every `JurisdictionCensus` and `Sweep` handler carries
`invocation_retry_policy(on_max_attempts = "pause")`, so an invocation whose journal no longer
matches the registered schema fails its three attempts and is parked instead of failing the caller.
A paused invocation **keeps its virtual-object key**, so every later submission for the same
`<jurisdiction>:<season>:<revision>` waits behind it as `pending` and never starts: the key stays
poisoned until the paused invocation is ended.

```bash
# Ends the paused invocation (the key is released; it reports `completed`)
curl -s -X PATCH http://127.0.0.1:19095/invocations/<id>/kill \
  -H 'content-type: application/json' -H 'accept: application/json'
```

Measured 2026-09-24: rebuilding `census-serve` in place and restarting it at the same URI left 59
paused invocations — the revision-5 sweeps of roughly half the states. The DC key absorbed three
further submissions, none of which ran, before the paused one was killed; the run then completed in
minutes and wrote its observations. Re-registering the rebuilt binary bumped each service to
revision 8 and the symptoms stopped.

## Replay hazards

A Restate workflow invocation may replay on a different build of the same endpoint. If the binary
changes, replayed steps may produce different results. The hazards are:

| Hazard | Description |
|---|---|
| **Current time** | A step that reads `SystemTime::now()` at build A may see a different value at build B during replay, causing divergent branch decisions or date-based queries. |
| **HTTP calls** | A step that fetches a URL may receive different content between builds if the upstream served new data; the step's result (a parsed struct) may have different fields. |
| **Random values** | A step that generates a random identifier or sample will produce different values on replay, potentially altering which records are selected. |
| **Unordered collections** | A step that collects results into a `HashSet` or iterates a `HashMap` may produce different ordering on replay, affecting any downstream comparison that depends on iteration order. |

These hazards are inherent to the journal-and-replay model. The only mitigation is immutability:
if a version's binary is never rebuilt in place, the step code is the same on replay as it was
during the original execution, so the hazards are bounded to the same build's behavior.

## Manual lifecycle

### 1. Build into a versioned path

Build the binary for the target commit and place it under `/releases/<git-sha>/`:

```bash
cargo build --release -p census-service
cp target/release/census-service "/releases/$(git rev-parse HEAD)/census-service"
```

The path is keyed by the full git SHA so every build is addressable and never collides with another.
The directory must not already exist: if `/releases/<sha>/` exists, a previous build already claimed
that SHA and the binary must not be overwritten. This is the enforcement of the immutability rule.

### 2. Assign a port slot

Each version gets its own TCP port, starting at 19100 and incrementing per version:

| Version index | Port  |
|---|---|
| 1 | 19100 |
| 2 | 19101 |
| 3 | 19102 |
| … | … |

The port is chosen before starting the endpoint and must not be in use by another version. A port
collision means two versions are serving on the same address, which breaks the journal's routing.

### 3. Start the new endpoint

Start `census-serve` on the assigned port, pointing at the version's binary:

```bash
census-serve --listen 127.0.0.1:<port> --data-dir var/census-service --max-concurrent 8
```

One store serves one process. `census-serve` opens `--data-dir` for writing at startup and a Fjall
store has one writer, so the new endpoint cannot come up on its new port while the old one still
holds the store: the old process has to drain and exit first, and only then does the new port start
answering. The registration order is the half that must not be skipped — register the new deployment
before removing any registration for the old endpoint, so no invocation window resolves to a dead
address. Measured 2026-09-24: the old endpoint exited 2 s after `SIGTERM` with nothing in flight, the
new one answered on its own port 2 s later, and the canary that followed ran its results stage on the
new revision.

### 4. Register the deployment

Register the new endpoint with the Restate node so that new invocations route to it:

```bash
restate deployment register --name <name> --uri http://127.0.0.1:<port>/
```

New invocations will route to the new deployment; existing invocations on the old deployment
continue unaffected.

### 5. Drain the old deployment

Inspect the old deployment's active invocation count:

```bash
restate deployment describe <old-id> --extra
```

Wait until `active_invocations` is zero. During this period, new invocations use the new
deployment while in-flight ones on the old deployment complete or are cancelled.

### 6. Shut down and unregister

Once `active_invocations` is zero:

```bash
# Shut down the old endpoint process (SIGTERM triggers the drain sequence)
kill <old-pid>

# Unregister the old deployment
restate deployment unregister <old-id>
```

Without the CLI on `PATH` — and it is not installed on this host — the admin API does the same work,
and retirement needs `force=true`: `DELETE /deployments/<id>` answers `501 Not Implemented`, while
`DELETE /deployments/<id>?force=true` answers `202 Accepted`. Registration is `POST /deployments`
with `{"uri": "http://127.0.0.1:<port>/"}`. Measured 2026-09-24: two registrations for one endpoint
had accumulated — `http://localhost:9080/` at revisions 9/5 and `http://127.0.0.1:9080/` at revisions
8/4, because one host spelled two ways is two deployments — and both were retired this way once the
new deployment on its own port was registered and serving revisions 10/6.

The binary at `/releases/<old-sha>/` may be removed after unregistration.

### 7. Reuse the port

After unregistering, the port is free. If this SHA is revisited in the future (e.g. a rebuild
for a dependency bump), a new SHA will be produced and a new port will be assigned. The old
port does not need to be reclaimed.

## The deploy-lifecycle script

`tools/deploy-lifecycle.sh` automates the mechanical steps: installing the binary, printing the
register command, and listing registered deployments with their active-invocation counts. See
`tools/deploy-lifecycle.sh --help` for usage.
