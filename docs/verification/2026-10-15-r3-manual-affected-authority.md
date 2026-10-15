# R3 manual affected-record authority

## Scope and status

This repair binds the affected-record audit inventory to retained, original snapshot authority. It does not acquire sources, adjudicate real identities, replace accepted decisions, or manufacture `AppliedAthleteIdentity` rows. Source and reviewed identity operations retain their existing context-v1 and witness formats. This document records the contract and authored regressions; compilation and native test execution remain required on the selected compute environment.

## Versioned manual authority

The current prepared payload is `LocalIdentityAdjudicationRecordV2`, with `version = 2` and policy `local-operator-retained-identity-review/v2`. In addition to the existing identity `authority_context`, it requires an `affected_context` content digest. The current manual commit witness is version 5. Manual witness 4, record version 1, old policy, and a missing affected-context field are refused rather than reinterpreted. Existing source witnesses 1/2 and reviewed witness 3 keep their existing replay paths. The terminal approval envelope remains version 1 because its exact-byte approval protocol has not changed; its prepared payload carries the new explicit schema.

## Affected-key contract

Keys have the grammar `<table>:<entity-id>`. The separator is the first colon; the rest is the exact retained logical entity ID. IDs must be nonempty, at most the existing store ID limit of 512 bytes, and free of control characters. The only generated table prefixes are:

- `athletes`: every retained canonical candidate in the public identity being split.
- `source_observations`: each retained athlete observation whose exact provider namespace and provider subject ID matches an identity retained on any member athlete.
- `performances`: every logical performance with at least one physical raw variant whose typed athlete ID is one of those members.
- `relay_results`: every logical relay result with at least one physical raw variant containing a typed member athlete ID belonging to those members.

A name, matching school text, source URL, or relay display name never establishes ownership. Shared logical IDs are deduplicated only after all physical observations are considered. This includes member-owned variants hidden by a merged projection's retained scalar/ownership conflict. Unknown relay membership does not become a typed athlete assignment. Athlete keys ensure a legitimate result-free identity can still be corrected.

The derived keys are sorted, unique and complete under this explicit contract. Request keys must equal that set exactly. Nonexistent keys, existing unrelated keys, missing member/result/observation keys, duplicates and noncanonical order are refused. The prepare check precedes context staging. The exact same comparison runs during approval consumption and retained historical verification.

## Manual-only retained object

The existing content-addressed journal context machinery stores a separate manual object. It does not change the identity-context schema or make source births scan results.

The first journal record has key/kind `header` and value:

- `schema`: `local-identity-affected-records/v1`
- `store`: original store identity
- `authority_context`: exact identity context digest from the same pinned snapshot
- `evidence_generation`, `derived_generation`, `snapshot_sequence`: the original pinned read view
- `counts`: `[physical_performance_rows, physical_relay_rows]`

The next records are the complete physical performance inventory, then complete physical relay inventory, including unrelated rows. Keys are `performance:<20-digit ordinal>` and `relay:<20-digit ordinal>`. Each value contains `physical_key_base64` and `raw_json`: canonical base64 of the exact Fjall observation key, and the exact original UTF-8 JSON bytes. Physical order and duplicates under one logical entity ID are preserved. Replay requires strictly increasing physical keys within each table and verifies physical table/id against the decoded entity using the existing shared physical-observation decoder.

Both inventories are captured through `StoreSnapshot::for_each_physical_observation_bounded` from the same snapshot used for the identity context. Completeness is established by that bounded full physical scan, not an operator-supplied list. Every scan must finish; exhaustion refuses, never truncates. Original raw bytes, keys, context identity and revision are committed through the content digest referenced by the exact approved payload.

## Bounds and replay

No existing resource limit is raised. Each physical table scan admits at most 8,192 rows, 16 MiB of physical bytes including keys, and 1 MiB per original row. Retained records additionally fit the journal's 1 MiB record limit and each retained section fits 16 MiB after encoding. Keys admit at most 1,024 bytes before base64; normal entity keys remain subject to the existing 512-byte ID limit. The context object remains bounded by 32,769 records and 64 MiB. Affected logical keys remain limited to 1,024. Approval and manual witness bytes remain limited to 1 MiB. Encoding expansion can cause an earlier typed refusal; it does not enlarge any grant.

Frozen capture includes both roots referenced by each manual witness. Frozen replay requires exactly the referenced context inventory, verifies the affected object digest, validates original store/context/revision binding, and derives the exact key set from retained bytes. It does not query current result tables. Later live result growth therefore does not alter the historic correction's affected set. Head/revision fencing, exact approval-envelope comparison, original retained claim disposition validation, superseded-merge binding and atomic history/witness/receipt commit remain in force.

The frozen bundle retains its existing cap of 4,096 distinct context objects and its existing aggregate byte limit. A manual operation may require two distinct roots. Consequently a manual-heavy history can exhaust frozen-context capacity before reaching the 4,096-operation history limit; capture and replay then refuse without truncation or reinterpretation. This deliberate bounded-capacity consequence is not an assertion that every admissible history fits a frozen bundle. No cap is increased by this repair.

## Repeated preparation storage dependency

Inspection only measures context objects and does not write journal rows. Preparation stages durable context objects before approval. The affected header binds the original Fjall snapshot sequence; staging itself advances that sequence. Consequently identical logical requests prepared repeatedly can retain distinct affected objects. The existing per-object limits bound one preparation, but there is no aggregate lifetime staging quota or garbage collection in this repair. Repeated preparation therefore has unbounded lifetime storage growth. A bounded admission/reuse policy using the existing journal lifecycle is an explicit production acceptance dependency, not closed by the per-object limits. No idempotent-prepare claim is made.

The existing consume fence checks store origin, evidence/derived generations and exact registry head. Staging only journal objects does not move those entity revisions or the head. Original snapshot sequence is validated against the retained affected header and approved record; it is deliberately not compared with the post-staging live sequence. Exact approval bytes, evidence changes and head changes remain fenced.

## Authored regressions

- Genuine reviewed merge and approved manual split with actual member and unrelated performance/relay records.
- Raw conflicting variants whose merged result still points only to an unrelated athlete; complete physical inventory includes the member-owned variant.
- Exact sorted set, refusal of existing unrelated and nonexistent keys for all four table kinds, and refusal after individually omitting every expected key.
- Refusals precede all store writes; successful corrections preserve every original raw fact.
- Original header/revision and full raw context survive later live results, reopen, operation replay, and offline frozen replay after the store is dropped.
- Old manual schemas/witnesses refuse; missing or tampered affected context refuses even after rehashing the outer frozen bundle.
- Retained row/count limits precede JSON parsing; section count and raw/key byte limits refuse malformed or oversized inputs.

## Verification boundary

The canonical formatting task was executed after verifying Moon's project root was this isolated worktree. The final whole-tree formatting check reports only pre-existing incoming formatting in `census-domain/src/school_index.rs`; that unrelated change is excluded from this patch. No compilation, test execution, heavy build, live acquisition, or real-store adjudication was run. Native focused tests, all-target compilation, trusted AST size scan, source lint and release integration remain required. The shared toolchain environment provides `bd`, but its Dolt authority reconciliation is blocked by the read-only `/home/agent/.dolt` location in this executor.
