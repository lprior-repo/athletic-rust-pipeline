# Public identity input revisions v2

## Authority boundary

`PublicIdentityInputRevisionTokenV2` is never standalone authority or a standalone
cache key. Equal tokens do not establish equal stores, runs, records, or content.
Consumers must bind the token to the same pinned snapshot's durable store identity,
applicable immutable run lineage, and verified content root before reusing a table
object or authority context. Restored copies can retain the same store identity
and token while subsequent writes produce divergent contents. A verified content
root remains necessary even when lineage and all six counters match.

The token does not replace `StoreRevision`, evidence/publication fences, retained
authority verification, identity decisions, or qualification permissions. It is
not a global evidence-generation cache key. Its fields and persistent codec are
private; it has no public constructor or deserializer.

## Public API

- `StoreSnapshot::public_identity_input_revisions_v2()` returns a fallible
  `PublicIdentityInputRevisionsV2::Disabled` or `Enabled(token)` from that exact
  MVCC snapshot. Read rows, lineage, and content roots through the same snapshot.
- `token.table_revision(Table)` returns the requested counter, refusing tables
  outside Athletes, SourceObservations, ReviewCases, IdentityVerdicts,
  Performances, and RelayResults.
- `Store::enroll_public_identity_input_revisions_v2(StoreRevision)` uses existing
  generic mutation permissions. Qualified stores require
  `PublicIdentityWriterV1::enroll_public_identity_input_revisions_v2(&Store,
  StoreRevision)` with the verified current-V2 writer. Both paths use one shared
  implementation under the existing append mutex and require a current,
  same-origin revision, including repeat enrollment requests.

## Enrollment and mutation semantics

Missing metadata means Disabled. Opening, reading, initialization, and ordinary
writes never infer zero counters or enroll a store. Explicit enrollment adds one
bounded capability record named `capability:public-identity-input-revisions-v2`,
captures the current evidence/derived baseline, and starts six counters at zero
without rewriting historical rows. Zero means no relevant committed transaction
since that explicit baseline; it does not mean an empty table or no older input.
Repeat enrollment preserves the original baseline and all current counters.

Each successful transaction increments each touched dependency once in the same
durable batch as its data and receipts. Conditional effects are classified after
receipt filtering and merging. Every successful derived pointer publication
increments ReviewCases, including empty and carry-forward publications. Staging,
flush, reclaim, repeated or filtered-out effects, empty row operations,
identity-only transactions, journal-only transactions, and unrelated tables do
not advance these counters. Checked overflow refuses the committing transaction;
unpublished staging can remain for existing reclamation.

Malformed, partial, unsupported, oversized, or additional capability metadata
is refused; a baseline newer than its pinned store generations is refused.
Schema/key-format and migration gates remain in force. The optional capability
does not make historical stores fresh, confer installation or qualification
eligibility, or authorize implicit migration.

## Acceptance coverage

The accompanying tests cover pinned six-table row/token pairs, pinned ReviewCases
across pointer publication and reclaim, both deterministic orders of enrollment
versus a revision-fenced write and publication versus a revision-fenced write,
independent-store token collisions, and restored divergent copies sharing lineage
and tokens but retaining different content roots. These are authored acceptance
tests; this contract does not assert that compilation or execution has passed.
