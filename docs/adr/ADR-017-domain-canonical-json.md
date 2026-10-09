# ADR-017 — The census domain owns its canonical JSON encoder for digests

**Status:** Accepted (2026-09-29).

`tools/gate.sh`'s `domain purity` lane reads `cargo tree -p census-domain --edges normal` against
the ban list in `xtask/src/purity.rs`, and has been reporting `FORBIDDEN dependencies present:
serde_json` since the dependency entered `crates/census-domain/Cargo.toml` in `9e97084`
(2026-09-24). The domain's production code needed it: `model/serialization_digest.rs` and
`model/identity_decision.rs` hashed serde_json's compact encoding for athlete-identity and verdict
digests, and `model/contact_proof.rs` hashed a contact-proof payload the same way. The ban exists
because the domain is the crate that must stay free of the I/O and persistence stack; JSON is how
the persist layer stores records, not a domain capability.

The encoding is not an implementation detail of that fix. Identity decisions persist
`evidence_digest` and `verdict_digest`, the review checkpoint persists a digest over
`(verdicts, cases)`, and roster observations persist a digest over the roster tuple. Changing any
digest's bytes silently invalidates stored decisions, checkpoints and observation provenance, so
the encoder is a contract shared by every writer and reader of those stores.

## Decision

1. **The encoder lives in the domain.** `crates/census-domain/src/model/canonical_json/` implements
   a `serde::Serializer` that writes the same bytes as serde_json's compact writer: no whitespace,
   `"`-quoted keys, serde_json's escape set (`\"`, `\\`, `\b`, `\f`, `\n`, `\r`, `\t`, `\u00XX` for
   other C0 controls, everything else verbatim), non-finite floats as `null`, integers as decimal,
   string/number/bool map keys stringified exactly as serde_json stringifies them, and every integer
   width serde exposes — including `i128`/`u128` — written as canonical decimal.
2. **Floats are spelled by the formatter serde_json uses.** serde_json 1.0.151 formats `f32`/`f64`
   through `zmij`, so the domain pins `zmij = "=1.0.23"` and calls the same
   `Buffer::format_finite`. A different shortest-round-trip formatter (for example `ryu`) produces
   `1.7976931348623157e308` where serde_json writes `1.7976931348623157e+308`, which is enough to
   move every digest that covers a float.
3. **The public names and the persisted contract are unchanged.** `serialized_digest` keeps its
   name, its `Serialize` bound and its lowercase-hex SHA-256 result; only its error type moves, from
   `serde_json::Error` to the domain's own `CanonicalJsonError`. `athlete_identity_digest`,
   `identity_verdict_digest` and `IdentityError::Evidence` follow the same type. Byte equality with
   the previous encoder is the acceptance test, not an aspiration: the shapes the domain digests are
   asserted against `serde_json::to_vec` in `canonical_json/tests.rs`.
4. **`serde_json` stays, as a dev-dependency, pinned to the release the tests compare against.**
   The domain's decoding tests still read fixtures with it, and the purity lane reads
   `--edges normal`, so a test-only dependency is not production reachability. The dev-dependency is
   `serde_json = "=1.0.151"`: it is the oracle the byte-equality tests measure against, so an
   unpinned oracle could drift under a float formatter the domain deliberately holds still.
5. **No caller keeps a second encoder.** `census-review`'s checkpoint digest maps a failure to
   `StoreError::Invariant`, the roster digest in `census-service` maps it to the new
   `CrawlError::Canonical { table, source }`, and the parity-test helper
   `crates/census-service/tests/common/mod.rs::digest` calls the domain function instead of hashing
   `serde_json::to_string` itself. A second implementation would be a second definition of the
   digest contract.

## Consequences

- The `domain purity` lane clears without weakening the ban list, and the domain's normal
  dependency tree no longer contains the persist layer's JSON stack.
- Byte equality is proven for the payload shapes the tests and the parity corpora exercise, not for
  every type in the workspace: synthetic serialize shapes, the production payloads the digests cover
  (`CanonicalAthlete`, `ReviewVerdictRecord`, `ReviewCase`, `SourceObservation`), plus hard-coded
  digest values for those payloads checked against both the domain encoder and the serde_json
  oracle. A new digest input shape must add a byte-equality case before it is persisted; reviewing
  the writer against serde_json's source is not sufficient evidence.
- The roster and contact-proof digest payloads are constructed outside the domain (`Records`,
  `TeamRef`, contact proofs), so their byte equality is covered by shape (`SourceObservation`'s
  internally tagged encoding, `Option`, nested `Vec`) and by the service-level parity corpora rather
  than by a domain-owned pinned digest.
- Map keys outside serde_json's accepted set (strings, numbers, booleans) are refused with
  `CanonicalJsonError::Unsupported` rather than encoded differently. serde_json also refuses them, so
  no digest input may rely on them, and a payload that reaches one is a defect, not a fallback.
- The domain now formats floats itself, so its dependency set carries the float formatter directly.
  A serde_json upgrade that changes its formatter would break byte equality for float-bearing
  payloads, and the byte-equality tests are what would catch it.
