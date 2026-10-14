# Capture-to-owned-parser envelope contract

Status: detached implementation proposal, source-reviewed only. No build, formatting, test, browser, census, or publication command was run by this worker.

## Scope and ownership

Root adopted `structural-open-consumer-review/PARSER-ENVELOPE.md` and the verifier reconfirmed that contract before implementation. This proposal seals the runtime `CapturedFetch` and `OwnedMeetOutcome` together. It does not adopt structural-open behavior, Prepared recovery v2, installation/schema changes, source-family extensions, or a new persisted version. Root alone edits the live repository and runs verification.

`net::acquisition` owns the transport envelope. `milesplit::owned::outcome` owns the interpreted envelope. Existing transport sessions and checked retained readers remain the only retained producers. Existing resource-retention and journal implementations remain responsible for their original effects.

## Typed model and transitions

- `CapturedFetch` has private capture and provenance fields. The retained state is a private enum inside an opaque crate-private `CaptureProvenance`, with no Clone, Default, Deserialize, caller-accessible retained constructor, or mutable getter.
- Public `CaptureProvenanceView<'_>` borrows the occurrence and original references. Constructing or copying this descriptive view confers no authority: no envelope construction/promotion API accepts it.
- `CapturedFetch::capture` and `provenance` borrow without changing the envelope. `into_capture` consumes and erases its proof. The narrowly crate-private `into_parser_parts` moves raw capture and opaque provenance to the parser owner; there is no corresponding parts builder.
- `CapturedFetch::historical` is crate-private and always creates HistoricalAttemptUnknown. The production legacy adapter still calls `require_legacy_structured_capture` before fetching and wrapping the raw capture.
- `OwnedMeetOutcome::interpret` is restricted to the MileSplit implementation. It consumes one intact `CapturedFetch`, an already-parsed meet ID, and expected `CaptureContext`. It checks historical/durable compatibility before parser invocation. It derives Parsed/Malformed/Refused from that capture's own immutable body/status, without accepting any supplied verdict or retained refs.
- Resource failure returns `OwnedInterpretationOutcome::Resource { capture: CapturedFetch, failure: OwnedParseResourceFailure }`. The original envelope and Vec allocation survive until the caller deliberately erases authority for existing error retention.
- Successful construction moves the same raw capture and opaque provenance to private fields of `OwnedMeetOutcome`. Public accessors return immutable capture/verdict and borrowed provenance description. A consuming `into_capture` erases interpretation/provenance authority.
- MileSplit-restricted `release_body` drops only the resident Vec. Capture bytes count, digest, original URL, final URL, actual date, other metadata, and verdict stay unchanged. The type remains an interpretation of the checked bytes, not a currently resident verified transport input. There is no conversion from this outcome back to retained `CapturedFetch`.
- `AcquiredMeet` delegates its existing context guard and release boundary to those owner methods rather than duplicating admission or exposing mutable internals.
- Only `#[cfg(test)] historical_fixture` accepts a synthetic verdict. It can select no durable provenance, is scoped to MileSplit unit-test callers, and has no production feature escape.

## Error and effect ordering

The one new refusal is the already-existing `CaptureTransportError::CacheOccurrenceMismatch`, moved to the earliest owning interpretation boundary for mismatched context. It occurs before parsing or interpretation/archive journal effects. Existing post-acquisition transport status acceptance, parser malformed formatting, resource origin, archive path validation, recording limits, receipts, and nonretryable fetch failure recording remain unchanged.

The two production entrypoints both call the same interpreter. `fetch_owned_meet` retains a resource error through `retained_parse_resource` with `NotRequested`, as before. `read_owned_meet` uses `retain_limited_capture`, preserving `Retained` versus `ReportOnly` behavior, and consumes interpreted capture only for its existing recording-resource failure branch.

## Exact-context limitation retained by Root direction

The interpreter's guard checks historical-versus-durable provenance compatibility only. Internally, a retained envelope from context A can be passed to the interpreter with another DurableV1 context B; this guard does not certify exact owner, work, store, or target equality. Root explicitly retained this variant-only contract for the persistence-neutral slice after review. Exact current owner/work binding comes from the two immediate production acquisition-to-interpretation caller chains, which pass the same AdapterContext throughout acquisition and interpretation. The private Run is constructed and used with one immutable context; review found no current production context-switch path. Public callers cannot inject a CapturedFetch into the restricted interpreter. Do not describe the provenance view or this guard as an exact-context witness. A future externally injectable or cross-context interpreter would require its own explicit binding contract.

## Persistence and compatibility

No change to `FetchOutcome`, its serde implementation, cache metadata, eligible envelope version, acquisition records, journal phase/key/payload definitions, capture manifests/chunks, normalized rows, terminal records, projection receipts, policy IDs, or frozen schema IDs. Getter expressions serialize the same underlying values under the same keys. Older synthetic v3 parser fixtures remain visibly historical and continue seeding their existing old phases.

This intentionally tightens the Rust source API. Direct field access, public retained enum construction, and caller-assembled parsed outcomes stop compiling. All discovered current consumers are migrated in this slice or in the explicit Root-owned shared-file deltas. No runtime or persisted backward-compatibility shim is introduced.

## Resource and concurrency constraints

No additional body clone, shared lock, async task, retry, new allocation policy, parser budget, or persistence operation is introduced. Parsing runs at the existing synchronous boundary after acquisition. New typed variants move current ownership rather than allocating a second body. Actual allocation/performance and compiler evidence remains Root-owned; no speed or global readiness claim is made.

## Reference inputs

Read repository AGENTS.md and ARCHITECTURE.md; recovered skills rust-contract, holzman-rust, async-rust-reviewer, scott-ddd-refactor, test-planner and test-reviewer; Holzmann references `nasa-jpl-standards.md`, `runtime-performance-architecture.md`, `zero-cost-abstractions.md`; testing reference `test-planner/references/testing-philosophy.md`. Repository Moon-only commands, zero-comment rules, and Root's no-build boundary override generic skill command examples.
