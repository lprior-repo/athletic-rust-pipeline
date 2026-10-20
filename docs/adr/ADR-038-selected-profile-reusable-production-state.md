# ADR-038 — Selected profiles over reusable production evidence

Status: accepted target architecture (2026-10-14), amended for a new-only runtime cutover
(2026-10-15). Implementation qualification and production enrollment remain pending.
Acceptance does not enroll a store, execute a census or migrate bytes.

## Context and decision boundary

The [October 11 specification v2](../specifications/athletic-recruiting-end-vision-spec-v2.md)
is the requested full product target: persistent public-source evidence, stable identities,
configurable cohorts and geography, incremental refresh/reparse, reversible decisions and one
consistent recruiter publication. Registering profile JSON alone does not satisfy that target.
Its first delivery is Class of 2027 in MI, MN, IA, IL and CO with source-backed Michigan UP
priority. The distinct long-term national profile remains 48 contiguous states plus D.C.

[ADR-009](ADR-009-census-run-scope.md) and
[ADR-013](ADR-013-fresh-national-source-census.md) define the historical 49-jurisdiction,
fresh-acquisition census. [ADR-026](ADR-026-derived-generations-and-store-schema.md) defines
schema refusal and generation publication, not an installation enrollment contract. Keep
those historical meanings, manifests, receipts, workflow keys, exports and seals intact.
This ADR changes the forward production-lifecycle restriction only; it does not reinterpret
any historical run as persistent production or certify imported evidence as fresh acquisition.

## 1. Explicit lifecycle contracts

Use distinct constrained domain alternatives, not a reuse boolean or an implicit active
profile. The following names describe intended contracts, not existing serialized variants:

- **FreshQualification:** an isolated fresh store, unused durable run namespace/revision,
  explicit public-source obligations and no imported population, observations, decisions,
  completion receipts or seals. The 2027/49-jurisdiction national obligations remain;
  execute them through the current engine, not a retained legacy runtime.
- **ReusableProduction:** one explicitly enrolled installation/store retains qualified public
  evidence, accepted decisions, unfinished work and stable entity IDs across synchronization
  sessions and profile projections. New acquisition is scheduled for due or missing evidence;
  eligible retained captures/interpretations are reused under their original provenance.

A request explicitly declares its supported version, lifecycle, installation/store lineage and
immutable query profile. Missing enrollment, unsupported execution or an unsupported format
refuses before work or mutation. Historical requests retain their recorded meaning, not a
compatibility execution path; missing fields never silently select a lifecycle.

Main owns the versioned enrollment and runtime contracts. Profile registration/selection,
store identity initialization, moving a directory, a new season and reopening a store do
not enroll it. Do not replace the current single-run binding with a mutable latest-run slot.
New persistent installation and synchronization records must have explicit versioned
contracts, separate from historical single-run records and logical keys, through the same
existing service/store ownership and acquisition plane.

### New-only runtime cutover — 2026-10-15

One current runtime serves both lifecycle scopes. Migrate every caller and remove superseded
execution paths, fallback decoders, historical-result variants, aliases and compatibility shims.
An old receipt cannot become a new witness or execution capability. Preserve original stores,
workflow journals, identities, captures, receipts and seals as history; preservation does not
authorize current execution, automatic import or reinterpretation. No migration is implied.
Qualify fresh runs against the new contracts, including same-invocation recovery and lost-ack
replay. Historical passing runs do not qualify the replacement. Section 7 inventories the
pre-cutover implementation; it is not a requirement to retain those paths.

### Current fresh-serving composition — 2026-10-19

Initialize graph-V3 identity and existing capture-V2 declarations in one batch with
one store identity, only in a new/empty root. Serving requires explicit
`fresh-qualification-v3`; the flag names graph qualification, not a new capture wire.
Identity-only and historical capture stores are not silently adopted for acquisition.
Graph/input admission and roster enrollment use their existing explicit owner
operations. Capability enrollment accepts no athlete or membership facts and is not
reusable-production activation. Historical decoding does not authorize old serving.

### First qualification kernel — 2026-10-17

