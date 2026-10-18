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

**Decision:** use current dependency epochs 4 (full inventory) and 5 (row-local).
These are implementation identities, not qualification-profile or inventory-wire
versions. One current interpreter recomputes the original source inputs.

Read-only adapter V1 accepts only the original whole-document SHA-256 pins:

| Original epoch | Kernel document SHA-256 |
|---|---|
| 2 | `71ef18570ab2683c9f498e2f66983d6a782900fd2647eff4101864b782ecaa46` |
| 3 | `7352af3bf097e5c91a890809b1fa52e700469b3bbccb308eb445fe49fcf48f38` |

The adapter also pins the supported current compiler/configuration/lock context;
PR84's changed dependency context requires its own qualification. Historical
canonical bytes, original grant ownership and every non-kernel semantic field,
including rejected dispositions, must agree. Unknown identities or contexts refuse.

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

### Reviewed-school resource boundary — 2026-10-17

The owner's subsequent authorization permits a bounded increase to an 8-MiB
aggregate checked-read grant and 32,768 aggregate traversal steps. Individual
encoded objects remain bounded at 4 MiB; their structural limits are unchanged.
This changes resource admission, not documentary authority or historical
evidence. It does not establish process-memory or national-scale sufficiency.

Private school source closure `result-school-source-closure-v2` shares roots
already retained by the authenticated attachment closure and preserves school-only
roots separately. Unqualified V1 refuses; qualified roster/report formats and
original artifacts remain unchanged. The lifecycle-9 implementation is pending
qualification; the delivery matrix records actual failures.

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
