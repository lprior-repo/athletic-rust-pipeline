# ADR-022 — Admissible cross-source identity corroboration

**Status:** Accepted (2026-10-04); implemented and evidenced 2026-10-04 (athletic-rust-pipeline-46r).
Per-source link-document qualification remains open (athletic-rust-pipeline-b0f).

**CEN14 amendment (2026-10-08):** The capture-lineage contract below supersedes the URL-based
attestation definition in Decision 2 and 4 and the URL-only Class B consequences. The original
2026-10-04 test/evidence record is historical; it does not certify the amended predicate.

[ARCHITECTURE.md](../../ARCHITECTURE.md) §8 owns the binding rule: name, school text, cohort, a score
or model agreement alone cannot establish identity; provider ownership and hard contradictions
survive every projection; Rust requires admissible corroboration and reviewer agreement for
model-assisted acceptance. [ADR-005](ADR-005-ai-cannot-override-contradictions.md) owns model
authority, [ADR-016](ADR-016-merge-and-identity-semantics.md) keeps identity lossless, and
[ADR-011](ADR-011-census-seal.md) requires every accepted claim to resolve to retained bytes. This
ADR decides what a *second, independently captured* provider key can establish, before any shared
domain contract changes.

## Context

Admissibility lives in two places today and they disagree.

- `crates/census-domain/src/model/identity_validation.rs::positive_identity` accepts `SamePerson`
  only when every member shares the **identical primary provider person key**
  (`IdentityFact::shares_primary`). The key is produced by `identity_decision::person_key`, which
  admits only numeric ids in `MilesplitAthlete`, `TfrrsAthlete`, `DirectAthleticsAthlete` and
  `AthleticNet { kind: "athlete" }` namespaces.
- The review packet's positive-evidence gate
  (`crates/census-review/src/athlete_verdict.rs::has_positive_evidence`) accepts the packet when any
  `shared_source_identity: …` flag is present. That flag
  (`crates/census-review/src/athlete_flags.rs::shared_source_identities`) fires on any provider key
  present on **both** rows, including a key one row carries as a `source_link`.
- `distinct_provider_objects` (same namespace present on both rows with disjoint ids) is emitted but
  never withholds. `namespace_ids` reads `CanonicalAthlete::identities()`, which is
  `source ++ source_links`; `CanonicalAthlete::merge` (`crates/census-store/src/entities/canonical.rs`)
  absorbs a merged member's identities through `add_identity`, so an *attested* key and a key
  *derived* from an earlier merge are indistinguishable at decision time.

Consequence: a pair that is one person but holds different primary objects (a roster or result row
that links a profile key, plus the profile row whose primary *is* that key) is flagged as sharing a
source identity yet cannot pass `positive_identity`; and a provider that issued two athlete objects
for two rows is a hint rather than a reason to withhold. The census therefore cannot admit or refuse
cross-source corroboration on a stated contract.

## Decision

1. **Identity admissibility speaks only provider person keys.** A *provider person key* is a
   `(provider, id)` pair minted by `person_provider`/`person_key`. School, team, meet and
   association namespaces, and non-numeric ids, never corroborate a person.

2. **Attested keys, not derived keys.** A key is *attested* on a row when that row's own captured
   evidence carries it: the row has at least one `Evidence` with `method == Parsed` and a non-empty
   source URL, and the key is present in the identities that row's own observations recorded. A key
   that appears only because `CanonicalAthlete::merge` absorbed another member's identities is
   *derived* and never corroborates on its own. `IdentityFact` must carry the attestation split;
   today a single `parsed` bool cannot express it.

3. **Admissible corroboration classes.**
   - **Class A — shared provider object (already implemented).** Every member's *primary* key is the
     same `(provider, id)`. Sufficient alone; this is the current `shares_primary` rule.
   - **Class B — linked provider object (new).** There exists a key `k` such that: `k` is the primary
     key of at least one member; `k` is attested by every other member (as `source` or as an attested
     `source_link`); every member is parsed; grad years are equal; genders are not mixed; no member is
     `conflicted` under the existing alias rule (two keys of one provider on one row); no provider
     appears as primary on two members with different ids; and the attestations of `k` on distinct
     members resolve to distinct captured documents (distinct non-empty source URLs). When Class B
     holds, `SamePerson` is admissible with the shared key as its evidence.
   - **Class C — everything else stays REVIEW.** A matching name/school/class
     (`name_school_cohort_agree`), two agreeing models, a shared key attested only by one document,
     or a shared *derived* key alone never accept. They remain pending for independent advice, and
     agreement cannot override a hard contradiction.