Owner decision: qualify a distinctly versioned reviewed-roster kernel V2. Preserve
PR81's original V1 source checkpoint, artifacts and producer pins; retire that
unqualified predecessor from current issuance rather than claim V1 equivalence.
Unsupported predecessor evidence refuses without mutation. Layout-dependent
grant accounting belongs to the new qualification, not an inherited passing result.

The independently retained Report V3 artifact from producer `24f593911c6cab8f1679199fd70547a701fced27`
remains unchanged and requires separate historical replay acceptance. Read-only
frozen compatibility does not authorize old acquisition or issuance runtimes.
Fresh reviewed-roster A and changed-build B must carry their own authenticated
source, kernel and artifact identities. This decision supplies no passing receipt.

### Current implementation kernels and read-only replay

**Context:** structural repairs change the source identity of the immutable
reviewed-roster kernels. Their original dependency documents cannot be repinned.

**Decision:** current dependency epochs are 12 (full inventory) and 13 (row-local).
These identify implementation inputs, not qualification-profile or inventory-wire
versions. Registering the checked calendar-source producer changed the source
closure. Qualified 10/11 documents remain immutable; one current interpreter
recomputes original inputs under their authenticated provider contract.

The read-only adapter accepts only original whole-document SHA-256 pins:

| Original epoch | Kernel document SHA-256 |
|---|---|
| 2 | `71ef18570ab2683c9f498e2f66983d6a782900fd2647eff4101864b782ecaa46` |
| 3 | `7352af3bf097e5c91a890809b1fa52e700469b3bbccb308eb445fe49fcf48f38` |
| 4 | `435df869dd5994102f1aee7bebafb8f1cfc3afe5d99e8c2c60ae892be14b80d2` |
| 5 | `60fd1056573363a7985af568568db1f9b82f4b39d38f9e3efa3034beb3c63fe0` |
| 6 | `ba3d52bbc2673fc3f386815a39ed09702b5e2c68b528ca8eecf630d5f45ea8fd` |
| 7 | `b49bfad4374e7a664c08977bc62428e40e8c8e0063d33235796e5eb3b03c2f65` |
| 8, native context | `8a4c9f7f31199c9f856ed43f9905ed01741cfa506707c8cd472cc6032b331b6f` |
| 9, native context | `94ad0cb66c947e1fdc1efd31cbd99c7d1aac3bbc580bf1e81debfd16bf188505` |
| 8, alternate context | `9a4b8f629bfa43a1e706b8acc19498cf201876c583fec611355fbe006c4e97c3` |
| 9, alternate context | `93351cb0eeff6d532402e831d6f61143d1b1f9e31e0a77a0629094ea46b5d34b` |
| 10, qualified af9 native context | `35695fab34f7e130fe3c3560e0568edf0897b837488ee7761864103176d6dd1b` |
| 11, qualified af9 native context | `7c2fb5be2b9948e8df8c829fd9ac09ac7524d693ca980b3df1c81f5908ad858b` |

The adapter also pins the supported current compiler/configuration/lock context;
PR86's changed dependency context requires its own qualification. Historical
canonical bytes, original grant ownership and every non-kernel semantic field,
including rejected dispositions, must agree. Unknown identities or contexts refuse.

The service `base64` dev-dependency changed the whole-lock hash from
`687ef52c…b2d5e` to `5e5018c6…8ac54` (97,546 → 97,557 bytes), with identical
native compiler/configuration. The reviewed native executing context is now
`9e7f86bb6f374509c144ccf99fd3b2741b5512947840973299fe887c3db41210`.
Only that exact transition is admitted; old document hashes and V1 owner behavior
remain unchanged. The alternate `91910492…41c96f` lane gains no new-lock allowance.
Original-A replay must qualify the resulting build; static equality is insufficient.

