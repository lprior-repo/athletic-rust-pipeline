# Restate Operational Surface

> Fetch date: 2026-09-22. All URLs point to the current Restate docs at <https://docs.restate.dev>.
> The repo pins `restate-sdk = "=0.12.0"`, which maps to **Restate 1.7.x** server (protocol versions 5–7).
> The `deploy/restate.toml` notes the deployment runs the 1.7.10 build.
> Everything below is quoted from docs fetched on 2026-09-22 unless marked UNVERIFIED.

---

## Table of Contents

1. [restatectl — cluster/partition/status commands](#1-restatectl)
2. [Journal-level invocation introspection (SQL)](#2-journal-level-introspection)
3. [Ingress — calling handlers over HTTP](#3-ingress)
4. [Admin API — deployments and services](#4-admin-api)
5. [Retry / timeouts — operational controls](#5-retry-timeouts)
6. [Durability and data — where state lives, backups](#6-durability-data)
7. [Configuration reference — restate.toml keys](#7-configuration-reference)
8. [Observability — OTLP / metrics / tracing](#8-observability)

---

## 1. `restatectl`

`restatectl` is the cluster-management CLI for Restate. It connects to the **admin port** (default `5122`) on each node's advertised address.

**Source:** [Installation → Advanced: Installing `restatectl`](https://docs.restate.dev/installation.md), [Clusters → Controlling clusters with `restatectl`](https://docs.restate.dev/server/clusters.md)

### Commands and examples

| Subcommand | What it does | Example invocation |
|---|---|---|
| `restatectl status` | View cluster state: nodes, leaders, followers, log-server members | `restatectl status --addresses http://127.0.0.1:5122/` |
| `restatectl status --extra` | Same + log configuration, all partition processors (Applied/Durable/Archived LSNs), metadata members | `restatectl status --extra --addresses http://127.0.0.1:5122/` |
| `restatectl nodes list` | List all nodes and their roles | `restatectl nodes list --addresses http://127.0.0.1:5122/` |
| `restatectl partitions list` | List all partition processors, status (Leader/Follower), Applied/Durable/Archived LSNs, LSN-lag, dead nodes | `restatectl partitions list --addresses http://127.0.0.1:5122/` |
| `restatectl logs list` | List all logs, replication, sequencer, nodeset | `restatectl logs list --addresses http://127.0.0.1:5122/` |
| `restatectl logs describe` | Describe each log's segment chain | `restatectl logs describe --addresses http://127.0.0.1:5122/` |
| `restatectl config get` | Show cluster settings: partitions, replication, log provider | `restatectl config get --addresses http://127.0.0.1:5122/` |
| `restatectl config set` | Update cluster settings (replication, partition/replication independently) | `restatectl config set --replication 2 --addresses http://127.0.0.1:5122/` |
| `restatectl provision` | Provision a new cluster | `restatectl provision --address http://127.0.0.1:5122/ --yes` |
| `restatectl snapshots create-snapshot <N>` | Trigger a snapshot for a partition (or range, or omit for all) | `restatectl snapshots create-snapshot --addresses http://127.0.0.1:5122/` |

### Snapshot output

Snapshots land in the configured `worker.snapshots.destination` (S3/GCS/Azure) under a prefix containing the partition ID. The most recent snapshot is tracked in `latest.json`.

**Source:** [Snapshots & Backups → Observing processor persisted state](https://docs.restate.dev/server/snapshots.md), [Snapshots → Configuring Automatic Snapshotting](https://docs.restate.dev/server/snapshots.md)

---

## 2. Journal-level Introspection (SQL)

Restate exposes a **DataFusion-powered SQL interface** on the admin server. The port is the **admin port** (default `9070` for the HTTP admin endpoint, or the port configured under `[admin]`).

**Source:** [SQL Introspection API reference](https://docs.restate.dev/references/sql-introspection.md), [Introspection docs](https://docs.restate.dev/services/introspection.md)

### Access methods

```shell
# CLI (via `restate` CLI, not restatectl)
restate sql --json "SELECT * FROM sys_journal WHERE id = 'INVOCATION_ID';"

# Direct HTTP (admin API query engine)
curl http://127.0.0.1:9070/query --json '{"query": "SELECT * FROM sys_journal WHERE id = '\''INVOCATION_ID'\'';"}'
```

### Key tables for invocation-level inspection

| Table | Purpose |
|---|---|
| `sys_invocation` | Per-invocation metadata: status, retry_count, last_failure, trace_id, journal_size, pinned_deployment_id, target service/handler |
| `sys_journal` | Journal entries for all invocations. Columns: `id`, `index`, `entry_type`, `name`, `completed`, `invoked_id`, `invoked_target`, `sleep_wakeup_at`, `raw` (Binary), `entry_json` (JSON string, journal version 2 only), `appended_at` |
| `sys_vqueue_entry_status` | Queue stage, status (new/scheduled/started/backing-off/yielded/killed/cancelled/failed/succeeded), attempt counts, blocking reasons |
| `state` | Application K/V state: `service_name`, `service_key`, `key` (state key name), `value_utf8`, `value` (Binary) |
| `sys_service` | Registered services: `name`, `revision`, `public`, `ty` (service/virtual_object/workflow), `deployment_id` |
| `sys_deployment` | Deployments: `id`, `ty`, `endpoint`, `created_at`, `services` |

### Example queries

```sql
-- The journal of invocation X, step by step:
SELECT index, entry_type, name, completed, invoked_target, entry_json
FROM sys_journal
WHERE id = 'INVOCATION_ID'
ORDER BY index;

-- Current status of invocation X:
SELECT id, target, status, retry_count, last_failure, journal_size, trace_id
FROM sys_invocation
WHERE id = 'INVOCATION_ID';

-- All invocations for a given service:
SELECT id, target, status, created_at, modified_at
FROM sys_invocation
WHERE target_service_name = 'Census'
ORDER BY created_at DESC;

-- Application state for a virtual object:
SELECT key, value_utf8
FROM state
WHERE service_name = 'Ingest'
  AND service_key = 'mshsl';

-- Join invocation with its journal entries:
SELECT i.id, i.target, i.status, j.index, j.entry_type, j.name, j.completed
FROM sys_invocation i
JOIN sys_journal j ON i.id = j.id
WHERE i.id = 'INVOCATION_ID'
ORDER BY j.index;
```

### Entry types (from `sys_journal.entry_type`)

Entry types are defined in Restate's source: [`crates/types/src/journal/entries.rs`](https://github.com/restatedev/restate/blob/main/crates/types/src/journal/entries.rs).
The `entry_json` column (journal version 2+) provides a JSON-serialized view of each entry.

### Entry status values (from `sys_invocation.status`)

`pending` | `scheduled` | `ready` | `running` | `paused` | `backing-off` | `suspended` | `completed`

**Source:** [Introspection → Inspecting invocations](https://docs.restate.dev/services/introspection.md), [SQL Introspection API reference](https://docs.restate.dev/references/sql-introspection.md)

---

## 3. Ingress — Calling Handlers over HTTP

The ingress listens on the **ingress port** (default `8080`; the repo config sets `bind-port = 18095` under `[ingress]`).

**Source:** [HTTP invocation docs](https://docs.restate.dev/services/invocation/http.md), [Versioning docs](https://docs.restate.dev/services/versioning.md)

### Path patterns (Restate 1.7+)

| Target type | Path pattern | Example |
|---|---|---|
| Plain service | `/restate/call/{Service}/{handler}` | `/restate/call/Census/status` |
| Virtual object | `/restate/call/{Service}/{key}/{handler}` | `/restate/call/Ingest/mshsl/ingest` |
| Workflow | `/restate/call/{Service}/{workflow_key}/{handler}` | `/restate/call/NationalCensus/national:2025:1/run` |

The repo uses the **1.7+ path shape**: `restate/call/<Service>/<handler>`, `restate/call/<Service>/<key>/<handler>`. (Older <=1.6 used `/{service}/{handler}`.)

### Key examples (matching the repo's services)

```shell
# Census/status — plain request/response service
curl http://127.0.0.1:18095/restate/call/Census/status --json '{}'

# Ingest ingestion — keyed virtual object
curl http://127.0.0.1:18095/restate/call/Ingest/mshsl/ingest \
  --json '{"url": "https://example.com/results", "window_start": "2025-06-01"}'

# JurisdictionCensus — keyed virtual object
curl http://127.0.0.1:18095/restate/call/JurisdictionCensus/jurisdiction:ia:2025:1/run \
  --json '{"season": "2025", "revision": 1}'

# NationalCensus — workflow
curl http://127.0.0.1:18095/restate/call/NationalCensus/national:2025:1/run \
  --json '{"season": "2025", "revision": 1}'

# Report/run — workflow, keyed by scope+timestamp
curl http://127.0.0.1:18095/restate/call/Report/report:ia:all/1726972800/run \
  --json '{"scope": "all"}'
```

### One-way fire-and-forget

Use `/restate/send/` instead of `/restate/call/`. Response includes `invocationId`:

```shell
curl http://127.0.0.1:18095/restate/send/Ingest/mshsl/ingest --json '{}'
# → {"invocationId":"inv_...","status":"Accepted"}
```

### Idempotency

Pass `Idempotency-Key` header. Restate persists the response for the configured retention (default 24h for idempotency keys).

```shell
curl http://127.0.0.1:18095/restate/call/Census/status \
  -H 'Idempotency-Key: my-key-123'
```

### Attaching to a running invocation

```shell
# By ID
curl http://127.0.0.1:18095/restate/attach/inv_XXX

# By target
curl http://127.0.0.1:18095/restate/attach --json \
  '{"target":"workflow","workflowName":"NationalCensus","workflowKey":"national:2025:1"}'
```

### Lookup invocation ID from a target

```shell
curl http://127.0.0.1:18095/restate/lookup --json \
  '{"target":"workflow","workflowName":"NationalCensus","workflowKey":"national:2025:1"}'
# → {"invocationId": "inv_..."}
```

### HTTP status codes and headers

| Code | Meaning | Retry? |
|---|---|---|
| `200` | Success | — |
| `400` | Bad request (e.g. handler not found) | **No** |
| `404` | Service not found | **No** |
| `408` | Request timeout | Yes |
| `425` | Too early (ordered invocation, waiting for earlier ones) | Yes |
| `429` | Rate-limited / overloaded | Yes |
| `5xx` | Server error | Yes |

The `x-restate-error-source` header indicates whether the error came from `invocation` (your handler — don't retry) or `ingress` (transient — may retry).

**Source:** [HTTP invocation docs](https://docs.restate.dev/services/invocation/http.md)

---

## 4. Admin API — Deployments and Services

The admin API lives on the **admin port** (default `9070` in the repo's config: `bind-port = 19095` under `[admin]`).

**Source:** [Versioning docs](https://docs.restate.dev/services/versioning.md), [Register deployment OpenAPI](https://docs.restate.dev/admin-api/deployment/register-deployment.md), [Introspection docs](https://docs.restate.dev/services/introspection.md)

### Registering a deployment

```shell
# CLI
restate deployments register http://127.0.0.1:9080

# With --force (overwrites existing deployment at same URI)
restate deployments register --force http://127.0.0.1:9080

# With --breaking (allows breaking schema changes)
restate deployments register --breaking http://127.0.0.1:9080

# Admin API (JSON body)
curl -X POST http://127.0.0.1:19095/deployments \
  --json '{"uri":"http://127.0.0.1:9080/","force":true}'
# → 201 Created, Location header with deployment ID
```

The deployment is invoked at its `uri` to discover service definitions (HTTP manifest).

### Listing and inspecting deployments

```shell
# List all deployments
restate deployments list

# Describe a specific deployment (includes active invocations with --extra)
restate deployment describe <DEPLOYMENT_ID> --extra
```

### Removing a deployment

```shell
# Graceful drain (waits for in-flight invocations)
restate deployments remove <DEPLOYMENT_ID>

# Force remove (can break in-flight invocations)
restate deployments remove --force <DEPLOYMENT_ID>
```

### Listing services

```shell
# CLI
restate services list
# → Lists all registered services with metadata

# SQL
restate sql --json "SELECT * FROM sys_service;"
# → name, revision, public, ty (service/virtual_object/workflow), deployment_id
```

### Service-level SQL introspection

```sql
-- All handlers of a service:
SELECT * FROM sys_service_handler WHERE service_name = 'Census';

-- Service metadata:
SELECT * FROM sys_service WHERE name = 'Census';
```

### Modifying service configuration via Admin API

```shell
# Via SQL (the CLI shortcut)
restate sql --json "SELECT * FROM config WHERE key = 'service.restate-service.config.retries';"

# Admin API endpoint for service config modification
curl -X PATCH http://127.0.0.1:19095/services/Census/config \
  --json '{"retryPolicy": {"maxAttempts": 3}}'
```

### Versioning semantics

- Deployments are **immutable**: once registered, the handler signatures cannot change without `--breaking`.
- In-flight invocations are **pinned** to the deployment that started them. Retries always go to the same deployment.
- Adding handlers to a deployment is allowed; removing or renaming them requires `--breaking`.
- Virtual Object state **persists across deployments**.

**Source:** [Versioning docs](https://docs.restate.dev/services/versioning.md)

---

## 5. Retry / Timeouts — Operational Controls

### Retry policy (server defaults)

**Source:** [Service Configuration → Retries](https://docs.restate.dev/services/configuration.md)

```toml restate.toml
[invocation.default-retry-policy]
initial-interval = "50ms"
exponentiation-factor = 2.0
max-attempts = 70
max-interval = "60s"
on-max-attempts = "pause"     # "pause" (default) or "kill"
```

### Service/handler-level retry (Rust SDK attribute)

From the Rust SDK `0.12.0`, retry policies are set on `#[service]` or `#[handler]` attributes:

```rust
#[restate_sdk::service(invocation_retry_policy(
    initial_interval = "100ms",
    factor = 2.0,
    max_interval = "3s",
    max_attempts = 3,
    on_max_attempts = "pause",
))]
impl MyService { ... }
```

Or per-handler:

```rust
#[handler(invocation_retry_policy(
    max_attempts = 3,
    on_max_attempts = "pause",
))]
async fn my_handler(&self, ctx: Context<'_>) -> Result<(), HandlerError> { ... }
```

### Timeouts

| Setting | Default | Description |
|---|---|---|
| `inactivity_timeout` | `1m` | Max time without journal progress before suspend |
| `abort_timeout` | `10m` | Wait after inactivity timeout before killing handler |

Set via SDK attributes (durations as strings):

```rust
#[restate_sdk::service(
    inactivity_timeout = "15m",
    abort_timeout = "5m",
)]
```

Or via server config `[worker.invoker]`:

```toml restate.toml
[worker.invoker]
inactivity-timeout = "1m"
abort-timeout = "10m"
```

### Pausing and resuming invocations

```shell
# Pause (stops retries, preserves progress)
restate invocations pause <INVOCATION_ID>

# Resume (continues from where it left off; optionally on a different deployment)
restate invocations resume <INVOCATION_ID>

# Resume on a new deployment (after bug fix)
restate invocations resume <INVOCATION_ID> --deployment <NEW_DEPLOYMENT_ID>

# Kill (force terminate — no compensation logic runs)
restate invocations kill <INVOCATION_ID>
# Also: kill all invocations for a service/object/handler
restate invocations kill <SERVICE_NAME>
restate invocations kill <SERVICE_NAME>/<HANDLER_NAME>
restate invocations kill <VIRTUAL_OBJECT>/<KEY>
```

### Inspecting retries via SQL

```sql
-- Invocations currently retrying:
SELECT * FROM sys_invocation WHERE retry_count > 1;

-- Invocations backed off:
SELECT * FROM sys_invocation WHERE status = 'backing-off';

-- Most recent failure:
SELECT id, last_failure, last_failure_error_code, last_failure_related_command_index
FROM sys_invocation
WHERE status = 'backing-off';
```

### Journal retention

| Setting | Default | Description |
|---|---|---|
| `default-journal-retention` | `1d` | How long journal survives after invocation completes |
| `default-idempotency-retention` | `1d` | How long idempotency responses are cached |
| `default-workflow-completion-retention` | `1d` | Workflow result retention after `run` completes |

The repo's services set their own retention (90 days journal, 180 days workflow completion) — see `deploy/restate.toml` comment on line 23–26.

**Source:** [Service Configuration](https://docs.restate.dev/services/configuration.md), [Service Configuration → Retention](https://docs.restate.dev/services/configuration.md)

---

## 6. Durability and Data

### Where data lives

Single-node Restate stores everything in `base-dir` (default: `restate-data/` relative to the working directory). The repo's config sets:

```toml restate.toml
base-dir = "/var/lib/census-service-restate"
```

The data directory contains RocksDB databases for:
- **Metadata** (cluster membership, log/partition config)
- **Bifrost logs** (durable write-ahead log)
- **Partition store** (invocation journals, K/V state, timers)

### Backups (single-node)

```shell
# Option 1: Stop the server, archive, restart
systemctl stop restate-server
tar -czf /backup/restate-$(date +%F).tar.gz /var/lib/census-service-restate
# ... later ...
tar -xzpf /backup/restate-2026-09-22.tar.gz -C /var/lib/census-service-restate
# Ensure only one instance runs, then:
systemctl start restate-server
```

**Source:** [Snapshots & Backups → Data Backups](https://docs.restate.dev/server/snapshots.md)

### Snapshots (multi-node clusters)

```shell
# Trigger a snapshot via restatectl
restatectl snapshots create-snapshot --addresses http://127.0.0.1:5122/

# Snapshot destination config
restatectl config get --addresses http://127.0.0.1:5122/
# Look for: worker.snapshots.destination in the running config
```

Snapshots are stored in the object store (S3/GCS/Azure) under the configured prefix. Each partition has its own prefix with `latest.json`.

### Durability mode

The repo uses `durability-mode = "replica-set-only"` (single-node):

```toml restate.toml
[worker]
durability-mode = "replica-set-only"
```

This means the partition store is considered durable when all replicas have flushed locally. For single-node, this means the local RocksDB write is the durability guarantee.

### Restart behavior

| What survives restart | What replays |
|---|---|
| All completed invocation metadata | — |
| All K/V state (virtual objects, workflows) | — |
| In-flight invocation journal entries | Restate replays journal entries from the partition store to restore handler state |
| Registered deployments | — |
| Pending timer/sleep entries | Replayed from journal |
| In-flight handler execution | **Lost** — the handler is re-invoked from the last journal checkpoint |

On restart, Restate reads the partition store (RocksDB), replays any unflushed journal entries, and resumes in-flight invocations from their last journal checkpoint. The deployment the invocation was pinned to is preserved.

**Source:** [Snapshots & Backups](https://docs.restate.dev/server/snapshots.md)

---

## 7. Configuration Reference — `restate.toml` Keys

The repo's config is at `deploy/restate.toml`. Below are the keys it uses plus relevant server-default keys.

**Source:** [Restate Server Configuration reference](https://docs.restate.dev/references/server-config.md)

### Root-level keys

| Key | Value in repo | Default | Purpose |
|---|---|---|---|
| `roles` | `["http-ingress", "admin", "worker", "log-server", "metadata-server"]` | all five | Which roles this node runs |
| `node-name` | `"census-service"` | hostname | Unique node identifier |
| `cluster-name` | `"census-service"` | `"localcluster"` | Cluster membership key |
| `auto-provision` | `true` | `true` | Allow automatic cluster provisioning |
| `default-num-partitions` | `24` | `24` | Partitions (fixed at cluster provision time) |
| `default-replication` | `1` | `1` | Replication factor (log + partition) |
| `base-dir` | `"/var/lib/census-service-restate"` | `"restate-data"` | Data directory |
| `bind-ip` | `"127.0.0.1"` | (auto) | Listen IP |
| `bind-port` | `15152` | `5122` | Fabric port (node-to-node) |
| `advertised-address` | `"http://127.0.0.1:15152/"` | (auto) | External address |
| `listen-mode` | `"tcp"` | `"all"` | TCP only (no unix socket) |
| `shutdown-timeout` | `"1m"` | `"1m"` | Graceful shutdown window |
| `disable-telemetry` | `true` | `false` | Disable anonymous usage reporting |
| `experimental-enable-protocol-v7` | `true` | `false` | Enable experimental protocol v7 |
| `experimental-enable-vqueues` | `true` | `false` | Enable virtual queues (flow control) |
| `experimental-enable-scoped-virtual-objects` | `true` | `false` | Enable scoped virtual objects |

### `[bifrost]`

| Key | Value | Default | Purpose |
|---|---|---|---|
| `default-provider` | `"replicated"` | `"replicated"` | Bifrost log provider |

### `[worker]`

| Key | Value | Default | Purpose |
|---|---|---|---|
| `durability-mode` | `"replica-set-only"` | `"replica-set-only"` (no snapshot repo) | When partition store is considered durable |

### `[admin]`

| Key | Value | Default | Purpose |
|---|---|---|---|
| `bind-port` | `19095` | `9070` | Admin API port |
| `advertised-address` | `"http://127.0.0.1:19095/"` | (auto) | External admin address |

### `[ingress]`

| Key | Value | Default | Purpose |
|---|---|---|---|
| `bind-port` | `18095` | `8080` | Ingress HTTP port |

### Server-default retry policy (not overridden in repo config)

```toml restate.toml
[invocation.default-retry-policy]
initial-interval = "50ms"
exponentiation-factor = 2.0
max-attempts = 70
max-interval = "60s"
on-max-attempts = "pause"
```

### Other notable server defaults (from server config reference)

| Key | Default | Purpose |
|---|---|---|
| `default-journal-retention` | `"1d"` | Journal retention for invocations without explicit config |
| `default-idempotency-retention` | `"1d"` | Idempotency key retention |
| `default-workflow-completion-retention` | `"1d"` | Workflow result retention |
| `tracing-filter` | `"info"` | OTLP tracing filter |
| `log-filter` | `"warn,restate=info"` | Log filter |
| `disable-prometheus` | `false` | Prometheus metrics endpoint at port 5122 |

---

## 8. Observability

### Prometheus metrics

**Endpoint:** `http://<node-advertised-address>:5122/metrics`

Metrics are in Prometheus exposition format. The repo's config does not set `disable-prometheus = true`, so metrics are available.

Key metrics:
- `restate_ingress_requests_total` — ingress request count by state
- `restate_ingress_request_duration_seconds` — ingress latency summary
- `restate_rocksdb_estimate_live_data_size_bytes` — RocksDB size gauge
- `restate_invoker_invocation_task_total` — invocation tasks to handlers

Grafana dashboards: **Restate: Overview** (ID 24747), **Restate: Internals** (ID 24748).

**Source:** [Metrics docs](https://docs.restate.dev/server/monitoring/metrics.md)

### OTLP Tracing

**Config keys:**

```toml restate.toml
tracing-endpoint = "http://jaeger:4317"           # OTLP/gRPC endpoint
tracing-runtime-endpoint = "..."                  # Override for runtime traces
tracing-services-endpoint = "..."                  # Override for service traces
tracing-filter = "info"                            # Span/event filter
tracing-headers = { authorization = "Bearer ..." } # Auth headers
```

**Spans emitted per invocation:**

| Span | When emitted |
|---|---|
| `ingress <target>` | HTTP request received by ingress |
| `invocation-start <target>` | Invocation starts (anchor span) |
| `invocation-attempt <target>` | One per attempt (marked error if retryable failure) |
| `invocation-end <target>` | Invocation completed (success/failure) |

Spans are exported **as they end**, not at invocation completion — so you can inspect running invocations in real time.

**Events within attempt spans:**
- `restate.invocation.lifecycle.new_command` — journal command created
- `restate.invocation.lifecycle.run_ended` — `ctx.run` block finished
- `restate.invocation.lifecycle.suspended` — invocation suspended
- `restate.invocation.lifecycle.yielded` — invocation yielded execution

**Span attributes:**
- `restate.invocation.id` — invocation ID
- `restate.invocation.target` — target (e.g. `Census/status`)
- `rpc.service` / `rpc.method` — service and handler names
- `restate.deployment.id` — deployment ID
- `restate.invocation.result` — "success" or "failure"

To correlate: copy an invocation ID from the SQL introspection query and search for it in your tracing system via the `restate.invocation.id` attribute.

**Source:** [Tracing docs](https://docs.restate.dev/server/monitoring/tracing.md)

### Tokiko-console

The repo already wires `console-subscriber`. The server listens on port **6669** (default) for tokio-console connections.

```toml restate.toml
tokio-console-bind-address = "[::]:6669"
```

This is separate from OTLP tracing — tokio-console gives you per-task scheduling details (which Restate's OTLP spans do not).

### Logging

```toml restate.toml
log-filter = "warn,restate=info"
log-format = "pretty"   # "pretty" | "compact" | "json"
```

The `log-filter` follows the `tracing-subscriber` EnvFilter syntax. Override via `RUST_LOG` environment variable.

**Source:** [Logging docs](https://docs.restate.dev/server/monitoring/logging.md)

---

## Operator Commands Summary (Wave-C/D)

Ten commands the ops team will use daily:

```shell
# 1. Check cluster health
restatectl status --addresses http://127.0.0.1:5122/

# 2. Check partition status (are all partitions caught up?)
restatectl partitions list --addresses http://127.0.0.1:5122/

# 3. Query invocations for a specific service
restate sql --json "SELECT id, target, status, retry_count, modified_at FROM sys_invocation WHERE target_service_name = 'NationalCensus' ORDER BY modified_at DESC;"

# 4. Inspect the journal of a single invocation
restate sql --json "SELECT index, entry_type, name, completed, entry_json FROM sys_journal WHERE id = 'INVOCATION_ID' ORDER BY index;"

# 5. Find a blocked virtual object
restate sql --json "SELECT invocation_id FROM sys_keyed_service_status WHERE service_name = 'Ingest' AND service_key = 'mshsl';"

# 6. Check running deployments
restate deployments list

# 7. Cancel/kill a stuck invocation
restate invocations kill <INVOCATION_ID>

# 8. Pause all invocations of a service (maintenance)
restate invocations kill Ingest

# 9. Register a new deployment
restate deployments register --force http://127.0.0.1:9080

# 10. Check Prometheus metrics
curl -s http://127.0.0.1:5122/metrics | grep restate_ingress_requests_total
```

### Exact command for one invocation's journal

```shell
# Via CLI (human-readable)
restate invocations describe <INVOCATION_ID>

# Via SQL (full journal with entry details)
restate sql --json "SELECT * FROM sys_journal WHERE id = '<INVOCATION_ID>' ORDER BY index;"

# Via direct HTTP (for automation)
curl http://127.0.0.1:9070/query --json '{"query": "SELECT index, entry_type, name, completed, invoked_target, entry_json FROM sys_journal WHERE id = '\''<INVOCATION_ID>'\'' ORDER BY index;"}'
```

---

## Appendix: Port Reference (this repo)

| Service | Port | Config key |
|---|---|---|
| Ingress (HTTP calls) | `18095` | `[ingress].bind-port` |
| Admin API / UI | `19095` | `[admin].bind-port` |
| Node fabric (RPC) | `15152` | `bind-port` (root) |
| Prometheus metrics | `5122` | (hardcoded, from `[admin]`-bound NodeCtl) |
| Tokio-console | `6669` | `tokio-console-bind-address` |

---

*Document fetched 2026-09-22 from docs.restate.dev. SDK version: 0.12.0 (maps to server 1.7.x). All endpoint paths, config keys, and CLI flags quoted from documentation.*