4. **Independent source-family attribution means independent documents.** Two attestations of the
   same key corroborate only when they come from distinct captured documents — distinct retained
   source URLs. A single association or meet document that lists one person under two schools (a
   transfer) does not corroborate with itself; such a pair remains REVIEW carrying the
   `shared_source_identity` flag. Independence is a property of retained bytes, not of model count.

5. **Hard contradictions withhold acceptance with reasons.** The existing
   `census-review::athlete_verdict::HardContradiction` kinds remain (`grad_year_evidence_differs`,
   `gender_differs`, `retained_source_conflict`). This ADR adds to the withholding set: a provider
   that issued disjoint primary objects across members (`distinct_provider_objects`), and structural
   alias conflict inside any member. Withholding writes no acceptance: the case stays `Retained` with
   its reasons and every member row is preserved.

6. **Model assistance stays advice.** A reviewer lane may decide Class B only from a packet that
   states the mechanical facts (the shared key, the distinct provider objects, the cohort evidence);
   two agreeing lanes decide, one lane or disagreement leaves the case pending, and no model output
   substitution for the attestation requirements above. Deterministic merge remains limited to what
   `AthleteIdentityIndex::supports_identity` admits: Class A and Class B both decide under the
   existing `RULE_REVIEWER = "deterministic:shared-provider-object"` rule, and nothing else decides.

7. **Acquisition uses existing source obligations only.** Corroborating keys are exactly the
   identities adapters already record per captured page at the capture/ingest boundary
   (`CanonicalAthlete::add_identity`). Missing corroboration is a source-qualification gap closed
   through the normal ingest obligations; no lookup service, no invented cross-provider fetch, and
   no private corpus may be introduced to manufacture a key.

8. **No circular admission.** A merged row never becomes evidence for its own expansion: after a
   decision the surviving row's attestations are exactly those of its members, and a new row is
   judged against attested keys, never against keys a prior merge derived. Acceptance chains
   therefore grow only through keys that some captured document independently asserted.

## Consequences

- **Domain.** `crates/census-domain/src/model/identity_index.rs` gains the attested-key set on
  `IdentityFact` (in-memory only; no persisted shape or digest changes, so ADR-017/ADR-018 framing
  is untouched). `identity_corroboration.rs::positive_identity` implements Class A and Class B and new
  `IdentityDecisionIssue` variants name each new withholding reason. Attestation reads the row's own
  retained documents: the primary identity's URL or the row's parsed-evidence URL, and every
  `source_link`'s own `SourceIdentity::url`; `observe_attested` documents override them. A key with
  no retained document never attests, and a key merged into a row keeps the document it was recorded
  with, so §8's union reading holds without new persisted provenance.
- **Review.** `athlete_flags.rs` states primary-versus-link detail in `shared_source_identity` and
  promotes disjoint provider objects to a withholding input; `athlete_verdict.rs`'s positive-evidence
  gate requires an attested shared key (Class A or B) rather than any flag;
  `reconcile_athletes`' deterministic path keeps `RULE_REVIEWER = "deterministic:shared-provider-object"`
  and extends to Class B only when the domain predicate does.
- **Tests delivered** (2026-10-04, see the verification ledger (removed 2026-10-09)):
  `model_tests::corroboration` implements `linked_object_corroborates_when_attested_independently`,
  `a_link_shared_pair_without_attestation_stays_review`, `two_ids_from_one_provider_withhold`,
  `same_document_attestation_does_not_corroborate`,
  `name_school_and_cohort_agreement_alone_is_not_positive_evidence`,
  `a_contradictory_third_member_withholds`, `an_unparsed_member_withholds_corroboration`, plus
  `a_retained_link_document_corroborates_without_an_explicit_attestation` and
  `one_document_attesting_both_objects_does_not_corroborate` for the automatic feed.
  `athlete_clusters_tests::corroboration` decides and admits one athlete from a retained link
  document end to end (`a_link_retained_with_a_distinct_document_decides_one_athlete`) and confirms
  an undocumented link stays pending. Existing pins stay green: `athlete_tests::contradictory_cohorts`
  and `athlete_tests::retained_conflict` (fixtures aligned to carry an admissible shared primary so
  the contradiction remains the refusing reason), `athlete_clusters::tests::three_members`,
  `athlete_packet_tests::flags`, and `model_tests::source_ownership`.
