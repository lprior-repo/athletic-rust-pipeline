# Capture protocol API freeze

This is the detached implementation target, staged for Root to integrate with the real service/transport consumer. Historical names, keys and wire bytes are unchanged.

## Domain exports

`census_domain::model` re-exports the exact existing `HistoryWindow` and `HistoryError` moved by Root, plus the new source-acquisition values:

- `CaptureQualificationV1`, `AcquisitionDigest`, `AcquisitionSlotId`, `AcquisitionWorkId`, `AttemptOrdinal`, `HopOrdinal`, `AcquisitionAttemptId`, `AcquisitionDispatchId`, `AcquisitionOccurrenceRefV1`.
- `ResultsCaptureScopeV1::new(owner_key: &str, jurisdiction: UsJurisdiction, run: CensusRun, run_manifest_digest: AcquisitionDigest, history: HistoryWindow, historical_year: u16, source_admission: SourceAdmissionV1, cache_intent: CaptureCacheIntent) -> Result<Self, CaptureDomainError>`.
- `SourceAdmissionV1::milesplit_results(plan_fingerprint: &str) -> Result<Self, CaptureDomainError>`.
- `CaptureCacheIntent::{ReuseEligible, AcquireNew}`.
- `RequestRepresentationV1::get(original_url: &str, representation_headers: Vec<(String, String)>, effective_user_agent: &str) -> Result<Self, CaptureDomainError>`. The caller passes its existing canonical representation headers and the actual client UA. Domain bounds/syntax checks do not replace the existing URL/source/redirect/DNS/pacing checks.
- `AcquisitionWorkV1` and `AcquisitionSlotV1` have private fields, checked decoding and exact adopted field order/digests. Public accessors expose every field, `slot.id()` and `work.id()`.
- `DispatchRequestV1::new(target_url: &str, conditional: ConditionalRequestV1) -> Result<Self, CaptureDomainError>`, with `ConditionalRequestV1::{Unconditional, Pinned { original: AcquisitionOccurrenceRefV1, etag: Option<String>, last_modified: Option<String> }}`. Pinned validators belong to the dispatch, not work identity.
- `ResponseObservationV1::new(status: u16, response_url: &str, observed_at: &str, headers: Vec<(String, String)>) -> Result<Self, CaptureDomainError>`; exact observed response checkpoint.
- `PreparedCaptureInputV1::new(content_digest: &str, content_bytes: u64, metadata_bytes: Vec<u8>) -> Result<Self, CaptureDomainError>` binds the exact existing serialized CacheMeta bytes and their digest. Existing archive paths are derived from the two full digests. The Store verifies the original files itself; no caller-provided boolean certifies retention.
- `AcquisitionFailureV1` and `InterruptionCauseV1` are explicit closed enums. No arbitrary error text is identity or scheduling authority.

## Store lifecycle

`Store::create_capture_qualification_v1(root: impl AsRef<Path>) -> StoreResult<CaptureQualificationV1>`.
`Store::capture_qualification_control_existing(root: impl AsRef<Path>) -> StoreResult<CaptureQualificationControl>`; `show(&self) -> StoreResult<CaptureQualificationV1>`.
`Store::open_capture_qualification_v1(root: impl AsRef<Path>) -> StoreResult<CaptureQualificationStore>`; `into_parts(self) -> (Store, CaptureWriterV1)`.
`CaptureWriterV1::store_identity(&self) -> &StoreIdentity`.

Only the explicit opener produces the private nonserialized writer. It binds to this Store's existing appends Arc by pointer identity and retained StoreIdentity, not a parsed declaration or a filesystem spelling. Ordinary open/control/migrate/inspect refuse lifecycle4. Backup/restore validate and preserve it without issuing a writer.

## Service owner and one continuation

`CaptureWriterV1::results_owner(&self, store: &Store, scope: ResultsCaptureScopeV1) -> StoreResult<ResultsCaptureOwnerV1>`.
`ResultsCaptureOwnerV1::for_owned_meet(&self, site: UsJurisdiction, meet_id: &str, request: RequestRepresentationV1) -> StoreResult<AcquisitionWorkV1>`.
`CaptureWriterV1::continuation_for_owned_meet(&self, store: &Store, owner: &ResultsCaptureOwnerV1, site: UsJurisdiction, meet_id: &str) -> StoreResult<RetryOwnerGrant>`. The grant binds the exact slot; no unbound continuation method exists.
`ResultsCaptureOwnerV1::slot_for_owned_meet(&self, site: UsJurisdiction, meet_id: &str) -> StoreResult<AcquisitionSlotV1>`.
`ResultsCaptureOwnerV1::scope(&self) -> &ResultsCaptureScopeV1`.
`ResultsCaptureOwnerV1::require_cache_root(&self, store: &Store, cache_root: &Path) -> StoreResult<()>` verifies actual canonical root equality with Store/http and runtime origin before grant consumption.