**2026-10-19 portable-context qualification:** candidate
`f4fe30b9c6e15dce5de20c698fbdd22d2eccf63de86d820c3a887e73393de387`
has identical compiler, configuration-file and dependency identities to the native
lane, with different build-profile flags. Only this exact executing context may
extend already-known kernels whose required context is `9e7f86bb…db41210`;
the alternate lane and unknown contexts remain refused. Qualification requires
the optimized CLI to verify the unchanged original-A corpus, not merely a debug
consumer or constant comparison. The ledger records whether that execution passed.

The adapter amendment changes the executing source closure to
`5343f50812694f9a616e701f8492ce822af4ea7337c6ff04e90bd063110b80e9`;
both semantic inventory owners pin it exactly. Its new registry module is included
in that97-file closure. Four original V12/V13 native/portable documents from source
closure`68194f31…74490` remain exact historical identities with their original V2
owner policy. Unknown documents and reverse portable→native execution still refuse.

**Consequences:** preserve original artifacts and producer pins; do not relabel
them, reactivate old issuance claims or execute an old runtime. Qualification is
artifact-specific, not a universal equivalence claim. The acceptance ledger must
record authentic producer A and a distinct changed-build B before closure.

## 2. Profile scope is immutable configuration

The effective profile binds graduation-year set, jurisdiction set, sport/category set,
performance window, effective as-of cutoff, contact season/currentness, regional membership,
discovery priority, source-plan/registry and acquisition window, completion fence and every
identity/cohort/event/affiliation/participation/contact/publication policy reference. Resolve
references by exact version/revision/content identity, not by a mutable name or latest alias.
Changing meaning creates a new profile identity and preserves its predecessor.

The initial production profile selects exactly MI, MN, IA, IL and CO and Class of 2027.
UP-first scheduling within that profile does not shrink its five-state denominator. A UP-only
qualification filter is a separate source-backed school/program membership contract, including
sport, season, category and co-op relationships. Neither priority nor a roster count is such
a filter. A five-state publication may certify only its declared five-state obligations;
it cannot discharge the distinct 49-jurisdiction national obligations.

Future cohorts and an explicit 50-state-plus-D.C. profile require no identity/schema redesign.
They still require supported, qualified execution and their own declared completion fence.
Territories require a separately versioned model extension. Never enlarge a historical 49
claim, silently admit Alaska/Hawaii, or infer unplaced schools into a jurisdiction.

Apply cohort, resolved school affiliation at the effective cutoff, regional membership,
participation and event policy again at publication. An in-scope athlete may retain an
eligible out-of-state meet mark; unrelated participants in the same captured meet remain
out of the selected recruiting view. Store-wide export is not a selected-profile projection.
Preserve excluded/unknown/conflicting evidence and its reason counts under retention policy.

## 3. Identity and revision separation

Keep these concepts distinct, typed and immutable after acceptance:

| Concept | Contract |
| --- | --- |
| Installation/store identity | Durable lineage across restarts and permitted relocation; distinct from a process-local snapshot capability or path. A restore declares its retained lineage and recovery fence; it does not manufacture a new identity to evade receipts. |
| Query profile identity | Canonical immutable configuration and exact semantic references. Selection is an explicit store-bound capability, not a mutable global default. |
| Run/synchronization identity | The declared lifecycle, lineage, profile, source obligations, traversal/revision and completion fence. A session or failed attempt is not a new logical run. Preserve legacy CensusRun/RunManifest bytes and semantics. |
| Source-work/attempt/capture identity | A logical obligation retains its retry ownership; every genuinely new physical acquisition has its own attempt/capture provenance. Recovery replay does not become a new fetch. |
| Evidence/data revision | A committed input dependency state and its lineage. Operational timing and publication bookkeeping alone do not create semantic facts. A bare MVCC sequence is not a durable reopenable checkpoint. |
| Interpretation/application identity | Exact retained observation plus parser/dependency versions; an effect also binds its accepted source/input revision and target/rule meaning. Different payload under one effect ID is a typed conflict. |
| Publication identity | An owning immutable logical job plus frozen input, effective profile, semantic policies, schema/codec/token/recipe and artifact-generation identity. Historical bundles remain addressable under their original identities. |

