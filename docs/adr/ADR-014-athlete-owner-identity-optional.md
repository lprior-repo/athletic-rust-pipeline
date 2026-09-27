# ADR-014 — An entity row persists an optional owner identity

**Decision.** `CanonicalAthlete.source` and `CanonicalPerformance.source_athlete` are `Option`. A row
written by the current code always carries `Some(owner)`: `CanonicalAthlete::new` takes a
`SourceIdentity` and stores it, `add_identity` adopts the first identity as the owner when a row has
none, and every adapter stamps the owner on the performance it writes. A row read from a store
written before the source-owned identity cutover (`99f88c9`) is accepted in its persisted shape:
`source_identities: [first, rest…]` decodes as `source = Some(first)` and `source_links = rest`, an
empty list decodes as `source = None`, and a performance row with no `source_athlete` decodes as
`None`.

Serialization stays single-shape: `source` and `source_athlete` are skipped when absent and the
pre-cutover `source_identities` key is never written. The domain type stays pure — the decode map is
a `serde(from = "PersistedAthlete")` conversion inside `CanonicalAthlete`, not a second code path in
the store, so the Fjall scan, the consolidated JSONL reader and any other `serde_json` reader accept
the same bytes. A performance merge fills an absent owner from the incoming row instead of recording
a conflict, because an absent owner is unknown, not contradictory.

**Why.** The delivered census of ADR-009 (`var/midwest-census`) is written in the pre-cutover shape.
Decoding it failed at the first identity-less athlete — `cargo xtask census-status --store <dir>`
answered `row athletes:ath_0000477b2bc153dc#1363551 is not valid json: athlete … carries no source
identity` — and the PR path failed at the first performance without an owner — `Decode { key:
"performances:perf_0000150f06a7a1de#5600", source: Error("missing field \`source_athlete\`") }` —
which made the durable evidence behind the sealed workbook unreadable by the tree that produced it.
The identity-less rows are real: 74,468 of 2,374,515 athletes (3.1%) in `entities/athletes.jsonl`,
discovered from rosters and entry lists without a provider profile, and every one of the 204,102
performance rows written on 2026-09-22 predates the owner field. Refusing them loses evidence ADR-010
and ADR-013 both require to be preserved, and inventing an owner for them would be the fabrication the
delivery contract forbids.

**Consequences.** The fresh-run invariant "a new athlete and a new performance are source-owned" is
enforced by construction, not by the durable type; a `None` owner can only enter a process from
storage written before the cutover. Consumers therefore handle the absent case rather than assuming:
the athlete observation lane skips an athlete with no identity, the PR reduction falls back to the
athlete's own owner — the value the cutover would have written — for the provenance set and the
`source_athlete` column, and adapter tests name the owner explicitly. Reads of the delivered store
work again: `census-status --store var/midwest-census` reports `schools=31870 athletes=2220866
co2027=575991` in core scope, and a release build of the PR reduction over that store returns 10,231
selections in 3.2 s with a 1.4 GB peak RSS. A future persisted-schema change to these rows is a
migration under ADR-001 and ADR-010, not another accepted shape.
