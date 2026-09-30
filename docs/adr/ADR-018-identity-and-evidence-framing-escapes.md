# ADR-018 — Identity and evidence framing escapes its delimiter bytes

**Status:** Accepted (2026-09-30); framing correction below supersedes the initial escape encoding.

`Id::mint(prefix, parts)` hashes `prefix` followed by each part preceded by a `0x1f` byte, and
`CaseEvidence::digest` hashes each fact as a tag (`s\x1f`, `m\x1f`), the payload bytes, `0x1f`
between a member label and each member id, and a `0x1e` terminator. Both feeds write payload bytes
verbatim, so a payload that contains a delimiter byte is indistinguishable from the framing that
follows it:

- `CaseEvidence::of(["a\u{1e}m\u{1f}b\u{1f}<id>"])` and
  `CaseEvidence::of(["a"]).with_members("b", [<id>])` produce the same digest, and `ReviewCase`
  takes its identity from that digest, so a statement payload can impersonate a member fact.
- `Id::mint("sch", &["a\u{1f}b"])` equals `Id::mint("sch", &["a", "b"])`, and
  `Id::mint("a\u{1f}b", &["c"])` equals `Id::mint("a", &["b", "c"])`.

Both sites are reachable from source text. `ReviewCase::pending` frames a subject and detail that
adapters derive from captured pages, `identity_index.rs` frames observed identity statements the
same way, and `Id::mint` hashes source-derived parts with no validation at all. `normalize_statement`
does not close the gap: it trims non-alphanumeric characters from token edges and lowercases, so an
interior C0 control survives. `Id` is also `#[serde(transparent)]`, so a persisted id can carry text
that never passed through `mint`. A delimiter byte in a source string is rare, and that is the whole
problem: the failure mode is a silent shared id between two entities or two review questions, which
is the class of defect the census cannot detect afterwards.

ADR-017 recorded this as an open contract question rather than fixing it silently, because the
obvious repair — padding or length-prefixing the framing — moves every stored digest and therefore
every `ReviewCase.id`.

The initial repair used `0x1f` as both escape introducer and field separator. Payload injectivity
alone did not prove framed-tuple injectivity: `["a\u{1e}b"]` still collided with
`["a", "\u{1}b"]`, and `["a\u{1f}b"]` with `["a", "\u{0}b"]`. The repaired contract requires that
escaped payloads contain **neither framing delimiter**, not merely that record separators disappear.

## Decision

1. **One escape primitive, in the domain.** `write_escaped(bytes, write)` in
   `crates/census-domain/src/model/identifiers.rs`, re-exported `pub(crate)` as
   `model::write_escaped`, uses `0x1d` as a distinct escape introducer: `0x1d` becomes
   `0x1d 0x00`, `0x1e` becomes `0x1d 0x01`, and `0x1f` becomes `0x1d 0x02`.
   Neither raw framing delimiter can occur in encoded payloads; the introducer itself is escaped.
2. **Both framing sites use it, and nothing else frames.** `Id::mint` escapes its prefix and every
   part; `CaseEvidence` escapes statement text, the member label and every member id. The tag
   literals stay literal bytes. A second framing implementation would be a second definition of id
   and digest identity.
3. **Byte stability excludes all three reserved bytes.** Payloads without `0x1d`, `0x1e` or `0x1f`
   retain their previous byte feed. Pinned ordinary-input IDs and evidence digests remain unchanged:
   `sch_97cc5251acc3706e`, `6f965b0486d5a217` and `6575f1a07410e577`.
   Runtime regressions compare differently partitioned fields and member facts, not just the encoder
   against itself.
4. **No stored row is rewritten and no shim is added.** A stored id or digest whose payload carried
   any reserved byte must be re-derived under the repaired framing rather than matched to an old
   result. This includes `0x1d`, whose old encoding was unambiguous but now serves as the escape
   introducer. [ADR-013](ADR-013-fresh-national-source-census.md) requires a fresh public-source
   national run. Historical stored rows remain untouched; no automatic mixed-version migration is
   implied.
5. **Proof claims are bounded and separate from hash collision resistance.**
   `check_escaping_is_injective` checks symbolic payload lengths zero through three;
   `check_escaped_payload_carries_no_record_separator` checks that neither delimiter survives
   symbolic four-byte payloads. Both invoke production `write_escaped`. These Kani results and the
   framed public-API regressions establish the exercised bounds, not an unbounded theorem or freedom
   from collisions in the underlying finite hash. The encoder adds no heap allocation.

## Consequences

- Two review questions or two entities whose evidence previously collided now keep distinct ids.
  The changed hash feed is confined to payloads containing `0x1d`, `0x1e` or `0x1f`.
- `Id::mint` and `CaseEvidence::digest` remain the single homes of minted-id and case-digest
  identity; the escape changes how payloads enter their hash feed, not what they hash.
- The escape is not a sanitizer. It preserves payload bytes in the digest feed and in stored ids and
  evidence, so a stored statement that carries a control character still reads back as it was
  captured; the delimiter is only disabled as framing.
- Any future framing that adds a delimiter byte must extend `write_escaped` and its harnesses rather
  than hash payload bytes directly. A new digest input shape is a framing change, not an encoding
  detail.
- Dated command evidence for this decision is recorded in
  [VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md).
