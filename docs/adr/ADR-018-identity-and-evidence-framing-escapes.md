# ADR-018 — Identity and evidence framing escapes its delimiter bytes

**Status:** Accepted (2026-09-30).

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

## Decision

1. **One escape primitive, in the domain.** `write_escaped(bytes, write)` in
   `crates/census-domain/src/model/identifiers.rs`, re-exported `pub(crate)` as
   `model::write_escaped`, writes payload bytes verbatim except that `0x1e` becomes `0x1f 0x01` and
   `0x1f` becomes `0x1f 0x00`. The escape byte is the field delimiter itself, so a framed stream
   decodes uniquely: no payload contributes a `0x1e`, and every `0x1f` in the stream is either a
   field separator or the first byte of an escape pair.
2. **Both framing sites use it, and nothing else frames.** `Id::mint` escapes its prefix and every
   part; `CaseEvidence` escapes statement text, the member label and every member id. The tag
   literals stay literal bytes. A second framing implementation would be a second definition of id
   and digest identity.
3. **Byte stability for delimiter-free payloads is the acceptance test.** For a payload containing
   neither delimiter the escape is the identity function, so the feed is byte-identical to the
   previous framing and no minted id or case digest moves. The pinned values are asserted in
   `crates/census-domain/src/model_tests/framing.rs` against an independent re-implementation of the
   old framing, following the ADR-017 pattern of an external oracle rather than a self-check:
   `sch_97cc5251acc3706e`, `6f965b0486d5a217` and `6575f1a07410e577`.
4. **No stored row is rewritten and no shim is added.** A stored id or digest whose payload carried
   a delimiter was ambiguous under the old framing and now recomputes differently, so such a row must
   be re-derived rather than matched. [ADR-013](ADR-013-fresh-national-source-census.md) already
   makes the national census a fresh run over public sources, and the escape engages only on payloads
   the previous framing could not encode unambiguously.
5. **The contract is proven, not reviewed.** `check_escaping_is_injective` and
   `check_escaped_payload_carries_no_record_separator` are registered in `xtask kani` and prove over
   symbolic payloads that escaping is injective and that no `0x1e` survives it; the escape remains a
   pure byte transform of the hash feed and adds no allocation to the fast path.

## Consequences

- Two review questions or two entities whose evidence previously collided now keep distinct ids.
  The behaviour change is confined to payloads containing `0x1e` or `0x1f`.
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