Athlete and school public identities are never reused or reminted because cohort, selected
geography, name, affiliation, as-of cutoff or season changes. Graduation-year and school
claims are evidence-backed mutable assertions, not permanent person identity. Preserve every
historical ID. Evidence-backed merges/splits use reversible versioned bindings and aliases;
an ambiguous old identity remains resolvable as ambiguous/deprecated with successor IDs.
Changing the forward allocation rule requires a separate explicit contract and historical
mapping treatment; this ADR does not retroactively rewrite existing minted IDs.

Reuse completed work only when its exact semantic dependency contract still matches. Changed
parser/policy/profile/evidence creates new interpretations or projections where needed, with
retained predecessors. A source returning A, then B, then A may reuse A's bytes/interpretation
but requires a distinct accepted A3 revision; A1's receipt cannot suppress B2-to-A3. Increasing
an execution budget may resume unfinished work without changing a completed workflow's meaning.
Record any permitted continuation separately; do not overwrite an immutable manifest or reset
an exhausted logical source budget by manufacturing a new key.

## 4. Qualified evidence reuse, not inherited completion

Reuse requires retained, readable, integrity-checked captures and interpretable provenance,
qualified source ownership, compatible parser/policy dependencies and the profile's actual
freshness/eligibility rules. Cache replay keeps the real acquisition date. A valid conditional
revalidation or a new physical capture supplies its own provenance; it never rewrites the old
one. Missing legacy fields stay unknown. Missing/corrupt/evicted bodies, stale contact tenure,
partial discovery and unresolved identity/cohort/affiliation remain explicit gaps or review.

Public-source discovery remains the population authority. No admissions workbook, operator
recruit list or prior workbook/export supplies a denominator. Historical qualified source
facts can support a production projection only through explicit lineage and current admissibility;
old coverage totals, receipts and seals cannot stand in for the new completion fence. Failed or
partial source work is not empty population; disappearance is not deletion without the exact
source-scoped negative-evidence contract in spec v2 §§7 and 10.

Offline reparse pins retained bytes/context and explicit dependencies and performs zero network
work. Corrections, withdrawals, identity remaps and policy changes invalidate their affected
projection keys durably. Retain candidate marks needed for corrected-winner fallback. Do not
change accepted source evidence merely to obtain a smaller unresolved set or easier seal.

## 5. Generation, acceptance and query-copy boundary

Preserve ADR-026's explicit format classification, one store owner, fenced evidence reads,
bounded staging and atomic complete-generation publication. The selected profile and all its
resolved references must join the frozen input and completion fence before any consumer derives
rows. Existing format versions and policy identities keep their original meanings; new meaning
requires explicit versioning, not hydration of an old frozen input or reuse of its Published receipt.

Workbook, normalized snapshot, coverage/review/audit, manifest and any selected query database
consume the same validated immutable generation and effective profile. Independently verify
membership, every output family, record/candidate/provenance equality, counts and hashes before
atomic publication. Stale work refuses; failed generation/promotion retains the previous valid
bundle and its original freshness. Retrying a logical job may reuse only its own matching input;
a changed profile/input/policy requires an explicit new publication identity. Corrections publish
new artifacts; they never edit an artifact already given to a consumer.

National publication uses the existing checked `ExportGeneration` and canonical selected
workbook request identity, independently of parent resume ordinals. It does not mint new
acquisition identities or reset attempts. New National submission requires advertised
`selected_publication_v2` and generation revision 1, binding the exact profile's single cohort.
Recorded V1 requests retain their original RPC payloads; a missing-selection bind may only read
its matching retained Class-of-2027 manifest. Direct workbook V1 and participation V4 are unchanged.
Missing historical generations do not default into current execution. Qualification remains
scoped to the cases in the evidence ledger; these contracts do not establish national completion.

Current documentary sealing compares the retained attachment and historical-school
inventories with current checked authority and accepted identity on one store
snapshot. Raw-table equality or an unrelated journal sequence is not that fence.
Historical opening verifies the original frozen authority without requiring it to
remain current. Cache only authenticated in-memory metadata during hydration;
do not recapture source bodies or introduce another replay format for this check.
Current sealing also requires the shared current-selected authority policy. Historical bundle
verification remains read-only; metadata presence alone cannot grant current authority.

