# TASK

Seal capture-to-owned-parser runtime envelopes coherently, while preserving parser/resource behavior and persistence bytes.

# OWNERSHIP

Detached source only under this proposal. Input baseline: `49ad9922a9ba24148bd4e73dbb46993ee7f39fce`, confirmed by Root. `source-pins.json` lists 36 changed/new Rust files; `input-pins.json` records all 36 original copies inspected. Root is sole live writer and build runner.

# DO NOT MODIFY

No store/domain/persistence/schema/frozen/policy contract changed. No structural-open or Prepared-v2 recovery adoption. No browser, census, production, publication, or build/test invocation. `candidate` deliberately contains no `net/acquisition/read.rs` or new M1 lock-order test replacement.

# INPUT CONTRACT

Root-adopted `structural-open-consumer-review/PARSER-ENVELOPE.md`, reconfirmed with its verifier. Existing real transport producers supply retained authority; synthetic legacy fixtures remain historical unknown. The final M1 lock plumbing remains Root-owned.

# OUTPUT CONTRACT

- `context.patch`: all cohesive owner/API/caller changes except shared-file deltas. Applies to the copied baseline with `git apply --check` (observed exit 0).
- `candidate/`: exact detached proposed sources, including new `milesplit/owned/outcome.rs` and six real-transport regression tests in `outcome/tests.rs`.
- `read-integration.patch`: precise constructor/view/getter delta against the final M1 candidate read.rs semantic version. Preserve its `with_archive_eligible_fence` and `archive_lock` argument. Root reconciles formatting rather than replacing the file from this proposal.
- `lock-order-tests-accessor.patch`: three access-site updates in M1's added lock_order.rs, from `.capture`/`.verdict` to read-only methods.
- `shared-integration-pins.json`: exact M1 input and proposed combined hashes for those two separate deltas.
- `CONTRACT.md`: model/transition/error/effect/persistence limits.
- `TEST-PLAN.md`: exact Root-owned Moon commands and acceptance oracles.

# ACCEPTANCE

Static-only result. Field/method search found no remaining production raw field consumer outside the acquisition owner and no retained/pure-verdict public promotion builder. Current historical fixture literals were migrated to one cfg(test) historical-only factory, with separate raw captures constructed for metadata variants. All body/error/receipt persistence writers remain intact; getter expressions replace direct reads. New tests acquire via actual TLS transport and real qualification store, and assert original Vec identity, exact metadata/verdict retention, context-refusal precedence, and proof erasure.

Root intentionally retained variant-only capture-context admission. The private interpreter does not compare exact DurableV1 owner/work identity; current exact binding is supplied by the two immediate same-context acquisition-to-interpretation production chains. Public callers cannot inject an envelope, and private Run keeps its immutable context. CONTRACT.md records the limit.

Build, fmt, check, lint, tests, executable negative privacy probes, and full gates remain unexecuted by explicit Root instruction. Source review is not acceptance evidence for runtime or release.

# HANDOFF

Apply the full `context.patch`, preserving M1's extra module registration in acquisition/tests.rs. Apply the semantic contents of both shared-file deltas only after the M1 slice, and keep Root's formatting. Do not copy the entire detached candidate tree over live source; use contextual patches/pins. Run the staged checks in TEST-PLAN and send any compiler/test findings back to this same worker for detached repair.
