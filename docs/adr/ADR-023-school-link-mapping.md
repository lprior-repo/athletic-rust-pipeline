# ADR-023 — School-link extension beyond the landed join core

**Status:** Accepted (2026-10-04); implemented in the landed core and this delta — §1–2 and the
attested provider keys in `7vk.2`, §3–4 (durable review cases and refusals) with the join's apply
path, §5 in `7vk.3` (`DirectoryIndex::link_name`, the co-op member path and the campus-designation
guard), §6–7 by the transfer-scope regression and the workbook/CSV readback. Dated evidence is in
the verification ledger (removed 2026-10-09).

## Context

[ADR-021](ADR-021-school-address-join-durable-stage.md) landed the school-address join as one tested
core: `DirectoryIndex`/`LinkDecision` in
[`crates/census-domain/src/school_directory/link.rs`](../../crates/census-domain/src/school_directory/link.rs)
(module root; the index and matcher live in `link/index.rs`, the form ladder in `link/forms.rs`, the
state-record attestation in `link/attest.rs`),
restricted to `Nces`/`Pss` keys published under the `Ccd`/`Pss` labels, requiring a published street
and a published state, guarded by middle-school status, matched through a name ladder
(exact/core/parenthetical/parenthetical-inner/alias) with city preference; ties return
`ReviewReason::Ambiguous` with candidate refs, and a match appends owned `SchoolPostalAddress`
claims plus the CCD website through the durable `SchoolAddressJoin` stage.

What that core does **not** cover, and what the `7vk` parent contract requires:

- `StateRecord { state, id }` (association/state-provider) identifiers are not index entries; only
  `Nces`/`Pss` are.
- `LinkDecision::Review` outcomes are counted and recorded in `report.json`/`outcomes.jsonl` only;
  no `ReviewCase` reaches the review lane, so an ambiguous join is not durably askable.
- Campus, co-op and transfer semantics beyond the middle-school guard are unspecified.

A parallel predicate (`crates/census-domain/src/model/school_link.rs`) was written during this
session before the landed core was visible (the worktree was one commit behind `origin/main`) and was
withdrawn on discovery: one subject, one implementation. This ADR records the delta to build inside
or beside the landed core, never as a second matcher.

## Decision

1. **Provider and state-record keys extend the same core.** `StateRecord { state, id }` entries
   become index entries only when the lane publishes an association or state-education identity for
   them, and they are matched by the landed ladder and guards (state equality, middle-school
   parity, city preference). `DirectoryKey::Weak` stays untargetable, as landed.
2. **Attested provider keys are evidence.** A state-record candidate is admissible when its record
   id already appears in the school's `source_identities` under the publishing association
   namespace (`SourceNamespace::AssociationSchool { association }`), or when the landed name+city
   corroboration holds. A shared identity never links across states.
3. **Ambiguity becomes durable.** `LinkDecision::Review` outcomes are filed as `ReviewCase`s under a
   `SchoolLink` review family with both sides' candidate refs and lane evidence, and remain visible
   in the join report counters. Filing has one home: the join core's apply path, applying once under
   replay; the offline reporter stays read-only.
4. **Impossibility is refused, not reviewed.** Entry state ≠ school state, or either state
   unpublished, is a refusal (reported), never a review; the landed `no_match`/`missing_state`
   counters keep their meanings.
5. **Campus and co-op.** Entries differing only by campus designation stay distinct identified keys
   and are never collapsed into one; a school marked `co_op` links several identified keys only when
   each link independently satisfies 1–4, and member campuses are never merged into the co-op.
6. **Transfers are athlete-scope.** A transfer never links or merges school identities; historical
   references keep the school they were observed at ([ADR-016](ADR-016-merge-and-identity-semantics.md)).
7. **Application and readback.** Accepted links flow through ADR-021 §3's durable stage; any schema
   addition for review cases follows the AGENTS migration contract. Consumers read store shapes only
   ([ADR-019](ADR-019-export-derivation-single-home.md)).

## Consequences

- Unblocks `3c5` (directory facts beyond CCD/PSS on census schools) and strengthens `rh3`'s program
  binding by making ambiguous joins askable rather than report-only.
- Implementation checklist: extend `DirectoryIndex`/`LinkDecision` (never a second matcher);
  association-lane corpus facts for state records; `ReviewFamily::SchoolLink` with label/field/
  parse/`askable` and packet facts in `census-review`; durable case filing with apply-once replay;
  named tests (cross-state refusal, unpublished-jurisdiction refusal, ambiguous outcome files one
  case, campus pair stays distinct, co-op links several keys, replay files once, workbook/CSV
  readback shows the link evidence with provenance); dated evidence from a real corpus generation
  with linked/ambiguous/no-match/refused counts.
- Withdrawn in this change: the parallel predicate and its tests that briefly existed under
  `crates/census-domain/src/model/school_link.rs`; the landed core plus this delta supersedes them.

## References

ADR-021 (landed core and durable stage), ADR-020 (corpus model), ADR-016 (identity semantics),
ADR-019 (single export home);
`crates/census-domain/src/school_directory/link.rs` and its `link/{index,forms,attest}.rs` modules,
`crates/census-service/src/school_address/join.rs` and its
`join/{apply,link,forms,support,generation,lanes}.rs` modules,
`crates/census-review/src/families.rs`.