Event policy22 governs new selected capture/publication. Frozen policy21 verifies
its original exact event reference and complete profile family against its retained
semantic context. The same selector uses that captured policy; mismatches refuse.
Historical readback grants no current write authority and introduces no replay
format, artifact relabeling or acquisition/issuance alternative.

Keep three decisions separate: exact-build implementation qualification, configured-run
completion with declared terminal gaps, and approval of a publication under its coverage rules.
A profile registry, command success, old seal, large row count or passing focused suite proves
none of the other decisions. Full spec v2 and existing applicable release/fault/fuzz/resource
obligations remain; this lifecycle amendment waives no feature or acceptance gate.

A finished query database remains an optional, rebuildable consumer of accepted generations.
This ADR neither chooses SQLite/DuckDB/PostgreSQL nor authorizes database creation, dual evidence
writes, reverse synchronization or population of a query store.

### Reviewed-source frozen container — 2026-10-17

**Context:** the two-claim replay probe measured 679,089 encoded bytes in the
numeric-byte `records` field of an approximately 721-KB container; the unchanged
4-MiB grant refused historical artifact verification.

**Decision:** payload schema `roster-reviewed-source-publication-closure-v2`
encodes record key/value pairs as canonical padded base64, preserving their exact
decoded bytes. Reference-shape V1, durable Store layout and source-object hashes
do not change. Retain the aggregate cap and four-times-wire decode admission.

**Consequences:** current issuance writes only payload V2; unqualified payload V1
refuses explicitly, without migration, fallback or artifact relabeling. Original
qualified Report V3 replay remains independent. Qualification requires actual
source-free workbook/artifact readback and malformed/predecessor refusal; this
decision is not a passing receipt.

### Immutable attachment read capability — 2026-10-17

Current and frozen owners fully validate their ledger, receipts, inventory,
targets and source closure before capturing an opaque original-grant continuation.
Borrowed capability access authenticates that continuation's origin and usage
floor; it does not rebuild the unchanged projection. Owner data remains immutable
for the borrow. Serialized formats, historical bytes and resource limits do not
change; foreign or poisoned grants still refuse.

### Original result school statement — 2026-10-17

Retained-result parsing may expose a checked quote of the uniquely owned,
untransformed school cell: absolute UTF-8 byte span, original body/row/context
references and text digest. Missing, transformed or ambiguous cells withhold the
quote. Existing parsed rows, context wire and IDs remain unchanged. This quote
is source data, never a SchoolId, accepted affiliation or issuance authority.

### Dated documentary status — 2026-10-18

Point-status evidence contributes only on its documented event day. A different
day with the same reviewed person is retained but contributes no positive fact.
Held store, target, accepted head and inventory must agree before date filtering.
Withdrawing all matching-day reviews cannot promote another day's status.

Grant origin is authenticated before the continuation usage floor; the floor
still compares pre-charge usage. A replenished foreign grant remains foreign.

### Explicit full-window source input — 2026-10-18

`IndependentParticipationWindowPacketV1` records an explicitly stated full window,
school year, cohort and documented summer boundary from one uniquely owned MHSAA
article. Publication year/grade and declared cohort must agree. No schedule extrema
or point-status dates infer a full window. This checked input is source metadata,
not accepted eligibility or NCAA certification; public review and selector binding
remain separate requirements.

`ParticipationWindowPacketV1` uses its own retained phase and slot. Its `RootJson`
leaf binds the exact whole packet, not a capture leaf that another packet may
already own. The kind is refused on other roots/slots. Original bodies and distinct
subject-heading/window-paragraph proof locations remain retained and replayable.

The full-window input introduced dependency epochs 6/7 with a 94-input closure.
The qualified `af9c58e7` checkpoint emitted 10/11 with a 96-input closure. Current
captures emit 12/13; original 2–11 artifacts remain read-only and are neither
relabeled nor regenerated. The qualification ledger distinguishes each tested build.

