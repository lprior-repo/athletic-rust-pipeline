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

The binary at `/releases/<old-sha>/` may be removed after unregistration.

### 7. Reuse the port

After unregistering, the port is free. If this SHA is revisited in the future (e.g. a rebuild
for a dependency bump), a new SHA will be produced and a new port will be assigned. The old
port does not need to be reclaimed.

## The deploy-lifecycle script

`tools/deploy-lifecycle.sh` automates the mechanical steps: installing the binary, printing the
register command, and listing registered deployments with their active-invocation counts. See
`tools/deploy-lifecycle.sh --help` for usage.
