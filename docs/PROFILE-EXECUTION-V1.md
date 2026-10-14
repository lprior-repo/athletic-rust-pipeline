# Profile configuration capability v1

[ADR-038](adr/ADR-038-selected-profile-reusable-production-state.md) defines the full forward
selected-profile reusable-production architecture. This document records current support only;
its narrower compiler envelope does not reduce that target or enroll/migrate a production store.

This additive domain boundary checks a selected query profile against nine immutable,
role-specific evidence-policy descriptors. It does not execute a recruiting publication,
qualify a source, resolve an athlete, or certify completion.

`census_domain::profile_execution::compile_profile` returns a privately constructed,
non-serializable `CompiledProfileV1` borrowing the original `QueryProfileV1`. A caller
cannot mutate that profile while retaining its checked capability. Existing profile JSON,
canonical profile IDs, store formats and historical artifacts are unchanged.

## References and roles

`ContractRoleV1::ALL` fixes the validation order: cohort, event, identity, affiliation,
contact, participation, publication, source plan, completion. Each exact reference includes
version, revision and the canonical descriptor SHA-256. Wrong revision, digest, role, or
missing raw field is refused. The descriptors bind evidence meaning, not field names:

- Cohort: typed direct graduation and dated grades after accepted-alias union; unresolved contradictions withheld.
- Event: the exact corrected `RecruitingCatalogV1` identity, with walking and wheelchair exclusions.
- Identity: captured primary or independently attested person keys; hard contradictions withheld.
- Affiliation: resolved school at the effective cutoff, distinct from historical result affiliation.
- Contact: same-capture mailbox, role, program and selected current school year.
- Participation: source-owned individual results or explicit ordered relay members; no inferred legs.
- Publication: one frozen generation, shared derivation, scoped Coverage and verified atomic bundle; schema 3/policy 18 components.
- Source plan: qualified source obligations bound to scope, season and acquisition window.
- Completion: exact profile scope/revision and verified terminal source evidence; unknown is not complete.

Descriptors explicitly distinguish existing components from integrations still unimplemented.
Their `configuration_only` / `not_certified` fields prevent this registry from presenting a
runtime qualification claim. Changing descriptor meaning requires explicit versioning; old
references are not rewritten. These are newly reconstructed descriptor identities, not a
claim of byte equivalence to unavailable unpublished work.

## Interim execution envelope

The compiler admits subsets of the existing 49-jurisdiction envelope, uniform discovery,
all-selected-jurisdiction membership, provisional exclusion, and acquisition through no later
than the effective cutoff. Other modeled choices return typed unsupported outcomes. In
particular, the raw profile continues to support explicit 51-jurisdiction representation and
acquisition dates after the effective cutoff. The compiler's narrower envelope is unfinished
capability, not a change to that persisted model or the final product specification.

Multiple graduation years, sports, categories, performance windows, contact school year and
other supplied values are retained exactly through the borrowed profile. There is no active
global profile and no new implicit default.

## Evidence and outstanding integration

Canonical fixtures independently pin all nine descriptor byte strings/digests, the nested
catalog identity, role swaps, all reference-slot mismatches, deterministic refusal order,
nondefault selections and raw-model preservation. These tests establish the configuration
boundary only. Negative external API fixtures have not been executed through a dedicated
compile-fail harness.

Current components include domain-owned comparison policy, fallible athlete construction,
bounded enrollment-text interpretation, immutable regional configuration and explicit Linux
cgroup accounting. Participation, current-affiliation and identity-history candidates are pure
components, not their runtime integration. [Dated repair evidence](verification/2026-10-14-deep-codebase-review.md)
owns exercised results; none of these components authorizes reusable production or certifies
UP/five-state discovery, a whole-workload resource envelope or the final workbook.

Spec v2 sections 4 and 14 remain incomplete until selected profiles are bound into one
retained evidence generation and every publication consumer. Dated current affiliation,
qualified profile-bound source graphs, completion fences, UP qualification and the matching
workbook/eight-file normalized snapshot are not certified by this module.

## Enrollment-only lifecycle

The explicit installation controls now bind a retained profile and bounded resources into an
immutable lifecycle-3 enrollment. They reuse this configuration compiler and do not broaden
its supported envelope. Generic execution of an enrolled installation is refused before
recovery or serving. Fresh qualification keeps lifecycle 2; historical data is never
implicitly enrolled. See [installation evidence and remaining qualification](verification/2026-10-14-pr29-summary.md#installation-and-result-status).