### Reviewed-roster owner spans — 2026-10-18

**Context:** an unchanged genuine roster has a 23,082-byte JSON-LD owner
declaration. The former shared 4-KiB statement limit withheld that authority.

**Decision:** only `OwnerSourceSpan` permits a borrowed range up to 2 MiB.
Person, membership and temporal spans remain bounded at 4 KiB. Current provider
contract V2 and authenticated af9 10/11 use this owner policy. Original 2–9
identities retain V1's 4-KiB policy before collecting or comparing checked evidence.
The 32-declaration limit, first-failure precedence and late contrary-owner
validation remain. Participation wire V4 and roster V2/V3 envelopes do not change.

**Consequences:** preserve old unavailable outcomes, ranges and retained family
hashes. Compiler/dependency context must match the exact known historical kernel;
unknown identities, changed bytes or a mismatched provider contract refuse.

### Reviewed-school resource boundary — 2026-10-18

The original aggregate checked-read/workflow grant was 128 MiB (134,217,728 bytes)
and 131,072 traversal steps. Individual decoded objects and physical inventory remain
bounded at 4 MiB. Reviewed-roster envelopes and each school/calendar source proof
are bounded at 8 MiB; their writer, Store replay and Report loader share that limit.
School journal entries retain their 4-MiB bound and school records their 64-KiB
bound; calendar enrollment and record-bundle limits are unchanged.
Count encoded bytes before fallible reservation; preserve original grant ownership,
poisoning and every charge. Base64 expansion does not enlarge decoded-object limits.

**2026-10-19/20 amendments:** the aggregate encoded grant is 16 GiB
(17,179,869,184 cumulative charged bytes); traversal is bounded at16,777,216 steps.
Retained national diagnostics first required1,719,214,944 encoded bytes, then
5,232,582,526 cumulative charged bytes and at least1,065,627 steps. This byte
counter is not a heap allowance; the diagnostic process still has a separate
16-GiB/zero-swap scope. Original-owner, poison and pre-allocation admission
rules remain mandatory; per-object limits are unchanged. These are authorized,
bounded capacity changes, not RSS or national acceptance. The verification ledger
records each actual refusal and subsequent execution.

Missing reviewed-roster authority remains an explicit withheld disposition. Its
aggregate gap/reference/reason inventory uses the physical observation budget:
4,000,000 entries and4-GiB encoded data. The computed roster-family proof shares
that aggregate byte bound in capture and descriptor validation; individual roster
records/source proofs retain8-MiB admission. No rejection is truncated or promoted.
Aggregate family encoding measures first and reserves exactly before writing.

Generic content-addressed proof storage admits an object up to4GiB so that the
aggregate family is representable. Selected proof directories still require at
most3GiB total and16,384 objects; that combined bound can be stricter. Ordinary
metadata/source/chunk readers keep their own smaller ceilings. Generic integrity
verification hashes through the existing64-KiB buffer rather than loading each
large object merely to verify it. This is not a peak-RSS qualification.

Grouping/normalized input stages admit at most4GiB of accounted encoded data;
the1-MiB per-row and16,777,216-work-unit limits remain. This counts borrowed
input serialization, not a4-GiB allocation or additional heap permission.
Overflow still refuses the complete stage with its name/resource/needed/available.

School-bound V4 installs its mandatory attachment descriptor from the authenticated
school closure under the original school and attachment grant continuations.
It neither rereads the live snapshot nor starts another allowance. The optional
school descriptor does not make absent historical school authority eligible.

School/calendar proof loading checks the 8-MiB reference ceiling and exact encoded
payload length before payload reservation or chunk reads. Hash, canonical encoding
and native proof validation still apply. These are admission limits, not measured RSS.

Calendar review authenticates the original approved `CaptureJson` leaf through the
live or frozen attachment owner, not by equating an application-context digest with
a packet-root digest. Body identity remains its approved digest and byte count:
content-addressed body reuse may retain an older container root. Original grant
continuations remain mandatory; no ledger reload, payload clone or new wire version.