`ResultsCaptureOwnerV1` can be cloned for immutable scoped plumbing. `RetryOwnerGrant` cannot be cloned or deserialized and is consumed by registration. Only the service retains `CaptureWriterV1`; the transport gets owner+one grant or already registered work. Service verifies actual ctx.key against the unchanged WorkflowIdentity, retained state/plan selection and exact HistoryWindow before calling results_owner. Store independently verifies retained manifest/digest/run/cohort/jurisdiction/lineage and owner/work equality.

Service acquires its existing WorkAdmission guard keyed by the approved unchanged owning WorkflowIdentity before registration/recovery and moves that guard into the supervised worker beside the non-cloneable Store token. It remains held through request drain/finalization. No Store-to-service dependency or duplicate global lock registry is added. Every derived token owns the original work token. A caller merely disappearing is not a fence while the worker retains the service guard.

## Store protocol

All methods return `StoreResult<...>`:

- `register_acquisition_work(&self, owner: &ResultsCaptureOwnerV1, work: AcquisitionWorkV1, grant: RetryOwnerGrant) -> OwnedAcquisitionWork`.
- `inspect_acquisition_work(&self, work: &OwnedAcquisitionWork, budget: &mut AcquisitionReadBudget) -> AcquireDecision`.
- `reuse_acquisition_cache(&self, work: OwnedAcquisitionWork, original: &AcquisitionOccurrenceRefV1, budget: &mut AcquisitionReadBudget) -> VerifiedRetainedOccurrence` requires ReuseEligible, no armed facts, exact equal request representation and an original reusable200 ResponseRetained. It writes only the immutable current work/result reference, without a new attempt/occurrence/date.
- `require_legacy_structured_capture(&self) -> ()` refuses lifecycle4 LegacyMetadata downgrade before HTTP, without reopening.
- `reserve_acquisition_attempt(&self, work: OwnedAcquisitionWork) -> ReservedAttempt`; handles absent or safely owner-fenced unarmed reservation, never a terminal-success/new ordinal replay.
- `arm_acquisition_dispatch(&self, attempt: ReservedAttempt, request: DispatchRequestV1) -> ArmedDispatch`.
- `ArmedDispatch::dispatch_id(&self) -> &AcquisitionDispatchId`; `ArmedDispatch::into_observation(self) -> DispatchObservation` consumes one local invocation authorization immediately before the actual one-shot execute.
- `record_acquisition_response(&self, dispatch: DispatchObservation, response: ResponseObservationV1) -> ObservedResponse`.
- `arm_acquisition_redirect(&self, response: ObservedResponse, request: DispatchRequestV1) -> ArmedDispatch` validates previous actual redirect and next hop <=4. Existing transport admission remains required before each arm.
- `prepare_acquisition_capture(&self, response: ObservedResponse, input: PreparedCaptureInputV1, disposition: CaptureDispositionV1) -> PreparedCapture` before existing immutable archive publication.
- `finish_acquisition_capture(&self, prepared: PreparedCapture) -> AcquisitionOccurrenceRefV1` verifies backing bytes/metadata and creates terminal+receipt+work result atomically.
- `finish_prepared_acquisition(&self, work: OwnedAcquisitionWork, budget: &mut AcquisitionReadBudget) -> AcquisitionOccurrenceRefV1` performs zero-network recovery of the original prepared attempt from exact archive backing.
- `fail_acquisition_dispatch(&self, dispatch: DispatchObservation, failure: AcquisitionFailureV1) -> ImmutableAttemptResult`.
- `refuse_acquisition_response_checkpoint(&self, dispatch: DispatchObservation, actual_status: u16, actual_response_url: &str, reason: ResponseCheckpointRefusalV1) -> ImmutableAttemptResult` records only a genuine actual response whose full checkpoint cannot encode. Closed reason is UnrepresentableResponse. It validates the bounded actual final URL against the admitted dispatch target and retains that exact URL with checked100..599 status inside ResponseCheckpointRefused failure at the last Armed checkpoint. Generic dispatch failure accepts Transport/Timeout only and records OutcomeUnknown.
- `fail_acquisition_response(&self, response: ObservedResponse, failure: AcquisitionFailureV1) -> ImmutableAttemptResult`.
- `refuse_acquisition_attempt(&self, attempt: ReservedAttempt, failure: AcquisitionFailureV1) -> ImmutableAttemptResult` yields NotDispatched.
- `recover_acquisition_attempt(&self, work: OwnedAcquisitionWork, cause: InterruptionCauseV1) -> ImmutableAttemptResult` is legal only after the service gate is acquired and authoritative prepared recovery attempted. It refuses Prepared and seals incomplete armed/observed truth as OutcomeUnknown.
- `finish_acquisition_not_modified(&self, response: ObservedResponse, original: &AcquisitionOccurrenceRefV1, budget: &mut AcquisitionReadBudget) -> AcquisitionOccurrenceRefV1` requires actual304, exact dispatch pinned original, verified original body, and original cache compare fence retained by caller. The reference names the new revalidation terminal while its terminal points to the unchanged original occurrence/body/date.
- `read_retained_acquisition_v1(&self, reference: &AcquisitionOccurrenceRefV1, budget: &mut AcquisitionReadBudget) -> VerifiedRetainedOccurrence` uses one pinned Store snapshot and complete closure/backing verification. Getters expose `reference()`, `body_reference()`, `prepared()`, `response()` (original body response), `revalidation()` (actual304 only) and `body_path()`; it grants no dispatch rights.