- **Open obligation (source qualification).** The predicate and intake are complete, but no adapter
  records a document for the cross-provider links it sees: `crates/census-crawl/src/ihsa/tournament/
  entities.rs::athlete_identities` mints link identities with no URL, and its `AthleticNet { kind:
  "live" }` key is not a person namespace, so the current census feeds Class B nothing and stays
  Class A. Adapters that set `SourceIdentity::url` to the page that asserted the link feed Class B
  with no further change; links without a document stay Class C (REVIEW). That per-source audit is
  tracked by athletic-rust-pipeline-b0f; the census-plan identity row points here.

## References

- [ARCHITECTURE.md](../../ARCHITECTURE.md) §8 (identity review), §9 (source independence).
- [ADR-005](ADR-005-ai-cannot-override-contradictions.md), [ADR-011](ADR-011-census-seal.md),
  [ADR-016](ADR-016-merge-and-identity-semantics.md), [ADR-018](ADR-018-identity-and-evidence-framing-escapes.md).
- `crates/census-domain/src/model/identity_decision.rs` (`person_provider`, `person_key`),
  `identity_index.rs`, `identity_corroboration.rs` (`positive_identity`), `identity_validation.rs`;
  `crates/census-review/src/athlete_flags.rs`, `athlete_verdict.rs`, `athlete_clusters.rs`;
  `crates/census-store/src/entities/canonical.rs` (`merge`).

## CEN14 capture-lineage amendment

A canonical athlete retains typed identity attestations. Each claim binds an exact published
subject namespace and identifier to a source reference, immutable capture SHA-256, acquisition
instant, publisher family, upstream producer, and exact subject locator. Malformed capture metadata
and claims for a subject absent from the row are typed errors, including candidate-only claims.
The subject binding uses namespace and identifier, not URL spelling.

`CandidateOnly` preserves an observed binding but cannot prove independence.
`IndependentPublished` requires source-owned qualification of the publisher and upstream producer.
A URL, hostname, provider namespace, or locally minted fallback identifier does not qualify a
source. Primary and linked URLs remain candidates; the index never synthesizes independent
attestations from them.

Class A retains the existing parsed shared-primary-provider-object rule. Class B requires that
every pair of candidate rows has qualified captured claims for the same linked key with different
capture bytes, source families and upstream producers. Case-only lineage spelling changes do not
establish separation. The existing cohort, gender, denied provider-object and alias-component
contradictions still withhold acceptance. Mirrored bytes, replicated state-result feeds and two
pages emitted by one timing producer remain REVIEW even when their URLs differ.

`AthleteIdentityIndex::observe_attested` accepts typed additional claims; ordinary `observe`
consumes the canonical row's retained claims. Additional claims also enter the review evidence
fingerprint, so supplying a new claim cannot silently change an existing review decision's
admissibility. The review reducer uses the shared index and never qualifies a source itself.

The index owns only the three lineage strings needed after its borrowed observation returns,
not cloned subject/source/URL/date/locator payloads. Duplicate-lineage checks borrow existing
facts before this necessary index ownership boundary. Canonical persisted rows own complete
claims. No performance improvement is claimed without measurements.

The IHSA tournament acquisition path carries validated `FetchOutcome` capture digests and
acquisition instants to the row mapper. Exact Athletic.net athlete bindings are retained as
candidate-only claims: the source report documents a shared state-meet feed, not independently
qualified roster authorship. No timed-mark parser or public source DTO is changed.
The digest is moved from the fetcher's existing verified body/capture boundary; the adapter does
not rehash bytes already verified by cache acquisition or captured by the fetcher.

Controlled domain fixture documents exercise the positive lineage predicate; they are not real
source-qualification evidence. The real captured IHSA finisher regression covers parse-to-store
retention, candidate-only mirror rejection and reopened claim preservation. The census-review
regression keeps distinct URLs without typed capture lineage pending. These changed scenarios
have not been executed during the concurrent repair wave.

Before certifying CEN14, execute a genuinely independently qualified official-roster/timer
control through parsing, reviewed decision and merged publication. The retained IHSA feed alone
cannot satisfy that positive acceptance case. Execute the named CEN14 and CEN13 scenarios and the
full acceptance gates after all concurrent writers have finished; this amendment records no pass.