The authorized 8-MiB grant admitted genuine A after removing a duplicate
preparation-time retained-body decode. Genuine B still required a derived
8,609,689 accounted bytes, including its journal envelope and receipt.
The authorized 2026-10-18 increases retain every remaining allocation charge.
The integrated documentary workflow exceeded the former 9-MiB grant at
10,152,626 cumulative encoded bytes; 16 MiB cleared that documentary refusal,
not the complete roster workflow. The genuine partial-roster case subsequently
required a roughly 6-MB envelope and approximately 40 MB cumulative replay work.
The 8-MiB envelope/64-MiB work bounds admit that case without dropping physical
inventory. These are workload admission limits, not resident-memory allowances
or national-run qualification; exact-build resource evidence belongs in the ledger.

The unchanged-wire refresh path reuses historical declaration, input and target
checks already proven at load. It still verifies every material against the
current view, reloads receipts and validates the store-bound projection. Changed
wire takes the full checked-load path. There is no grant reset or omitted charge
for remaining work.

After removing repeated preparation and immutable-metadata validation, both
32,768- and 65,536-step trials still refused the supported 16-source workflow.
The finite allowance is four times the original ceiling, under the subsequent
explicit authorization to loosen traversal limits. Every remaining operation
stays charged; the source-count policy is not narrowed to fit the old quota.
Exact-budget and complete-chain results belong in the verification ledger.

Preparation validates introduced leaves once before returning checked state.
Application revalidates them: retained-root replacement need not advance
evidence/derived generations. This changes resource admission, not documentary
authority or historical evidence, and does not prove national-scale memory use.

Private school source closure `result-school-source-closure-v2` shares roots
already retained by the authenticated attachment closure and preserves school-only
roots separately. Unqualified V1 refuses; qualified roster/report formats and
original artifacts remain unchanged. Lifecycle-9 public review/revoke and cold/frozen
readback passed on `78a3504d`. Those historical receipts do not certify a later
eligible-PR/window publication; use the current delivery matrix and verification ledger.

### Reviewed exact-day calendar authority — 2026-10-19

Explicit calendar enrollment advances an already school-enrolled store from
lifecycle 9 to 10. It binds the original school enrollment proof; it does not
relabel that proof, change fresh-store lifecycle 2, or enroll during publication.
School review and correction remain available under their original contract.
All open/inspection paths validate protected authorities from one snapshot;
generic journal and receipt writers cannot publish calendar authority.

Reviewed calendar correspondence asserts only the retained result's exact
original day, scholastic context and in-season relationship. It does not assert
a full eligibility window, cohort, school identity or NCAA eligibility.
Participation remains wire V4. Frozen calendar proof must preserve enrollment,
decisions, revocations and original source context, including an enrolled-empty
inventory. Missing or altered required proof refuses; old generations retain
their recorded authority. This is the integration contract, not passing
publication or national-run evidence.

One review record may cover several documentary subjects. Bind correspondence
to the exact checked packet root, not the shared record digest. Embedded capture
metadata authenticates against the retained root's canonical JSON encoding;
different object-key order is not different evidence. Hash that encoding with
the existing bounded writer, without cloning the body or changing source bytes.

### Reviewed secondary-person authority — 2026-10-17

Qualification profile V5 gives affiliation/publication references revision 6,
frozen affiliation descriptor 3 and source-family inventory V2. A reviewed
roster may join a secondary provider person only through a history-authenticated
SourceBound/SamePerson decision committing the exact current candidate row;
all decision members must remain in one accepted public group. A correction
clears prior secondary authority; only a later explicit accepted decision can
restore it. Unproven links withhold. V4 descriptor 2/family V1 remains
strict-primary under live and frozen interpretation. No Store migration,
historical relabeling or passing acceptance receipt is implied.

### Exact roster-member withdrawal

The reviewed negative action binds the original sealed proposal, source person
and row locator; its scope is the original exact capture day. A checked parser
row authenticates source evidence, not accepted membership or issuance authority.
Use the existing roster head, atomic review/object/receipt batch and unchanged
`WithdrawsClaim` reducer. Preserve original claims and receipts; replay cannot
reactivate them. Current and frozen validation enforce the same target and scope.