`AcquireDecision::{Ready, ResumeReserved(AcquisitionAttemptId), FinishPrepared(AcquisitionAttemptId), Replay(AcquisitionOccurrenceRefV1), RetryOwed(ImmutableAttemptResult), RecoveryRequired(AcquisitionAttemptId), Settled(ImmutableAttemptResult), Exhausted, Interrupted}`. It is a checked read result, not an I/O capability. At most three persistent attempts can ever occupy one original slot; National pass/retry/restart cannot reset this.

`CaptureDispositionV1::{ReusableResponse, RetryableResponse, TerminalResponse}` is persisted in Prepared and ResponseRetained. `for_http_status(status)` is the shared closed taxonomy:200 reusable,429/5xx retryable, other non304 responses terminal. Store validates it. A retained retryable response has an immutable occurrence/receipt but no settled work/result; it permits only the next original-owner grant within the same3. Cache reuse and304 original admit reusable200 ResponseRetained only.

`AcquisitionReadBudget::for_work()` allows84 journal roots for current+at-most-one body-original work,90 checkpoint/receipt envelopes and one32MiB body plus metadata.304 originals cannot reference revalidations; there is no recursive traversal. Every separate Store operation gets its own budget because it pins/rechecks a separate snapshot; all reads within an operation draw from the same mutable budget. `new(max_roots: usize, max_encoded_bytes: usize, max_backing_bytes: u64)` constructs a shared consumable budget. Callers must pass the same mutable budget through joined closure; nested methods never reset it.

## File map

- domain `model/source_acquisition.rs` and `source_acquisition/{error,scalar,qualification,identity,request,work,protocol}.rs`, Root's moved `model/history.rs`, registrations in model.rs.
- Store `acquisition.rs` and `acquisition/{error,capability,qualification,init,codec,facts,read,transition,terminal,backing,budget}.rs`, API-focused tests.
- Store existing `lib.rs,error.rs,format.rs,format/existing.rs,format/migrate.rs,lineage.rs,backup/manifest.rs,run.rs,write.rs,receipt.rs,receipt/credit.rs,receipt/frontier.rs,write_batch/journal.rs,backup/http_cache.rs,backup/http_cache/metadata.rs,format/open.rs` for shared guards and staging. No service/crawl caller source edits in this proposal.

## Root-owned integration requirements

Root applies the exact service HistoryWindow re-export; this core patch includes the existing chrono serde feature for standalone domain builds. Root adds the explicit CLI/init and serve enum and passes the runtime writer into actual consumer service ownership. Crawl owns existing cache envelope v2 parsing/publication and guarantees the archive/cache compare fence around304 and cache publication. Store backup validates protocol/receipt/archive closure and the strict cache v2 original occurrence/metadata/body relation with that same core verifier. Historical v1 classification is preserved. Nothing here certifies all roles or reusable production.


## Current operational bounds and qualified-recovery exclusions

Inventory/open/backup/restore use one shared bound of1,048,576 journal scan roots,512MiB encoded reads,2GiB unique backing bytes and64MiB retained verification memo accounting. Exact immutable archive addresses plus digest/length deduplicate backing reads only within the same operation/pinned view. Public reusable budgets never carry verified-file authority across operations; counters still draw down. Larger configured archives are a typed capacity refusal and unresolved scale acceptance, not corruption or qualified production scale.

A retained occurrence verifies all logical roots/receipts of its current work and at most one original work, then verifies the selected body's backing. Inventory verifies every retained terminal backing, including earlier retryable refusal bodies. Journal roots are42 per work; the shared84 bound is journal roots only. Up to6 receipts draw from the encoded budget, whose default derives90×(65,536+4,096) bytes including conservative key allowance.

Prepared with missing/corrupt original body or metadata is explicitly fail-closed: finish_prepared refuses, recovery does not allocate a new attempt, and original IDs/facts remain. This is an unresolved durable-recovery acceptance gap; no full crash-recovery qualification claim is made. A future lost-backing terminal/retry policy requires explicit adoption.

Transient pre-arm DNS timeout may leave the immutable reservation unarmed. A later original-owner target-bound continuation resumes the same ordinal. Once Armed exists, this is refused and incomplete remote truth requires explicit recovery.

The stated acquisition inventory limits do not redefine the unchanged whole-backup HTTP/tree traversal policies. Declaration/lineage admission uses its own fixed bounded registry reads; the retained run manifest is pinned and debited once per inventory snapshot.
