# ADR-017: Canonical digests and identity derivation live in census-store

**Status:** Accepted (2026-09-29)

## Context

The domain-purity lane refuses a serialization crate in `census-domain`'s normal dependency tree,
so the domain cannot hash canonical bytes itself. Three groups of rules nevertheless need exactly
that: athlete-identity derivation and verdict digests (`AthleteIdentityIndex`,
`AthleteIdentityProjection`, `IdentityProjectionBuilder`, `IdentityApplication`,
`AcceptedAthleteIdentity`, `IdentityError`), coach contact-proof verification (`ValidatedContactProof`,
`verify_contact_proof`, `compute_contact_proof`, `ContactProofError`) and review-checkpoint
digesting (`serialized_digest`). Carrying them in the domain required `serde_json`, and the store
already assembled the identity projection from persisted rows, giving one subject two homes.

## Decision

Canonical-JSON encoding and every rule that hashes it live in `census-store`, the crate that owns
storage serialization. The domain keeps the pure vocabulary those rules use — `AthleteCandidateId`,
`AthleteIndexId`, `PersonKey` with `person_provider`/`person_key`, `IdentityStatus`,
`IdentityDecisionIssue`, `VerdictKind`, `AppliedIdentityKind`, `AppliedAthleteIdentity`,
`ATHLETE_IDENTITY_POLICY`, `CONTACT_COLUMNS`, `CONTACT_PROOF_COLUMN`, `RawContactRow`,
`ContactClaimEvidence`, `ContactProofField` and the review-record types — and gains no
serialization dependency. The engines moved verbatim: same functions, same inputs, same canonical
byte stream.

## Consequences

- The domain's normal tree no longer contains `serde_json`; `sha2` stays because roster and
  evidence digests are pure. A future rule that needs canonical bytes belongs in `census-store`
  beside the encoder, not behind a new purity exception.
- Identity derivation, identity application and contact-proof verification have one home each,
  reachable as `census_store::{AthleteIdentityIndex, AthleteIdentityProjection,
  IdentityProjectionBuilder, IdentityApplication, AcceptedAthleteIdentity, IdentityError,
  ValidatedContactProof, verify_contact_proof, compute_contact_proof, ContactProofError,
  serialized_digest}`. `census-crawl`'s contact artifact API keeps publishing its existing names
  (`ValidatedRow`, `stage_verified_contacts`, `read_verified_contacts`), which now carry the store's
  proof types in their signatures.
- Every caller migrates to the store path (crawl, reconcile, report, review, service); no mirrored
  types, domain re-export shims or duplicate rules remain.
- Existing persisted digests, keys and golden files keep their values, because the encoding is
  unchanged; this is a relocation, not a re-derivation.
- [ADR-007](ADR-007-crate-boundaries.md) is not superseded: no new crate was created and the
  extraction stays complete.

## Evidence

`cargo xtask domain-purity` (normal tree without `serde_json`), `cargo xtask gate`, and the
workspace suite with unchanged identity, contact-proof and review digest expectations; dated
commands belong in [VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md).
