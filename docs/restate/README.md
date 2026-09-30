# Restate documentation (local vendor reference)

Local copies of official Restate documentation pages, retrieved 2026-09-27, for operating this repository's Restate deployment.

**Everything in this directory is external vendor reference material (official Restate documentation from docs.restate.dev). It is not project policy, not a specification for this repository, and carries no authority over how this repository is built or configured.**

Deployment under study: a **native `restate-server` node** with loopback ingress `18095` / admin `19095`, serving a **`restate-sdk-rust/0.12.0`** service endpoint. Pages below use the vendor defaults (`8080` ingress, `9070` admin) in their examples; substitute the loopback ports of this node.

## Files

| File | Description | Source URL |
| --- | --- | --- |
| [index.md](./index.md) | Restate documentation home / product overview: capabilities, use cases, entry points, SDK list | https://docs.restate.dev/ |
| [server-configuration.md](./server-configuration.md) | Server configuration: `restate.toml`, `--config-file`/`RESTATE_CONFIG`, layer order (defaults → file → env → CLI), env-var naming, `--dump-config`, `SIGUSR1` live-config dump | https://docs.restate.dev/server/configuration (requested as https://docs.restate.dev/operate/configuration/server) |
| [registration.md](./registration.md) | Deployments, registration and versioning: `restate deployments register/list/describe/remove`, `--force`/`--breaking`/`--use-http1.1`, journal mismatch (RT0016), pause+resume on a new deployment | https://docs.restate.dev/services/versioning (requested as https://docs.restate.dev/operate/registration) |
| [invocation.md](./invocation.md) | Invocation lifecycle: invocation IDs and statuses, cancel, kill, pause, resume (optionally on another deployment), purge, restart-as-new, restart-from-prefix | https://docs.restate.dev/services/invocation/managing-invocations (requested as https://docs.restate.dev/operate/invocation) |
| [introspection.md](./introspection.md) | Introspection: `sys_invocation`/`sys_journal`/`sys_keyed_service_status`/`state` queries via CLI and `POST /query`, state inspection and editing (`restate kv get/edit`) | https://docs.restate.dev/services/introspection (requested as https://docs.restate.dev/operate/introspection) |
| [inspecting.md](./inspecting.md) | SQL Introspection API table reference: `sys_invocation`, `sys_service`, `sys_deployment`, `sys_journal`, `sys_inbox`, `sys_keyed_service_status`, `sys_promise`, `state` column schemas and types | https://docs.restate.dev/references/sql-introspection (requested as https://docs.restate.dev/operate/inspecting) |
| [deploy-services.md](./deploy-services.md) | Deploying services/endpoints: containers and VMs, request-identity (ED25519) keys, registering the endpoint, load balancers and HTTP/2 (`grpc_pass`, `http2 on;`) | https://docs.restate.dev/services/deploy/standalone (requested as https://docs.restate.dev/deploy/services) |
| [durable-execution.md](./durable-execution.md) | Key concepts: application structure, durable execution via journals and replay, failure coverage, resilient RPC, consistent state, FaaS suspension | https://docs.restate.dev/foundations/key-concepts (requested as https://docs.restate.dev/concepts/durable_execution) |
| [services.md](./services.md) | The three service types (basic service, virtual object, workflow), their state/concurrency/lifecycle properties, and deployments/endpoints/versions | https://docs.restate.dev/foundations/services (requested as https://docs.restate.dev/concepts/services) |
| [restate-server.md](./restate-server.md) | Self-hosted server overview: single binary, prerequisites, persistent volumes, `restate-data`, single-node vs cluster trade-offs, what the server stores | https://docs.restate.dev/server/overview (requested as https://docs.restate.dev/develop/restate-server) |
| [invocation-http.md](./invocation-http.md) | HTTP ingress reference: `/restate/call`, `/restate/send` (delay), idempotency keys, `x-restate-error-source` retry semantics, attach/output/lookup, OpenAPI export | https://docs.restate.dev/services/invocation/http |
| [networking.md](./networking.md) | Networking: ingress/admin/fabric default ports, `listen-mode`, unix sockets, per-service `bind-port`/`bind-address`, advertised addresses, message-size limits | https://docs.restate.dev/server/networking |
| [error-codes.md](./error-codes.md) | Error code reference, including META0003–META0017 (registration/discovery) and RT0001–RT0025, notably RT0016 journal mismatch on replay | https://docs.restate.dev/references/errors |
| [architecture.md](./architecture.md) | Architecture reference: ingress, durable log, partition processor, control plane, write path and step lifecycle, failover, and node roles (`metadata-server`, `log-server`, `worker`, `http-ingress`) | https://docs.restate.dev/references/architecture |

## Requested-path resolution

The vendor docs were restructured: the historical `/operate/*`, `/concepts/*` and `/deploy/*` prefixes no longer exist. What was requested for this directory and what was retrieved:

| Requested URL | Result | Local file |
| --- | --- | --- |
| `https://docs.restate.dev/` | 200 | [index.md](./index.md) |
| `https://docs.restate.dev/operate/configuration/server` | 301 → `/server/configuration` | [server-configuration.md](./server-configuration.md) |
| `https://docs.restate.dev/operate/registration` | 301 → `/services/versioning` | [registration.md](./registration.md) |
| `https://docs.restate.dev/operate/invocation` | 301 → `/services/invocation/http` (lifecycle material lives on `/services/invocation/managing-invocations`) | [invocation.md](./invocation.md), plus the redirect target in [invocation-http.md](./invocation-http.md) |
| `https://docs.restate.dev/operate/introspection` | 301 → `/services/introspection` | [introspection.md](./introspection.md) |
| `https://docs.restate.dev/operate/inspecting` | **HTTP 404** (no redirect) | current equivalent: [inspecting.md](./inspecting.md) (`/references/sql-introspection`) |
| `https://docs.restate.dev/deploy/services` | **HTTP 404** (no redirect) | current equivalent: [deploy-services.md](./deploy-services.md) (`/services/deploy/standalone`) |
| `https://docs.restate.dev/concepts/durable_execution` | redirect → `/foundations/key-concepts#durable-execution` | [durable-execution.md](./durable-execution.md) |
| `https://docs.restate.dev/concepts/services` | redirect → `/foundations/services` | [services.md](./services.md) |
| `https://docs.restate.dev/develop/restate-server` | **HTTP 404** (no redirect) | current equivalents: [restate-server.md](./restate-server.md) (`/server/overview`), [networking.md](./networking.md) (`/server/networking`), [architecture.md](./architecture.md) (`/references/architecture`) |

## Fidelity notes

Each file starts with a header stating its source URL(s), the retrieval date (2026-09-27), the redirect/404 status where relevant, and the statement that it is external vendor reference material and not project policy. Page bodies are reproduced from the vendor's markdown alternates; Mintlify component wrappers (`<Frame>`, `<Card>`, `<Steps>`, `<Accordion>`) are kept as-is for fidelity, with image URLs preserved. Where a page was too large or contained material irrelevant to this deployment, each file's header lists exactly what was omitted (or the local-copy note inside the file does). Not copied locally: the full server configuration option reference (`/references/server-config`), SDK-language pages other than the snippet variants noted, and Restate Cloud/BYOC material.