Withdrawals use distinct V1 basis/review/input schemas. Nonempty withdrawal
families use inventory V3; positive-only family bytes and historical kernels
remain unchanged. This grants neither secondary-person authority nor a new
participation format. Missing proof refuses; old frozen generations retain their
original school. Acceptance is recorded separately in the verification ledger.

## 6. Enrollment, migration and historical protection

No automatic migration or corpus import is authorized by this decision. Unknown/newer formats,
unsupported profiles and conflicting enrollment refuse without mutating historical bytes. If
production enrollment requires a persisted shape change or historical corpus adoption, Main
must first specify its compatibility and provenance contract, inventory evidence/IDs/receipts/
workflow state/publication pointers, verify a consistent restorable backup, reserve overlap space
and rehearse restartable migration/rollback on an isolated copy or separate destination. Preserve
original bytes, timestamps, claims, contradictions, IDs, manifests and artifacts; unknown
historical provenance cannot be fabricated. Shadow-compare and explain changed projections.
Cutover is explicit; rollback uses a compatible verified predecessor/backup and a qualified
post-cutover-write replay policy, not an old binary opening incompatible bytes.

Current reviewed-roster publication captures the protected ledger and original issuance
closures, not a duplicate global school/team/conflict/source-observation inventory.
Original inspection bounds and conflict proofs remain authoritative. Historical wire
physical arrays remain readable unchanged; this grants no support to an unsupported
historical semantic policy.

Retained historical data has its original read/refusal policy. Preservation does not require
current code to semantically verify unsupported old formats. Reader/job/backup pins and declared
replay horizons constrain bounded garbage collection; adopting this ADR grants no deletion,
receipt pruning or destructive retention authority.

## 7. Current supported runtime and supersession

At proposal preparation, the source supports durable store identity, immutable QueryProfileV1
registration/selection and a configuration-only profile compiler. The compiler refuses
source-backed region membership/priority, jurisdictions outside the legacy 49 envelope,
provisional inclusion and acquisition after as-of. These modeled unsupported choices are not
removed from the raw profile. Its descriptors explicitly disclose current-affiliation,
profile-source-graph and completion integrations still pending. The configuration-only descriptor
remains policy 18; scalar publication now derives policy 20. Neither descriptor registration nor
publication-policy support qualifies selected-profile execution.

The legacy store RunManifest accepts one census run and refuses a different binding. Legacy
sealing remains Class-of-2027-bound. Current athlete allocation incorporates candidate
school/name/cohort material. Those facts establish implementation gaps, not authorization to
weaken existing refusal, change IDs or claim stable multi-cohort runtime is implemented.
[PROFILE-EXECUTION-V1](../PROFILE-EXECUTION-V1.md) describes the current narrower capability.

The enrollment-only lifecycle-3 installation capability now exists and binds an immutable
profile/resource configuration. It refuses generic execution before recovery or serving;
it is not qualified ReusableProduction enrollment/execution. Lifecycle 4 scopes fresh capture
qualification; lifecycle 5 preserves historical public-identity qualification, and lifecycle 6
retains its original authority context. None certifies annual operation or changes the forward
production contract. See the [installation record](../verification/2026-10-14-pr29-summary.md#installation-and-result-status)
and [current repair evidence](../verification/2026-10-14-deep-codebase-review.md).

This ADR supersedes ADR-013's fresh-only lifecycle restriction solely for
explicitly enrolled, qualified ReusableProduction operation. ADR-013 remains binding for
FreshQualification and all historical fresh national claims. ADR-009 remains the denominator
of the distinct 49-jurisdiction national profile, not a global ban on other configured production
profiles. ADR-026's schema/generation protection remains in force; enrollment and any new
persisted/runtime wire are separately versioned and Main-owned. The existing
[NATIONAL-CENSUS-PLAN](../NATIONAL-CENSUS-PLAN.md) remains the only implementation/order/acceptance
plan; this decision adds no parallel plan or automatic runtime switch.
