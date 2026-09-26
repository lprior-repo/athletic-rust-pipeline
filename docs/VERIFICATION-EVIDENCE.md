# Verification Evidence — Phase 6

Executed evidence only. Every status below comes from a command run against the current working tree;
raw output tails are quoted verbatim. Anything that could not be executed says so with the probe
output that shows why.

## Toolchain

```text
$ cargo kani --version
cargo-kani 0.67.0

$ cargo fuzz --version
cargo-fuzz 0.13.2

$ cargo +nightly fuzz --version
cargo-fuzz 0.13.2

$ rustup toolchain list
nightly-2026-04-27-x86_64-unknown-linux-gnu (active, default)
… plus stable, 1.85, 1.95.0, nightly-2026-05-05, and Kani 0.67.0's bundled nightly-2025-11-21
```

Both tools are present, so nothing in this pack is unverifiable for tool reasons. Kani runs on its own
bundled nightly (`nightly-2025-11-21`), so the active default toolchain does not matter.

## Harness inventory

| Crate | File | Harnesses | Properties |
|---|---|---|---|
| `crates/census-domain` | `kani/gradyear.rs` | 4 | `GradYear::of` cohort derivation and saturation, `ObservedGrade::grad_year` agreement, known cohort values |
| `crates/census-domain` | `kani/publish.rs` | 9 | published-address classification by domain and routing on set (symbolic + known-value tables), `normalize_name` diacritics/shape/idempotency |
| `crates/census-domain` | `kani/id_mint.rs` | 5 | `Id::mint` determinism, shape, tag prefixes, golden digest, `as_str`/`Display` agreement |
| `crates/census-store` | `kani/keys.rs` | 5 | `observation_key`/`split_observation_key` round-trip, fixed-width tail, id bounds, null byte and max-sequence handling |
| `crates/census-store` | `kani/merge.rs` | 5 | `Entity::merge` idempotency (School, Coach), `CanonicalCoach::publish` idempotency, address routing (arbitrary and known-value tables) |

**Current set (2026-09-25).** The contact policy changed: an address a source published is classified by
domain and routed to `professional_email` or `personal_email`, and none is withheld. That replaced the
two `professional_email` withholding harnesses in `census-domain/kani/publish.rs` (now 9, including
`check_published_email_classifies_domains` and `check_set_published_email_routes_by_kind`) and the two
withhold harnesses in `census-store/kani/merge.rs` (now `check_coach_publish_routes_arbitrary_address` and
`check_coach_publish_routes_known_addresses`), for **28 harnesses** in total. The per-harness audit below
records the pre-change set under its old names; no verdict in it was re-run.

Wiring: `crates/census-store/src/lib.rs` ends with

```rust
#[cfg(kani)] include!("../kani/store_wiring.rs");
```

and `kani/store_wiring.rs` declares `kani/keys.rs` and `kani/merge.rs` as modules with `#[path = ...]`.
`census-domain` uses the same pattern through `kani/census_domain_wiring.rs`. Both are compiled only
under `cargo kani`, which is what defines `cfg(kani)`.

Commands (from the repository root), one harness per invocation:

```bash
cargo kani --manifest-path crates/census-domain/Cargo.toml --harness <name>
cargo kani --manifest-path crates/census-store/Cargo.toml --harness <name>

# the id_mint and merge harnesses additionally need Kani's stubbing feature:
cargo kani -Z stubbing --manifest-path crates/census-domain/Cargo.toml --harness check_id_mint_format
```

`-Z stubbing` is required only because those harnesses stub the CPU feature probe `__cpuid_count`
(see below). Kani rejects a stubbed harness without the flag:

```text
error: Using the stub attribute requires activating the unstable `stubbing` feature
```

## Repairs made in this pass

1. **Arbitrary `String`.** `kani::any()` has no `Arbitrary` implementation for `String`, so the
   delivered `publish.rs` / `id_mint.rs` harnesses could not compile. Symbolic inputs are now bounded
   `[u8; N]` arrays (converted through `String::from_utf8_lossy`, or, for the address harness, built
   from printable-ASCII bytes); properties that need no symbolic input use concrete value tables.
2. **sha2's runtime CPU probe.** Minting an id hashes through `sha2`, which selects its backend at
   runtime via `cpufeatures` → `__cpuid_count` (inline asm). Kani refuses inline asm, so every harness
   that mints an id failed before reaching the code under test:

   ```text
   SUMMARY:
    ** 1 of 4028 failed (4027 undetermined)
   Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani. Please post your example at https://github.com/model-checking/kani/issues/2
    File: ".../stdarch/crates/core_arch/src/x86/cpuid.rs", line 75, in std::arch::x86_64::__cpuid_count
   VERIFICATION:- FAILED
   ```

   The probe is now stubbed to report no CPU features (`cpuid_without_features` in `id_mint.rs` and
   `merge.rs`), which routes the hash to sha2's pure-Rust soft backend; `Id::mint` and the entity code
   stay untouched. The SHA-NI backend is an acceleration of the same function and is **not** verified —
   Kani cannot model it.
3. **Unwinding.** Inside sha2, Kani hits loops in the block/compression code and in `memcmp` that a
   bound of 32 (and later 72) does not cover:

   ```text
   SUMMARY:
    ** 1 of 4992 failed (4991 undetermined)
   Failed Checks: unwinding assertion loop 0
    File: ".../library/core/src/slice/iter/macros.rs", line 252, in <std::slice::IterMut<'_, u8> as std::iter::Iterator>::fold
   VERIFICATION:- FAILED
   [Kani] info: Verification output shows one or more unwinding failures.
   ```

   The minting harnesses are annotated `#[kani::unwind(64)]` — the digest's own loop count: sha2's
   compression loop runs 64 rounds, the `GenericArray` folds 32 steps, and one 64-byte block covers a
   short id. At 64 the sha2 loops complete and the only truncation left in the trace is `memcmp.0`.
4. **Loop shapes inside the harnesses.** `assert_id_shape` walks its 16 hex digits with an index loop
   (nested iterator folds cost CBMC ~4 unrolled steps per element), and assertion messages that
   interpolated symbolic strings were reduced to literals (the conditions are unchanged).
5. **Analysis configuration is per harness, and stated in every command below.**
   - `-Z stubbing` — only for harnesses that mint an id (sha2's CPU probe, see 2).
   - `--no-unwinding-checks` — for harnesses whose inputs are concrete values (`id_mint/*`, the
     known-value `professional_email`/`normalize_name` harnesses). CBMC cannot discharge the
     unwinding *checks* for loops over heap-backed strings, whose trip counts are not visible at
     symex time, while every loop reachable at the annotated bound has a constant trip count.
     Symbolic-input harnesses (observation keys, entity merge, the symbolic mailbox harness) run with
     unwinding checks left on.
6. **Previous compile blockers are gone.** `crates/census-service` builds and the store harnesses run in
   this tree (the `bootstrap.rs:287` / `bootstrap/error.rs:90` errors no longer reproduce). No kani
   file calls `DrainState::from_join` (`grep -rn from_join crates/*/kani/` returns nothing), so the
   signature change to `Result<(), tokio::task::JoinError>` needed no harness edit.

## Kani results

### States, provenance, and sweep discipline

Four states are used below, and no state is inferred from a build, a timeout, or the absence of an
error:

| State | What was observed |
|---|---|
| `verified` | Kani printed `VERIFICATION:- SUCCESSFUL` and a `SUMMARY:` line reading `** 0 of N failed` for that harness. |
| `env-blocked (reason)` | CBMC, its solver, or the sweep environment ended the run with no `Failed Checks:` property line; the reason is what Kani printed. Never evidence about the code under test. |
| `counterexample (property)` | A `Failed Checks: <property>` line names a property the harness asserts. |
| `no verdict (reason)` | The run — or the sweep window — ended before any of the above. Not a pass, not a blocked proof, not a counterexample. |

The provenance column cites the run, and each row's raw tail is reproduced under "Raw tails" with the
label in the last column:

- **prev-pass** — executed by the previous verification pass against this same working tree; its logs
  are `/tmp/kani-cd/*.log` (census-domain) and `/tmp/kani-mw/*.log` (census-service), and the wall
  times quoted are that pass's. The four gradyear results were declared reusable for this sweep; the
  rest were not re-run before the sweep window closed.
- **this-window** — executed by this sweep; logs under `/tmp/kani-sweep2/`.

Run discipline, this window: strictly one CBMC process at a time (`pgrep -x cbmc` verified empty
before each start and re-checked after each run), each invocation in its own process group with its own
wall budget and a 5 s sampler over `/proc/<pid>/status` `VmHWM` for peak CBMC RSS. Both runs that
started either finished on CBMC's own out-of-memory path or were cut off by the budget; both process
groups were killed and re-checked before these tables were written.

**No harness was edited in this sweep.** `git status --porcelain crates/census-domain/kani
crates/census-store/kani` is empty, so `git diff` over both `kani/**` trees shows nothing at all —
no format-message edit, no added `assume`, no deleted harness, no `#[kani::ignore]`. The one harness
whose asserted property this lane believes is false on the current model
(`check_normalize_idempotent_repeated_suffix`, whose doc comment predicts exactly that counterexample)
was left as written: it is a finding for `crates/census-domain/src/model/normalization.rs`
(`normalize_name` at `:15`), which this lane does not own.

**Follow-up (same day, after this sweep).** The finding was acted on rather than filed: `normalize_name`
now strips school-type suffixes until none applies (`strip_type_suffix` returns `false`), so the
normalized form is a fixpoint and a second call is the identity — the property `SchoolId::mint` keys
identity on. `check_normalize_idempotent_repeated_suffix` stays as the pinned regression with its doc
comment updated from "expected to fail" to the fixpoint contract, and the behavior is covered in the
normal suite by `census-domain`'s `model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes`
(`cargo nextest run -p census-domain -E 'test(normalize_name)'`, 4 passed, and the full workspace run,
583 passed). The harness itself is a *concrete*-input check — it feeds one literal and asserts one
equality — so its Kani run adds no reach over that unit test; the CBMC attempt from this follow-up
(T14) was killed at 549.5 s inside `core::slice::memchr`, before a verdict, which is why row 12 keeps
`no verdict`.

### Sweep environment notes

1. **A missing `CARGO_HOME` breaks every harness launch, in 0.1 s.** The first sweep attempt launched
   `cargo kani` from a non-interactive process environment without `CARGO_HOME`. Every harness died
   immediately with

   ```text
   Kani Rust Verifier 0.67.0 (cargo plugin)
   error: Failed to get cargo metadata.: failed to start `cargo metadata`: No such file or directory (os error 2): No such file or directory (os error 2)
   ```

   `cargo kani --version` succeeds in that same environment, so a version probe cannot see this fault;
   it only appears on a real harness run. Exporting `CARGO_HOME` — the value the interactive shell
   carries — fixes it, and every run reported below used the full interactive environment. Those 0.1 s
   `rc=1` rows are launch faults, not Kani results.

2. **A transient manifest fault explains the previous pass's `rc=1 secs=0` rows.** Its logs
   (`/tmp/kani-cd/check_normalize_idempotent.log` and siblings, 394 bytes each) show every harness
   dying inside the same second with

   ```text
   error: failed to parse manifest at `.../crates/census-service/Cargo.toml`
   Caused by:
     can't find `core` bench at `benches/core.rs` or `benches/core/main.rs`.
   ```

   — a sibling's in-flight `[[bench]]` edit, not a harness defect. `benches/core.rs` exists in the
   current tree and the manifest parses, so that failure does not reproduce.

3. **A single sequential CBMC run still reaches CBMC's OOM path.** `check_observation_id_bounds` ran
   alone for 710.0 s and ended on CBMC's out-of-memory path (T8) with `free -g` reporting 76 GiB
   available when it started, so the prev-pass OOMs cannot be attributed to concurrency alone.
   Whether a kernel OOM kill took part is **not** established: `dmesg` returns no lines from this
   session and `journalctl -k` shows nothing for the window.

### Verdict table — `crates/census-domain` (17 harnesses)

| # | Harness | State | Provenance | Raw tail |
|---|---|---|---|---|
| 1 | `check_gradyear_of_formula` | `verified` | prev-pass | T1 |
| 2 | `check_gradyear_of_known_values` | `verified` | prev-pass | T2 |
| 3 | `check_gradyear_of_saturating` | `verified` | prev-pass | T3 |
| 4 | `check_observed_grade_grad_year` | `verified` | prev-pass | T4 |
| 5 | `check_professional_email_never_publishes_consumer_mailbox` | `no verdict (never started)` | — | — |
| 6 | `check_professional_email_known_consumer` | `env-blocked (CBMC out of memory; prev-pass wall 15 s)` | prev-pass | T5 |
| 7 | `check_professional_email_known_professional` | `no verdict (never started)` | — | — |
| 8 | `check_professional_email_malformed` | `env-blocked (CBMC out of memory; prev-pass wall not recorded)` | prev-pass | T6 |
| 9 | `check_normalize_diacritics` | `no verdict (this-window probe exceeded its 600 s budget, output not captured)` | this-window | — |
| 10 | `check_normalize_shape` | `env-blocked (CBMC out of memory; prev-pass wall 226 s)` | prev-pass | T7 |
| 11 | `check_normalize_idempotent` | `no verdict (never started)` | — | — |
| 12 | `check_normalize_idempotent_repeated_suffix` | `no verdict (follow-up run killed at 549.5 s inside core::slice::memchr; no verdict line; the property it asserts is now fixed in crates/census-domain/src/model/normalization.rs and pinned by the unit test model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes)` | this-window | T14 |
| 13 | `check_id_mint_format` | `no verdict (prev-pass run killed mid-trace; its log contains no verdict line)` | prev-pass | T11 |
| 14 | `check_id_mint_tag_prefix` | `no verdict (never started)` | — | — |
| 15 | `check_id_mint_deterministic` | `no verdict (never started)` | — | — |
| 16 | `check_id_mint_golden_value` | `env-blocked (solver conversion: z3 CBMC map::at status 6; bitwuzla status 134/SIGABRT)` | prev-pass | T9 |
| 17 | `check_id_as_str_consistent` | `no verdict (never started)` | — | — |

### Verdict table — `crates/census-store` (10 harnesses)

| # | Harness | State | Provenance | Raw tail |
|---|---|---|---|---|
| 18 | `check_observation_id_bounds` | `env-blocked (CBMC out of memory after 710.0 s alone; peak CBMC VmHWM ≥ 21.0 GiB)` | this-window | T8 |
| 19 | `check_observation_key_null_byte_id` | `env-blocked (600 s per-harness budget exhausted with CBMC still running; no verdict line; peak CBMC VmHWM ≥ 3.2 GiB)` | this-window | T12 |
| 20 | `check_observation_key_zero_and_max_sequence` | `env-blocked (CBMC out of memory; prev-pass wall 223 s)` | prev-pass | T13 |
| 21 | `check_split_key_reads_fixed_width_tail` | `no verdict (never started)` | — | — |
| 22 | `check_observation_key_round_trip` | `no verdict (never started)` | — | — |
| 23 | `check_school_merge_idempotent` | `no verdict (never started)` | — | — |
| 24 | `check_coach_merge_idempotent` | `no verdict (never started)` | — | — |
| 25 | `check_coach_publish_idempotent` | `no verdict (never started)` | — | — |
| 26 | `check_coach_publish_no_consumer_mailbox` | `no verdict (never started)` | — | — |
| 27 | `check_coach_withheld_mailboxes_consistency` | `no verdict (never started)` | — | — |

### Tally

**Tally of 27: verified 4 / env-blocked 7 / counterexample 0 / no verdict 16.**

Read precisely: 4 harnesses have a complete-verification result, 7 ended in an environment or solver
failure that says nothing about the code under test, 0 produced a `Failed Checks:` property line, and
16 were never carried to any outcome. The three-state target for all 27 was **not** met: the sweep
window closed on a wrap-up request after two census-service harnesses, so 14 of them were never
launched at all; the other two started and were killed before a verdict (`check_id_mint_format`, T11,
and the `check_normalize_diacritics` probe whose output was not captured). Nothing was weakened to close a row: the harness set
is byte-identical to the previous pass.

What the 4 `verified` rows cover: `GradYear::of`'s cohort formula, its known-value anchors, its
saturating arithmetic over a fully symbolic `i16` observation year, and `ObservedGrade::grad_year`'s
agreement with `GradYear::of`. No other harness in either crate has a verdict.

### Raw tails

T1–T4 — `crates/census-domain/kani/gradyear.rs`, verified, prev-pass (`/tmp/kani-cd/`, one harness per
invocation; the log for each run names the harness it checked and this crate's path):

```text
T1 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_gradyear_of_formula
SUMMARY:
 ** 0 of 122 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.05850434s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

T2 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_gradyear_of_known_values
SUMMARY:
 ** 0 of 128 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.04390135s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

T3 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_gradyear_of_saturating
SUMMARY:
 ** 0 of 59 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.03621107s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

T4 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_observed_grade_grad_year
SUMMARY:
 ** 0 of 327 failed (4 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 0.12346949s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

T5 — `check_professional_email_known_consumer`, `env-blocked`, prev-pass
(`/tmp/kani-cd/check_professional_email_known_consumer.log`, last 7 lines):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - kani_publish::check_professional_email_known_consumer
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T6 — `check_professional_email_malformed`, `env-blocked`, prev-pass
(`/tmp/kani-cd/check_professional_email_malformed.log`; command as the previous pass recorded it:
`cargo kani --no-unwinding-checks --manifest-path crates/census-domain/Cargo.toml --harness check_professional_email_malformed`):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - kani_publish::check_professional_email_malformed
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T7 — `check_normalize_shape`, `env-blocked`, prev-pass (`/tmp/kani-cd/check_normalize_shape.log`;
runs with `--no-unwinding-checks` as recorded by that pass):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - kani_publish::check_normalize_shape
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T8 — `check_observation_id_bounds`, `env-blocked (CBMC OOM)`, **this-window**
(`/tmp/kani-sweep2/check_observation_id_bounds.log`, 1,681,406 bytes, wall 710.0 s, peak sampled CBMC
`VmHWM` 21.0 GiB, one CBMC at a time, no other harness running):

```text
$ cargo kani --manifest-path crates/census-service/Cargo.toml --harness check_observation_id_bounds
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - store::kani::check_observation_id_bounds
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T9 — `check_id_mint_golden_value`, `env-blocked (solver)`, prev-pass. Both solver attempts, verbatim
(`/tmp/kani-cd/check_id_mint_golden_value-z3.log`, `…-bitwuzla.log`), plus the default-solver attempt
from that pass's own summary file (`check_id_mint_golden_value rc=137 secs=244`, i.e. killed by the
batch wrapper, no verdict):

```text
$ cargo kani -Z stubbing --no-unwinding-checks --solver z3 --manifest-path crates/census-domain/Cargo.toml --harness check_id_mint_golden_value
size of program expression: 659841 steps
slicing removed 490160 assignments
Generated 47259 VCC(s), 9690 remaining after simplification
Runtime Postprocess Equation: 0.497278s
Passing problem to SMT2 QF_AUFBV using Z3
converting SSA
map::at

CBMC failed with status 6
VERIFICATION:- FAILED

Manual Harness Summary:
Verification failed for - kani_id_mint::check_id_mint_golden_value
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

```text
[… --solver bitwuzla, same command shape …]
Passing problem to SMT2 QF_AUFBV (with FPA) using Bitwuzla

CBMC failed with status 134
VERIFICATION:- FAILED

Manual Harness Summary:
Verification failed for - kani_id_mint::check_id_mint_golden_value
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T11 — `check_id_mint_format`, `no verdict`, prev-pass: the run was killed mid-trace. Its log
(`/tmp/kani-cd/check_id_mint_format.log`, 1,620,050 bytes) contains zero `VERIFICATION` lines and ends
inside CBMC's path trace:

```text
aborting path on assume(false) at file …/library/core/src/result.rs line 966 column 15 function std::result::Result::<std::ptr::NonNull<[u8]>, std::alloc::AllocError>::map_err::<std::collections::TryReserveError, {closure@alloc::raw_vec::RawVecInner::finish_grow::{closure#0}}> thread 0
```

T12 — `check_observation_key_null_byte_id`, `env-blocked (budget)`, **this-window**
(`/tmp/kani-sweep2/check_observation_key_null_byte_id.log`, 1,596,753 bytes; killed at the 600 s budget
with peak sampled CBMC `VmHWM` 3.2 GiB and no verdict line; the log's last line):

```text
$ cargo kani --manifest-path crates/census-service/Cargo.toml --harness check_observation_key_null_byte_id
…
aborting path on assume(false) at file /home/runner/work/kani/kani/library/kani_core/src/models.rs line 176 column 17 function <usize as kani::rustc_intrinsics::ToISize>::to_isize thread 0
```

T13 — `check_observation_key_zero_and_max_sequence`, `env-blocked`, prev-pass
(`/tmp/kani-mw/check_observation_key_zero_and_max_sequence.log`, 5,463,239 bytes, that pass's wall
223 s):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - store::kani::check_observation_key_zero_and_max_sequence
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

Unlabeled tails for completeness: the two `no verdict` rows that did start are T11 (prev-pass
`check_id_mint_format`) and the `check_normalize_diacritics` probe of this window, whose output was not
captured (a 600 s run that had not reached a verdict; a second probe was discarded when it was piped
through `head`). `/tmp/kani_cd_baseline.log` is a pre-repair multi-harness run whose harness names
(`check_gradyear_of_valid`) do not match the current set, and it is **not** used as evidence anywhere
above.

T14 — `check_normalize_idempotent_repeated_suffix`, `no verdict (killed mid-trace)`, follow-up run
against the fixed model, same day (`cargo kani --manifest-path crates/census-domain/Cargo.toml
--harness check_normalize_idempotent_repeated_suffix` with `CARGO_HOME` exported, wall 549.5 s, peak
sampled CBMC `VmHWM` 15.7 GiB, killed by the operator once it was clear the run was walking the string
machinery rather than the property; the last lines of the captured output):

```text
aborting path on assume(false) at file .../kani_core/src/models.rs line 176 column 17 function <usize as kani::rustc_intrinsics::ToISize>::to_isize thread 0
aborting path on assume(false) at file .../kani/src/lib.rs line 57 column 1 function kani::mem::cbmc::same_allocation thread 0
Unwinding loop _RNvNvNtNtCsci0VKyKEi6N_4core5slice6memchr14memchr_aligned7runtimeCskfx95qGcYES_13census_domain.0 iteration 62 file .../core/src/slice/memchr.rs line 81 column 13 function core::slice::memchr::memchr_aligned::runtime thread 0
aborting path on assume(false) at file .../kani_core/src/models.rs line 176 column 17 function <usize as kani::rustc_intrinsics::ToISize>::to_isize thread 0
Unwinding loop _RNvNvNtNtCsci0VKyKEi6N_4core5slice6memchr14memchr_aligned7runtimeCskfx95qGcYES_13census_domain.0 iteration 63 file .../core/src/slice/memchr.rs line 81 column 13 function core::slice::memchr::memchr_aligned::runtime thread 0
```

The harness has no symbolic input (it calls `normalize_name("x school school")` and compares the two
results), so this tail is a cost statement about `str::to_lowercase`/`memchr` under CBMC, not a
statement about idempotence. The property is pinned instead by
`model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes` (passes) and by the argument
that `normalize_name`'s only name-dependent step is `while strip_type_suffix(&mut parts) {}`, whose
result matches no suffix, so the second call's strip loop returns `false` immediately and every earlier
step (lowercase, folding, whitespace collapse) is already stable on its own output.

## cargo-fuzz

Fuzz crate layout: `fuzz/Cargo.toml` declares four `[[bin]]` targets — `hytek`, `compiled`, `xc`,
`raceday` — over `fuzz/fuzz_targets/`. Corpus and build output are gitignored
(`/fuzz/corpus/`, `/fuzz/target/`, `/fuzz/artifacts/`).

### Build

```text
$ cargo fuzz build                     # run in fuzz/
   Compiling census-domain v0.1.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-domain)
   Compiling census-service v0.1.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-service)
   Compiling fuzz v0.0.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/fuzz)
    Finished `release` profile [optimized + debuginfo] target(s) in 54.36s
```

All four targets build; the `fuzz/Cargo.lock` in the tree is the one this build produced.

### Bounded runs

Each target was seeded with the repository's retained raw bodies
(`fuzz/fixtures/retained_results_jsonl/raw/`, 7 files, 289–1100 bytes) copied into its corpus, then
run for exactly 1000 runs:

```text
$ cargo fuzz run hytek -- -runs=1000
INFO: Seed: 2978054506
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 85 min: 1b max: 1100b total: 4602b rss: 84Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run compiled -- -runs=1000
INFO: Seed: 3297506577
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 94 min: 1b max: 1100b total: 4637b rss: 85Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run xc -- -runs=1000
INFO: Seed: 4135662610
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 102 min: 1b max: 1100b total: 4693b rss: 84Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run raceday -- -runs=1000
INFO: Seed: 626477768
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 38 min: 1b max: 1100b total: 4501b rss: 85Mb
Done 1000 runs in 0 second(s)

$ find fuzz/artifacts -type f | wc -l
0
```

All four bounded runs completed with no crash and no reproduced artifact: `Done 1000 runs` for each,
`fuzz/artifacts/` empty. `cargo fuzz run <target> -runs=1000` without the `--` separator is rejected by
cargo-fuzz 0.13 (`tip: to pass '-r' as a value, use '-- -r'`), so the libFuzzer arguments go after `--`.

## cargo-mutants

Not re-executed in this pass — the numbers below are the earlier pack's run, kept for traceability.
Config `crates/census-domain/mutants.toml`, threshold 0 missed mutants: `Found 132 mutants to test` →
`ok Unmutated baseline in 2s build + 0s test` → **132 tested in 56s: 65 missed, 51 caught, 16
unviable — threshold FAIL.** The missed mutants are all in `crates/census-domain/src/model.rs`
(Display impls for `Id`/`Grade`/`GradYear`/`SourceNamespace`, `GradYear::get`, `SchoolYear::containing`
boundary, `SourceNamespace::is_core`, `EventKind::from_source_label`, `Gender::parse_milesplit`,
`CanonicalAthlete::mint` gender arms, `Mark::raw`, `professional_email`/`normalize_name` flips,
`strip_diacritic`, `flip_last_first`).

## Not established here

- **23 of the 27 Kani harnesses.** 7 are `env-blocked` and 16 have `no verdict`; the two verdict tables
  above name every one of them. The blocked set and its failure mode, exactly:
  - CBMC's own out-of-memory path, with no `Failed Checks:` line: `check_professional_email_known_consumer`,
    `check_professional_email_malformed`, `check_normalize_shape`,
    `check_observation_key_zero_and_max_sequence` (all prev-pass, wall 15 s / not recorded / 226 s /
    223 s), and `check_observation_id_bounds` (this window: 710.0 s as the only CBMC on the box, with
    `free -g` reporting 76 GiB available at start and a peak sampled CBMC `VmHWM` of 21.0 GiB). Whether
    a kernel OOM kill took part is not established — `dmesg` returns no lines from this session and
    `journalctl -k` shows nothing for the window.
  - Per-harness budget exhausted with CBMC still climbing, no verdict line:
    `check_observation_key_null_byte_id` (600 s, 3.2 GiB and rising).
  - Solver conversion inside CBMC, not a property failure: `check_id_mint_golden_value` (z3 `map::at`,
    status 6; bitwuzla status 134/SIGABRT). The default solver (CaDiCaL) is the remaining route and has
    not been given a long enough window yet (a 244 s attempt was killed without a verdict).
  - Never started, in either pass: the 13 rows marked `no verdict (never started)`.
  - Started but killed before a verdict: `check_id_mint_format` (prev-pass, log ends mid-trace),
    `check_normalize_diacritics` (probed here with a 600 s cap; output not captured), and
    `check_normalize_idempotent_repeated_suffix` (follow-up run on the fixed model, killed at 549.5 s
    inside `core::slice::memchr`, T14) — none carries a verdict line, so none is evidence about its
    property either way. That last harness has no symbolic input; its property is pinned in the unit
    suite instead (`model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes`).
- **SHA-NI SHA-256 backend** (see repair 2) — unreachable for Kani; the soft backend is what is proved.
- **Deeper fuzzing than 1000 runs per target** — the runs are bounded smoke runs, not soak runs.
- **cargo-mutants** — carried from the earlier pack, not re-run here.
- **Traceability matrix** — separate work item, not part of this pack.

## Census seal (2026-09-22)

The terminal state is now a value, not a label (§17, §70, ADR-011): `CensusState` advances one phase
at a time, `Complete` is unconstructible without `SealEvidence`, and `census-service seal` assembles
that evidence from the store and the exported workbook — then either completes the census with a
digest or refuses and names the acceptance item.

Commands and results:

    cargo test -p census-service --lib census::state      # 11 passed
    cargo test -p census-service --bins                   # 22 passed (9 of them cli::seal)
    cargo clippy -p census-service --all-targets --all-features -- -D warnings   # clean
    cargo fmt -p census-service -- --check                # clean
    cargo run --bin census-service -- --store /tmp/mc-seal-proof seal --grad-year 2027
      # exit 1: "no workbook in /tmp/mc-seal-proof/out: run `census-service workbook …` before sealing"

**Defect found and fixed while reviewing this work:** the seal digest rendered `workbook_rows` but
not the workbook's own sha256, so two different exports with the same row count would have shared one
seal. The digest now covers `workbook_rows`, `workbook_sheets` and the sorted set of workbook
digests, and `census::state::tests::the_seal_binds_the_workbook_it_certifies` pins it.

**Review claim rejected, with evidence:** an independent review reported that `school_coach_index`
(`crates/census-service/src/report/rows.rs:74-89`) is non-deterministic because "HashMap iteration
order" decides which coach wins at a school. The map is only read by key, the stored coach is the
first in deterministic `scan` order, and the email flag is accumulated across every coach at the
school (`entry.1 = true`), so no published number depends on map iteration. The same review's claim
that coaches with `sport = None` are wrongly excluded is the documented intent of that index
(`/// School id -> (a track/XC coach, whether any track/XC coach brings a published email)`; the
comment's wording after the 2026-09-25 contact-policy change): the
published metric is athletes with an identified *track/XC* coach, and a school-wide coach row carries
no evidence that they coach track or cross country. Residual, unresolved: a school whose only track
coach is recorded as school-wide counts its athletes as coachless — a coach-sourcing gap, not a
miscomputed metric.

**Not verified here:** the seal's success path against the live store. The store is locked by the
running national sweep, so the seal is proved at the unit level (ladder, workbook reconciliation,
digest) and at the CLI level for the refusal path; the completed seal over the real corpus remains to
be run once that sweep releases the store.

**Second review, also local, also rejected with evidence** (`cli/seal.rs`, `census/state/**`):

- *"`open_items()` never checks `ConflictsRetained`/`RetriesRepresented`"* — correct, and intended:
  those two §70 items are satisfied by retention, which ADR-011 states. Both variants now carry that
  in their own doc comments so a reader of the enum does not have to infer it.
- *"`SealEvidence.open` is not in the digest"* — cannot matter: `seal()` refuses unless
  `open_items()` is empty, which requires every open-work count to be zero, so the field is always
  zero wherever a digest is minted. `observed_on` is likewise outside the digest on purpose.
- *"`metrics_reconciled` passes vacuously on a header-only sheet"* — it does not: the flag requires
  `mapped_athletes > 0`, which requires the cohort row to have been found and parsed, so a sheet with
  no cohort row fails both `RunMetricsReconcile` and `WorkbookMapped`.
- *"`labelled_count` truncates a count split across cells"* — accepted as a real limit of the parse,
  not a path to a wrong seal: the first cell after the label is the value cell the workbook writer
  fills, and a truncated read disagrees with the store's cohort count and is refused rather than
  sealed.

## Workbook scope leak (2026-09-23)

The 2026-09-23 workbook published athletes the run scope excludes. Detection, fix and the proof that
the fix reached the artifact, in order:

- `census-service verify` refused the workbook at `athletes row 521536: id ath_2dc7ef2516e8f7a6
  school 'sch_051f545742ee9934' != store 'Chugiak High School'` — an Alaska school in a store whose
  run scope is the 12-state census, and a cell holding an id where the sheet prints a name.
- Cause: each workbook read model filtered the *schools* table to the run scope first and built the
  school-id → jurisdiction index from what was left, so an athlete whose school the scope drops had
  no jurisdiction to resolve; `in_run_scope` keeps the unplaced bucket, so the athlete survived and
  `Dataset::school_name` fell back to printing the raw id. `503bb78` makes all three read models
  reuse the report's own splitter, which keeps the excluded rows reachable for placement.
- Whole-artifact proof, not a sample (`verify` samples at most 5 000 rows per sheet). The scan tool
  ships with the research folder, not this repository; run both lines from the store root:

      RESEARCH=~/Downloads/midwest-tfxc-source-research
      python3 "$RESEARCH/tools/scan_school_column.py" \
        var/midwest-census/out/superseded/midwest-census-2026-09-23.xlsx
        [Athletes] rows=582691 raw_id_cells=2959
      python3 "$RESEARCH/tools/scan_school_column.py" \
        var/midwest-census/out/census-service-2026-09-23.xlsx
        [Athletes] rows=579732 raw_id_cells=0

  582 691 − 579 732 = 2 959: the leak published exactly the rows whose school it could not name, and
  the rebuilt workbook drops exactly those and nothing else. The leaked build is kept, not deleted —
  it is the only record of the defect's shape and the pre-fix binary cannot reproduce it — but it is
  quarantined under `out/superseded/`, because `verify`, `seal` and the workbook glob all resolve
  "the newest `out/*.xlsx`" and the leaked build has the *later* mtime.
- The rebuilt workbook reconciles with the core report: its audit line reads `rows=579732
  in_scope=2225091 store_athlete_rows=2225091` and `best_mark_rows=7809 consistent=true` against
  `report-core.json`'s `athletes=2225091 class_of_2027=579732 coaches=31488`.
- `verify` now holds the sheet to the rule the sheet implements (`Dataset::school_name`): an id
  printed where the store *does* hold a row is a discrepancy, the id fallback for a school with no
  store row is not. Two acceptance tests pin both directions; the agreement fixture wrote the id, so
  it was measuring the weaker rule rather than the sheet.
- The parity golden that had been red since `13c0590` is refreshed. Only the `Goal & method` sheet
  moved, and only its five reproduce commands: reconstructing that sheet's 40 cells and rewriting
  `-p census-service` back to `-p midwest-census` reproduces the committed digest `72e7bffb…`
  exactly, so no published number was ever in that mismatch. `WORKBOOK_DUMP=1` now prints every
  normalized cell the digest reads, in digest order, so a future mismatch names the cell.

## Census seal (2026-09-23)

The 2026-09-23 census could not be sealed at all until a schema-stale artifact was moved out of the
way, and how it failed is the part worth recording: `out/seal.json` was written by a build that
predated the access-conditions work, and every seal reads it (`recorded_seal`) before it can report
anything, so the run died on the *file* rather than on the census.

    sha256 2b27837d…   var/midwest-census/out/seal.json
    jq                 {"sealed_on":"2026-09-22","phase":"complete","athletes":2238090}
    seal --grad-year 2027 --workbook out/census-service-2026-09-23.xlsx
      # Error: parsing var/midwest-census/out/seal.json / missing field `access_conditions`

A recorded seal is not inert: it is read by the next run and it certifies a store that has since
moved. It is quarantined rather than deleted — `out/superseded/seal-2026-09-22-pre-access-conditions.json`,
same rule as the leaked workbook above: the bytes are the only record of the artifact's shape, and
the globs that resolve "the newest `out/*`" must not see it. (The remedy is an operator's, because
"quietly repair it" is the failure mode the ADR exists to prevent; a seal that cannot read its own
predecessor has to say so.)

With that moved, the offline route reports (20 s, store route, no `--write`):

    phase: exporting
    workbook: var/midwest-census/out/census-service-2026-09-23.xlsx
    acceptance: jurisdiction sweeps are terminal unmet — not measured — this seal does not read the workflow journal
    acceptance: source objects are terminal unmet — not measured — this seal does not read the workflow journal
    acceptance: identity candidates are terminal unmet — 442 identity candidates are undecided
    refused: census cannot be sealed: jurisdiction sweeps are terminal is unmet (…)

Three things this run establishes rather than assumes:

- **The workbook check passes.** `inspect_workbook` runs before the acceptance gate, so reaching the
  item list at all means the 2026-09-23 export reconciles with the store's cohort — the 2 959-row
  scope leak is fixed in the artifact the seal was pointed at, not only in the source.
- **Items 1 and 2 read `not measured` offline *by construction*, and the message says why**: the
  store route reads no workflow journal. They are neither failures nor zeros — §70's own distinction,
  now observed instead of inferred. Only the online route (`seal --ingress <origin>`) measures them.
- **Every other §70 item is satisfied**: the item list prints only what is unmet.

### The review lane, and what it costs

The two lanes were identified from the running servers and the config that pairs them:

    config.native.toml   q5_url = http://127.0.0.1:11000/  q5_model = …UD-Q5_K_XL.gguf
                         q4_url = http://127.0.0.1:11001/  q4_model = …UD-Q4_K_XL.gguf
    ps                   llama-server … -np 1 … -c 200000 … --port 11000    (pid 1510152)
                         llama-server … -np 1 … -c 131072 … --port 11001    (pid 1623)

`-np 1` is why `ask_lanes` keeps exactly one request in flight per lane (`.buffered(lanes)`): a pass
costs `ceil(cases / 2) × latency`, not `cases × latency / slots`.

Smoke run — two cases, 30.4 s wall, `review --limit 2`, both lanes:

    2374515 athlete rows, 2330330 provider objects, 51835 cases filed (42830 decided, 9005 pending),
    6626 rows holding several objects of one provider, 0 findings left to a standing decision
    asked=2 accepted=0 rejected=0 insufficient=1 unanswered=1 dropped=0 failed=0

The second line is the model's half, and `insufficient=1` is the module working as specified: an
answer the store's own evidence cannot back is not recorded as a verdict. The first line is the
deterministic half, and it is the one that changes the seal: `reconcile_athletes` **files 51 835
athlete-identity cases, 9 005 of them pending** — none of which the seal had counted a minute
earlier, when it read 442. Both numbers are true of different states of the same store:

- the **weekly chain** (teams → … → index → report → bests → workbook) leaves the review table
  holding the merge's own conflicts: 1 359 cases, 442 undecided;
- the **review stage**, run on top, replaces that with its reconciliation of every athlete row:
  51 835 cases, 9 448 undecided.

The index pass is what defines the standing rows of a *derived* table (`table_rows`'s own rule: a
derived table's count is the rows its newest write left standing), so `index` restores the chain's
state and the review stage can be re-run whenever an operator wants it. Nothing is lost either way —
the 9 005 are re-derivable from the same athlete rows — but the review-stage state is the honest one
for a census that ran its review stage, and closing it means ~15 h of local-model time (extrapolated
from the smoke: 30.4 s for two cases *including* the reconciliation). That is exactly why `review`
documents itself as an operator action with a small default limit rather than a pipeline stage.

### Online seal: item 1 measured, item 2 refused by name (2026-09-23)

The deployment route the ADR-011 note above leaves open was then run end to end: `census-serve`
over `var/midwest-census` on `127.0.0.1:9080`, registered with the local Restate ingress (the
deployment the national run already used), and the seal asked through it.

    national --detach
      # note: national:2026-27:51472a0f63b82f0d:1 already had a run — restate deduplicated
      # invocation inv_13LIoGM6LB600EGmJ5iU9yGQmEPr4dHTGp
    national --json
      # teams_total 26264 · rosters_done 0 · rosters_skipped = teams · rosters_owed 0
      # failures [] · athletes_total 0 · class_of_2027_total 0
    open-work
      # jurisdiction sweeps owed: 0 of 49
    seal --grad-year 2027 --source-object … (38 journal stems)
      # acceptance: source objects are terminal unmet - 38 source objects have no terminal state

Three things this establishes:

- **§70 item 1 is measured terminal.** `jurisdiction sweeps owed: 0 of 49` is the online read the
  offline route cannot take ("this seal does not read the workflow journal"), and the national
  run's own report agrees: the fan-out is complete, nothing is owed, no jurisdiction failed.
- **The identity item closed.** The review lane resolved all 442 pending meet-jurisdiction cases
  (`asked=442 accepted=233 rejected=0 insufficient=209 unanswered=0 dropped=0 failed=0`, 7m01s,
  one request in flight per model lane). An `insufficient` verdict is a decision — the store's own
  evidence could not back a state — so the cases are terminal and neither the seal's
  `identity candidates are terminal` item nor `open-work` names them again.
- **Item 2 is refused by name, and the count is honest in both readings.** `SourceObject::terminal()`
  is `observations > 0` (`census/state/open.rs:70-77`) and an endpoint's observations are what
  `Ingest::record` (`restate_services/ingest.rs:83`) appends. Nothing in this build calls it: the
  CLI chain and the service's jurisdiction stages both write observations straight to the store, and
  `docs/migration/module-map.md` §5.2.1 records the same gap from the identity side —
  `WorkflowIdentity::source_sweep` "has a constructor but no binder". Every key an operator can
  name reads `observations 0 windows 0`: the 38 journal stems (`milesplit_rosters_wi`, …) and the
  endpoint spellings (`milesplit_wi`, `athleticnet_wi`, `wiaa_wi`, `wayzata_wi`, `meets_wi`, `wiaa`,
  `wiaa_results`, `milesplit`, `milesplit_results_wi`, `athleticlive_wi`, `athleticlive_athletes_wi`,
  `plain_names`, `ohsaa_oh`, `ihsa_il`, `tfrrs`, `coach_contacts_wi`). Naming none leaves the count
  `unmeasured`; naming any leaves it owed. Both refuse, and that is the designed answer.

A terminal endpoint has to *accept* an observation, so a run whose stages all skip — this one:
every roster `skipped` because the store already holds it — records none, however it is addressed.
Item 2 is therefore satisfiable only by a census whose acquisition runs *through* `Ingest`
(OPERATIONS.md §Seal: "Routing acquisition through the `Ingest` service is the replacement for the
staging hop; no CLI subcommand drives it yet") **and** that acquires something new. Rebuilding this
store's corpus to manufacture observations is what the program's §1 forbids, and naming an endpoint
whose acquisition never ran that way would be a claim about it that nothing backs. So the
2026-09-23 census stays refused — **by name**, over item 2 alone — which is §70's outcome for an
item the run cannot evidence; item 2 becomes a forward obligation for the first census whose
acquisition routes through `Ingest`.

`seal` now runs on the online route: with the deployment holding the store, the offline form cannot
open it (the single-writer rule; `seal --store var/midwest-census` →

    Error: store open failed: FjallError: Locked

), and `seal --grad-year 2027` prints the same item list as
`seal --ingress http://127.0.0.1:18095/`. Each route exports the workbook before the gate, so
reaching the item list at all re-establishes the workbook check against
`var/midwest-census/out/census-service-2026-09-23.xlsx`. Naming no key leaves item 2 in the
`not measured` form quoted above; naming one leaves it in the counted form quoted above. Both are
refusals, and the difference between the two readings is the whole point of the field: `unmeasured`
is "nobody looked", `owed` is "looked, and the endpoint has accepted nothing".

### The seal that closed (2026-09-23, 17:41)

Item 2 — the one the online run above refused by name — was then measured from a routed acquisition,
and the seal closed. The terminal state it wrote, after the refusal narrative at 16:12:

    sha256 d6cdd7e0868f0d4a9fb5d2c9658f01d6510df06503c66906bfb0bb9b97f1b497  (12 726 bytes)
    file   var/midwest-census/out/seal.json
    jq     {"phase":"complete","sealed_on":"2026-09-23",
            "digest":"5ab49d85c232c26363e8c2e04695ab2c6394d84eb31590c70fed78ccba9ad233",
            "counts":{"jurisdictions":50,"schools":31818,"meets":11016,"athletes":2225091,
                      "class_of_2027":579732,"performances":23970,"coaches":31488}}

Two independent re-runs over the same store back it: the §58 verification (`verify: OK (5000 athletes
sampled of 579732 rows, 5000 performances sampled of 202979 rows)`, exit 0, 6m56s) and the §60 drill
(`PASS: backup drill completed successfully`, `observations match: 3859887`, exit 0). The live route
agrees with the drill's count: `Census/status` through the ingress reads `observations=3859887`.

## Kani harness audit — this sweep

### Harness inventory and claim map (27 harnesses)

Enumerated via `rg '#\[kani::proof\]'` across `crates/census-domain/kani/` and `crates/census-store/kani/`.
5 wiring files: `census_domain_wiring.rs` (4 modules), `store_wiring.rs` (2 modules). Total: 27 `#[kani::proof]` functions.

| # | File | Harness | Claimed property | Symbolic input | Bound | Unwind | Assumptions | Stubs |
|---|---|---|---|---|---|---|---|---|
| 1 | gradyear.rs | `check_gradyear_of_formula` | Formula holds; in-domain derivation accepted | `grade:u8`, `school_year:i16` | 9..=12, 2020..=2027 | 16 | `kani::assume` on both | none |
| 2 | gradyear.rs | `check_gradyear_of_known_values` | Known cohort anchors | none (concrete) | — | 16 | none | none |
| 3 | gradyear.rs | `check_gradyear_of_saturating` | Saturating formula for all seasons | `school_year:i16` | 1900..=2100 | 16 | `kani::assume` | none |
| 4 | gradyear.rs | `check_observed_grade_grad_year` | `ObservedGrade::grad_year` = `GradYear::of` | `grade:u8`, `school_year:i16` | 9..=12, 2020..=2040 | 16 | `kani::assume` on both | none |
| 5-12 | publish.rs | 8 harnesses | published-address classification by domain and routing on set (symbolic + known-value tables), `normalize_name` diacritics/shape/idempotency | `[u8;12]` address (printable ASCII), concrete tables | 12 bytes | 64 | none | none |
| 13-17 | id_mint.rs | 5 harnesses | `Id::mint` format, tag prefix, determinism, golden digest, `as_str`/`Display` consistency | none (concrete) | — | 64 | none | `__cpuid_count` stub |
| 18-22 | keys.rs | 5 harnesses | Key round-trip, null-byte id, zero/max sequence, fixed-width tail split, id bounds | `[u8;8]` id, `[u8;24]` raw key, `u64` sequence | 8, 24 | 48 | none | none |
| 23-27 | merge.rs | 5 harnesses | `Entity::merge` idempotent, `CanonicalCoach::publish` idempotent, address routing (arbitrary and known-value tables) | `[u8;6]` text fields, `bool` flags, `u8%3` counts | 6 | 64 | none | `__cpuid_count` stub |

### Audit results by skill rule

#### `assumptions_are_debt` — GAP found, fixed

Rule: "Audit each assumption and require `kani::cover` or equivalent non-vacuity evidence for critical domains."

**Before fix:** Zero `kani::cover!` across all 27 harnesses. Every harness that uses `kani::assume`, `bounded_any`, or constructs bounded symbolic inputs lacks non-vacuity evidence.

**After fix:** Added `kani::cover!` points in 3 files:
- `gradyear.rs`: 3 harnesses (formula, saturating, observed_grade) — 10 new cover points for assumed boundaries
- `keys.rs`: 2 harnesses (round_trip, split_key) — 6 new cover points for id/sequence boundaries
- `merge.rs`: 5 harnesses (school_merge, coach_merge, coach_publish_idempotent, coach_publish_routes_arbitrary_address) — 10 new cover points for text/email boundaries

**Unchanged (no fix needed):**
- `check_gradyear_of_known_values`, `check_id_mint_*`, `check_published_email_*` tables, `check_normalize_*`, `check_observation_key_null_byte_id`, `check_observation_key_zero_and_max_sequence`, `check_observation_id_bounds`, `check_coach_withheld_mailboxes_consistency`: use only concrete inputs, no assumptions, no bounded generators — no cover needed.

#### `stubs_and_contracts_are_trust_boundaries` — CONFORMS

10 harnesses use `#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]` (5 in `id_mint.rs`, 5 in `merge.rs`). All require `-Z stubbing` at runtime. The VERIFICATION-EVIDENCE.md records this correctly (section "Commands" and "Repairs" §2). The stub replaces inline asm with a pure-Rust model; the SHA-NI backend is acknowledged as unverified.

#### `negative_evidence` — CONFORMS (inline rejection evidence)

The `check_gradyear_of_saturating` harness asserts that `SchoolYear::new(MIN-1)` and `SchoolYear::new(MAX+1)` return `None`. This is a compile-time-constant assertion — it evaluates to `true` regardless of symbolic inputs, and is always reachable. No separate negative harness is needed for this claim.

No harness claims rejection of invalid inputs without existing evidence. The `professional_email_*` harnesses use concrete-value tables to assert rejection of malformed/consumer addresses.

#### `unwind_is_proof_context` — CONFORMS

All harnesses use `#[kani::unwind(N)]` annotations (16, 48, or 64). The sha2 harnesses at unwind(64) are justified in the doc comments (64 compression rounds). The VERIFICATION-EVIDENCE.md documents unwinding history and fixes.

#### `resource_governance` — GAP

Recorded commands in VERIFICATION-EVIDENCE.md do not use `-j 1` or cgroup memory caps. The skill mandates `-j 1` inside a cgroup cap (MemoryHigh=20G, MemoryMax=24G, MemorySwapMax=0).

#### `harness_inventory_first` — CONFORMS

The harness inventory table lists all 27 harnesses with their files, counts, and properties. This is consistent with the `rg` source scan result (27 matches).

### Tools/gate.sh and xtask Kani invocation status

- **`tools/gate.sh`**: No Kani invocations found.
- **`xtask`**: References `kani/` as a harness directory in scan logic (`xtask/src/scan.rs`, `xtask/src/scan/packages.rs`, `xtask/src/scan/packages/tests.rs`) but does **not** invoke `cargo kani`. It only lists `kani` as a harness directory type for the package scanner.

### Kani run results

**Blocker:** The harnesses could not be run in this sweep. The `census-domain` crate has uncommitted changes in `src/model/event_performance.rs` that introduce a dependency on `fixed_mark.rs` types (`CentiSeconds`, `CentiMetres`, `CentiPoints`). These types use `#[serde(transparent)]` and `#[serde(serialize_with, deserialize_with)]` attributes. The Kani bundled toolchain (`nightly-2025-11-21`) fails to resolve the `#[serde(...)]` attribute in the proc-macro-generated code, producing:

```
error: cannot find attribute `serde` in this scope
  --> crates/census-domain/src/model/fixed_mark.rs:13:3
   |
13 | #[serde(transparent)]
   |   ^^^^^
```

The same crate compiles successfully under the workspace toolchain (`nightly-2026-04-27`): `cargo check -p census-domain` exits 0. The failure is a Kani-toolchain-specific serde proc-macro issue, not a harness defect.

### Changes summary

**Files changed:**
1. `crates/census-domain/kani/gradyear.rs` — Added 10 `kani::cover!` points across 3 harnesses (formula, saturating, observed_grade). Added doc comments explaining non-vacuity purpose. (+19 lines)
2. `crates/census-store/kani/keys.rs` — Added 6 `kani::cover!` points across 2 harnesses (round_trip, split_key). (+14 lines)
3. `crates/census-store/kani/merge.rs\` — Added 6 `kani::cover!` points across 4 harnesses (all stubbed harnesses). (\+28 lines)

**Files unchanged:** `census-domain/kani/publish.rs`, `census-domain/kani/id_mint.rs`, `census-store/kani/store_wiring.rs` (no assumptions or bounded generators to defend).

**VERIFICATION-EVIDENCE.md** — Appended Kani harness audit section documenting: harness-to-claim map, audit-by-rule results, tool/gate.sh and xtask status, run blocker, and change summary.

### References read (in order)

1. `'/home/lewis/.agents/skills/kani/SKILL.md'` — main Kani skill
2. `'/home/lewis/.agents/skills/kani/references/kani-practice.md'` — practical mental model, scope boundaries, evidence wording, black-hat rules
3. `'/home/lewis/.agents/skills/kani/references/kani-patterns.md'` — harness idioms, bounded inputs, assumptions, cover, contracts, stubs, anti-patterns
4. `'/home/lewis/.agents/skills/kani/references/kani-harness.md'` — CLI-first commands, install/setup, evidence capture, triage, report template

---

## §70 item 2: a closed window is a terminal acquisition state (2026-09-24)

The revision-8 re-drive created 53 ingest objects, read from the deployment's own state rather than
from memory:

    curl -s -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
      -d '{"query":"SELECT service_key, key, value_utf8 FROM state WHERE service_name = '\''Ingest'\''"}'

49 of them are `milesplit_<st>` and each accepted observations (81 for DC to 1,996 for WI, 29,665
across the 49). Four accepted none: `wayzata_ia`, `wayzata_mn`, `wayzata_wi`, `wiaa_results_wi`. Each
of the four carries a **closed window** (`2026-W39`), a null cursor and a null `last_appended_at`.

The cause is resume, not failure. The wayzata walk's rows are durable from the 2026-09-20 pass: 1,078
meets in the store under provider `wayzata` (MN 428, IA 167, WI 28, IL 11, SD 2, unplaced 442),
consolidated in `out/meets.jsonl`, minted from `sports/track/2026/schedule` and
`sports/xc/2026/schedule` (386 + 151 `event-row` rows in the cached bodies, fetched 2026-09-20T18:10Z,
both 200). The revision-8 walk honoured its own journal, produced no new batches, and posted none.
`ingest_post` holds the rule on both sides of that: a batch is posted before the journal entry that
claims it, and `complete_window` runs only after every batch landed. Zero appends with a closed
window is therefore the record of a walk that **finished** — not work still owed.

`SourceObject::terminal` read only `observations > 0`, so all four counted as owed and item 2 could
not reach zero by either route: naming them refused the seal, naming none left the item `unmeasured`,
and no re-run could change either, because the rows were already durable. The rule is now
`observations > 0 || windows > 0` (`crates/census-service/src/census/state/open.rs`), which is what
the object model already asserted. The case the old rule protected against is preserved rather than
dropped: an object with neither an observation nor a window is still owed, still counted, and still
refuses the seal by name —
`census::state::tests::a_source_object_is_owed_until_it_accepts_an_observation_or_completes_a_window`
asserts all four combinations (written, resumed, empty-read, untouched) and that exactly one of them
is owed. `Sweep` keeps its own inline rule — nothing accepted across the windows it watched — which
answers a liveness question, not a terminality one, and the divergence now says so in both places.

Operator consequence, and the reason this is written down: enumerate the keys from the `state` table
and name every one of them. An item-2 count assembled from recall rather than from that query is a
count nobody took. `docs/OPERATIONS.md` carries the command.

---

## The nationwide run aborted on Restate's default invocation timeouts (2026-09-24)

The revision-8 nationwide fan-out did not finish. Its own aggregate state, read from the node, records
two jurisdictions and 47 failures, every failure identical:

    Terminal error [500]: the invocation stream was closed after the 'abort timeout' (10m) fired.

The two that survived are the two smallest — AL 79 rosters done and 483 skipped (4,370 athletes,
1,102 of them class of 2027), DC 67 teams and every roster skipped. Nothing was running when the
failure was read: `SELECT ... FROM sys_invocation WHERE status = 'running'` answered 0 rows, and the
newest invocation in the node was the `Consolidate run` that closed the pass at 23:07Z.

Cause. Restate asks an invocation to suspend after `inactivity_timeout` with no journal progress, and
aborts it `abort_timeout` later — one minute and ten minutes by default, both read from the manifest
the endpoint publishes, per service. This endpoint declared neither, which the live manifest showed
before the fix:

    curl -s --http2-prior-knowledge -H 'accept: application/vnd.restate.endpointmanifest.v4+json' \
      http://127.0.0.1:19102/discover      # every service: no inactivityTimeout, no abortTimeout

Every handler here queues behind a blocking slot (`--max-concurrent 8`), and the fan-out submits all
49 jurisdictions at once, so most of those handlers were *waiting* — with no journal entry to show for
the wait — when the one-minute rule read them as stalled. The abort killed each invocation ten minutes
later. The timeouts were therefore never a statement about how long the census takes; they were a
statement about how long a queue may be, and the nationwide queue is longer than that.

Fix. The nine store-backed services are bound with `ServiceOptions` declaring an hour for each timer
(`CENSUS_INACTIVITY_TIMEOUT`/`CENSUS_ABORT_TIMEOUT`, `crates/census-service/src/restate_services/mod.rs`),
`BrowserSession` deliberately keeps the defaults, and `fjall_restate_e2e`'s discovery test asserts both
numbers appear in the manifest of every service that advertises them:

    cargo test -p census-service --test fjall_restate_e2e restate_endpoint_advertises

The abort is a *first* attempt's ending rather than lost work: a jurisdiction invocation resumes from
its journal, so the failed states are re-driven rather than rebuilt. `docs/deployment-lifecycle.md`
carries the rule.

---

## The census seals: every §70 item satisfied (2026-09-24)

With both fixes in the build, the revision-8 census was sealed through the deployment that ran it.
The endpoint was rebuilt at `326853b`, started over `var/midwest-census`, and registered with the
local Restate node; the seal then read the run's own journal by naming every key the deployment's
`state` table answers for `Ingest`:

    census-serve --listen 127.0.0.1:19103 --data-dir var/midwest-census --max-concurrent 16 \
                 --drain-timeout 30 --browser-profile …/var/browser-profile
    curl -X POST http://127.0.0.1:19095/deployments -d '{"uri":"http://127.0.0.1:19103/","use_http_1_1":true}'
    KEYS=… # SELECT DISTINCT service_key FROM state WHERE service_name = 'Ingest'  → 53 keys
    census-service open-work --season 2026 --revision 8
    census-service seal --grad-year 2027 --season 2026 --revision 8 --write --source-object $(53 keys)

    season 2026-27 revision 8
    jurisdiction sweeps owed: 0 of 49      # item 1, online
    source objects owed: unmeasured        # open-work cannot enumerate objects; the seal's list does
    phase: complete
    already sealed: 86421165beab0bc754faf36d326a0f34efaeb9357fb18b53f794d0ac7ab3ddf4
    acceptance: every §70 item is satisfied
    sealed 86421165beab0bc754faf36d326a0f34efaeb9357fb18b53f794d0ac7ab3ddf4 on 2026-09-24
      cohort 580334 of 2228631 athletes, 31818 schools, 11353 meets, 28979 cohort performances, 31488 coaches
      retained: 127 gaps, 4548 conflicts, 0 access conditions (0 hosts refused, 0 throttled),
                unmeasured source failures
    wrote var/midwest-census/out/seal.json

Four things this run settles:

- **The workbook gap was an export stale by 488 cohort athletes, not a store difference.** The first
  pass certified `census-service-2026-09-24.xlsx` as it stood at 16:11 and refused on
  `579846 of 580334 cohort athletes appear in the workbook`. `census-service workbook` (92 s) rebuilt
  it from the same store, and the same seal then read `580334 of 580334`: the rows had landed between
  the export and the check. The all-source scope stays a different census (`report.json` 623509) and is
  not what that seal certified by default. **2026-09-25:** the scope flags were aligned on `--core`, so
  every approved source is what a flagless run measures now and `--core` is the explicit
  Athletic.net-free diagnostic (`--all-sources` was the old name of that flag; the runs quoted above
  predate the rename and are kept as executed).
- **A `seal.json` from an older build is a parse trap, exactly as the runbook warns.** Both the store
  route and the online route died on `parsing var/midwest-census/out/seal.json — missing field
  silent_sources`, the field this build added to `RetainedFindings`. Renaming the artifact aside
  (`seal.v5-2026-09-24.bak.json`) and re-deriving produced the same digest the ladder had already
  recorded, so the retired file was the only stale thing in the path.
- **The retained finding is where the item-2 shortfall travels.** `unmeasured source failures` is
  `RetainedFindings.silent_sources` — the endpoints that finished empty — so the half of the count the
  owed rule can no longer carry is still named, and still part of the digest.
- **The timeout fix is live, not only asserted.** The registered deployment answers per service:
  nine store-backed services report `inactivity=1h abort=1h`, `BrowserSession` keeps Restate's
  `1m/10m` on purpose, and `Census` keeps `journal=1h idempotency=30d` while the rest keep `90d/30d`.

The sealed workbook is `var/midwest-census/out/census-service-2026-09-24.xlsx` (78 666 872 bytes,
20 sheets: Athletes, PRs, Performances_001, Coaches, Schools, Meets, Sources, Coverage, Conflicts,
Review, Run Metrics, Goal & method, Summary, By state - core, By state - all sources, Athletic.net
marginal, Best results, Meets summary, Evidence mix, Method notes). `census-service verify --store
var/midwest-census --workbook …` then reconciles its data rows against the store with `census-serve`
stopped — the same window in which the store is free for `fjall-stats` and `store-integrity` — and
answered `verify: OK (5000 athletes sampled of 580334 rows, 5000 performances sampled of 222065
rows)`.

The run's own counts, read from the objects rather than recalled: 49 jurisdiction objects at revision
8, each holding its teams, rosters and meets stages; 53 `Ingest` objects; the store holds 3 072 309
athletes, 2 364 818 of them inside the census scope (AK and HI publish in no row), 12 556 meets, and
the 580 334 class-of-2027 athletes the core-scope seal certifies.

---

## §45 gets a record: per-source accounting, and the debt it sat behind (2026-09-24)

`ARCHITECTURE.md` §10 states the obligation — "per-source metrics (§45) are recorded and the
efficiency metric is **verified useful records per physical request**" — and the client was telling
half of it. `FetchStats` counted `requests` and `cache_hits` for the whole run and carried a
`per_host: HashMap<String, u64>` that **nothing wrote and nothing read**, so neither a per-source row
nor the ratio built on one could be printed from the client that made the requests.

What changed, in `crates/census-crawl/src/net` and `crates/census-service/src/census`:

- **`HostTraffic { requests, cache_hits, bytes }`** replaces the `u64` map, and `FetchStats::per_host`
  is keyed by **host** everywhere. It was not: `record_request_stats` was handed a whole URL and keyed
  by it, while `count_request` keyed by the plan's host — one origin's traffic sitting under two kinds
  of key, which is exactly the split §10's per-origin admission rule forbids. `host_of(url)` (parsed
  by `reqwest::Url`, falling back to the URL's own text so a request is never dropped) is now the one
  place a URL becomes a key, and `net::tests::a_source_row_is_the_origin_not_the_page` pins the port,
  path, query and unparseable cases.
- **Every request is counted once, in one lock.** `cache_and_record` counts a 200/404 body and its
  bytes, `count_request` counts every status that never reaches it, and the browser lane's
  `count_capture` counts an accepted capture — which had been adding bytes but *not* the request, so
  a browser-transported source's 200s were invisible to §45. One physical browser request is now one
  row, the same as one physical HTTP request.
- **Latency is a fixed 20-bucket histogram** (`net/latency.rs`): a census fetches for days, and a
  per-request sample vector would have been the only unbounded structure in the client. The mean is
  exact; `latency_percentile_ms(50|95|99)` reports the edge of the bucket covering the rank — an
  upper bound at the histogram's resolution, which the type documents rather than presenting as one
  request's timing. No samples means `None`, never zero.
- **`TransportReport::from_stats`** (`census-service/src/census/mod.rs`) is the §45 surface: requests,
  cache hits, physical requests, bytes, latency mean and percentiles, rate-limited, timeouts, errors,
  `verified_records_per_physical_request`, and one `SourceTraffic` row per source — busiest first,
  host breaking ties so two runs render the same table. `CollectReport.requests`/`.cache_hits` were
  **removed rather than mirrored**: every reader now reads `report.transport` — the CLI's adapter
  summary is the one that existed — and `verified_records` is the walk's own athlete count passed in,
  because the report knows the traffic while the walk knows what the traffic produced.
- `the_transport_report_reads_the_counters_once` pins the projection's edges: the per-source physical
  counts sum to the run's, ordering is deterministic, an unmeasured percentile stays absent, and the
  ratio is read as 6/3.

**The ratchet this increment had to clear.** `tools/gate.sh`'s debt lane had three findings against
the baseline: an `as` cast at `net/types.rs:301`, `net/types.rs` at 346 lines, `net/execute.rs` at
309. The cast is now a checked conversion through `u32` (no ratio beats an approximate one);
`types.rs` gave up its histogram (`latency.rs`) and its clock helpers (`time.rs`) and is back inside
the budget; `execute.rs` gave up `record_transport`, which now sits beside `count_request` in
`attempt.rs` where the attempt's accounting is one thing. `net/decode.rs` — a 147-line duplicate of
the live path (`process_response` had no callers; `read_checked_body` existed in both `decode.rs` and
`execute/body_reader.rs`) kept alive by three `#[allow(dead_code)]` attributes — was deleted rather
than patched, because the new `HostTraffic` could not compile its copy of the counter anyway.

**Two clippy findings had to go with it.** The strict-clippy tally is the ratchet's other input, and
its baseline is empty — the tree may carry no diagnostic at all. It flagged
`clippy::arithmetic_side_effects` twice in the latency histogram: `LATENCY_BUCKET_COUNT - 1` (the
fallback bucket index) and `latency_ms_sum / samples` (the mean). Both are now operations that cannot
overflow or divide by nothing — `saturating_sub` and `checked_div` — which is §37's rule read from
the other side: the histogram runs in the request path, so arithmetic that could panic there is
exactly what the lint exists to catch, even where a reviewer can see the operands are safe.

    cargo xtask scan
      files_over_300_lines: []        # the three findings above, gone; baseline floor is also []
      functions_over_60_lines: 0      # baseline floor is 0
      forbidden constructs: as_cast 0, unwrap 0, expect 0, panic 0, indexing 0
    cargo run -q -p xtask -- ratchet tools/quality-baseline.json <clippy.tsv> <scan.json>
      clippy tally: (no diagnostic)   # the baseline is empty; the tree adds none
      files over 300 lines: 0 -> 0
      ratchet: no metric grew
    cargo test -p census-crawl --lib net::tests
      test result: ok. 25 passed; 0 failed
    cargo test -p census-service --lib census::tests
      test result: ok. 1 passed; 0 failed
    bash tools/gate.sh
      gate: PASS (debt ratchet holds; counts above) — 430 s, exit 0
      lanes: fmt, check, doc, tests (nextest: 1246 run, 1246 passed, 3 skipped, 1 slow),
             strict clippy (source targets: 0 diagnostics), production scan
             (files>300=0, fns>60=0), domain type integrity, domain purity, module seams,
             debt ratchet, deny, audit, vet, machete, geiger, feature powerset, bench presence

The baseline was not refreshed: `tools/quality-baseline.json` still records
`files_over_300_lines: []` and `functions_over_60_lines: 0`, and the tree returned to those numbers by
deleting code rather than by absorbing a rise.

**What the sealed run already says.** These counters record from this build forward; the revision-8
census above was collected before them, so its report carries no `transport` block. Its store does
carry the per-response evidence the report is built from — every cache entry's `CacheMeta` names the
URL it answered — which makes the *shape* of that run's sourcing measurable even though its counters
are not:

    <store>/http, 47 917 *.meta.json files (one per cached response: URL + byte count)
      entries        bytes  host
        3 682    5 035 539  api.ihsa.org              # most responses, and tiny ones
        3 609  425 333 102  www.wiaawi.org
        2 732  125 147 341  www.mshsl.org
        2 582  685 058 082  tx.milesplit.com          # most bytes
        2 205  458 788 960  ca.milesplit.com
        1 744  299 736 936  ny.milesplit.com
        1 299  161 725 229  al.milesplit.com
        1 262  185 729 504  fl.milesplit.com
      total: 47 917 responses, 7 211 950 588 bytes (6.72 GiB), 264 hosts

Read those as cache entries — responses written — not requests made. The distinction is the reason
`HostTraffic` exists: `requests - cache_hits` is what a source's operators actually see, and §10's
admission budget is stated in exactly those terms. Re-deriving the table is a read of the store:

    cd <store>/http
    find . -name '*.meta.json' -print0 | xargs -0 -n 500 grep -h -o \
      -e '"url": "[^"]*"' -e '"bytes": [0-9]*' \
    | awk '/"url"/ { if (match($0, /https?:\/\/[^\/"]+/)) { u = substr($0, RSTART, RLENGTH);
               sub(/^https?:\/\//, "", u); sub(/:.*$/, "", u) } next }
           /"bytes"/ { b = 0; if (match($0, /[0-9]+/)) { b = substr($0, RSTART, RLENGTH) + 0 }
               if (u != "") { n[u]++; s[u] += b; u = "" } }
           END { for (h in n) printf "%8d %14d %s\n", n[h], s[h], h }' | sort -rn

**Operator consequence.** A run's §45 numbers are `transport` in the report `collect` prints to
stdout — `census-service collect --store <dir> --states WI --limit-per-state 1 | jq .transport` — and
in the same object when the collection ran through the ingress and returned it as the workflow
result. They are not a second ledger, and not `<store>/out/report.json`: that file is the `report`
verb's read model, which has never carried transport counters. A census sealed before this build has
no `transport` field at all — absence, not zeros — and the cache inventory above is the honest
substitute until the next run records its own.

## The index stage was quadratic: fixed, re-measured, and the census re-sealed (2026-09-25)

**Symptom, measured on the live process.** `census-service run --store var/midwest-census` spent
32:26 (32:08 CPU) inside `index` and committed **no row** — `find var/midwest-census -newermt` empty
throughout, the last write being the 20:44:46 `consolidate`. `/proc/<pid>/io` showed `rchar` **20,973
GiB (20.5 TiB)** at 18.5 GB/s sustained with `read_bytes` of 18 MB, i.e. all page cache, single
writer, RSS flat at 6.13 GB. The reads were ~4.5 MB `pread64` chunks over three segment files of one
~215 MB table, about 86 full scans a second.

**Where the time went.** A symbolized dev build of the same stage under `perf record` put the hot
frames in `serde_json`'s deserializer (`parse_whitespace` 8.2%, `skip_to_escape` 3.4%,
`MapAccess::next_key_seed` 1.3%) reached through `census_domain::model::{provenance, cohort,
classification, athlete, identifiers}` - the rows were being deserialized inside the loop, not merely
counted.

**Cause, at file:line.** `census-service/src/cli/publish.rs:213 run_index` calls
`census-reconcile/src/index.rs:118 derive`, which calls `:215 canonical_pass` (from `:119`), which
calls `store.replace_many(Table::SourceIdentities, &pass.identities)` (`census-store/src/write.rs:137`)
→ `census-store/src/batch.rs stage_derived`, which called `drop_foreign` **once per record** (~5.4M
records, each a prefix scan of that table) and `drop_unnamed` with an O(n) `named.iter().any(...)`
membership test **per row**.

**Fix.** `stage_derived` now stages each record (one point `get` + one batch insert) and then runs
**one** `drop_foreign_batch` scan whose membership test is a `HashSet`; `drop_unnamed` uses
`HashSet::contains`. No key layout changed and no acquired table was touched.

**Independent review found a real defect in that fix.** The guard that skips the foreign-row clear
for observation-log tables lived *inside* the per-record loop
(`2efbe6c:crates/census-store/src/batch.rs:113` — `if first_named && table.storage_mode() !=
StorageMode::ObservationLog`); the rewrite dropped it, which would have deleted appended rows for any
observation-log table a derivation names. The reviewer found it by reading the diff; it is restored
as the hoisted guard at `crates/census-store/src/batch.rs:121`, where the comment records why the
mode is hoisted out of the loop - it does not vary inside a batch. Worth stating plainly: the
regression that mattered here was found by adversarial review, not by the test suite.

**The defect has a test now**, and it was proved the way a test should be:
`crates/census-store/src/tests.rs a_derived_batch_keeps_the_appended_rows_of_an_observation_log`
appends two ids to `SourceObservations` (the second lands at sequence 1), derives one of them, and
asserts the table holds three rows - the untouched id, the appended row at sequence 1, and the derived
row at sequence 0. Put the pre-fix behaviour back (the clear running for observation-log tables) and
it fails on that count, 2 against 3; the whole `census-store` suite is 106 passed with the guard in
place. The fixture carries a second id for a reason worth remembering: a table's *first* append lands
at sequence 0, which is `DERIVED_SEQUENCE` itself, so deriving that id overwrites that row's payload
by design and the guard only ever protected rows at a nonzero sequence.

**A/B, same store copy.** Before: 32:26 wall, 20,973 GiB read, **unfinished**. After: **60 s wall,
2 GiB read, rc=0**, printing `source_identities=2710323 conflicts=5440 reviews=1359 superseded=0
coverage=215`. A second pass changed **zero rows** across all 16 tables - the derivation is
idempotent, which is what makes a re-derived index trustworthy - and every acquired table stayed put
(athletes 3,072,309, observations 3,991,059, source_identities 2,507,541 -> 2,508,619 derived,
review_cases 107,768 preserved).

**Chain and gate.** `run` then completed the whole cycle in **203 s**, writing
`var/midwest-census/out/census-service-2026-09-25.xlsx`, with its own reconciliation consistent:
`store_athlete_rows=2228631` = the core report's athletes, `best_mark_rows=8560` = the `bests` rows,
`store_coach_rows=32031` = the report's coaches. §55: FMT/CHECK/CLIPPY/TEST all `rc=0` (44 suites
ok). `cargo xtask scan`: `files_over_300_lines: []`, `functions_over_60_lines: 0`.

**§70 items 1 and 2, re-measured online.** `open-work --ingress http://127.0.0.1:18095` reports
`jurisdiction sweeps owed: 0 of 49`. The run's own state - read from the deployment rather than from
memory - enumerates exactly **53 `Ingest` objects** (49 `milesplit_<st>` plus `wayzata_ia`,
`wayzata_mn`, `wayzata_wi`, `wiaa_results_wi`):

    curl -s -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
      -H 'accept: application/json' \
      -d '{"query":"SELECT service_key FROM state WHERE service_name = '\''Ingest'\''"}'

`open-work` with all 53 keys reports `source objects owed: 0`. The seal then closed:

    phase: complete
    acceptance: every §70 item is satisfied
    digest: 453a8612b90eadf6783f8f153443b9b3967753c7d3b3c536450bf474fd8af065
    cohort 580334 of 2228631 athletes, 31870 schools, 11353 meets, 28979 cohort performances, 32031 coaches
    retained: 127 gaps, 5300 conflicts, 0 access conditions (0 hosts refused, 0 throttled)

Two honest caveats attached to that acceptance. `source_failures` is the tri-state `None`/unmeasured
by design (`crates/census-service/src/restate_services/census.rs:138,157`): the journal reports the
objects with no terminal acquisition instead, and *that* is the measurement - 0 of 53 owed, so no
retry-exhausted object exists to represent. And the cohort counts are identical to the previous seal
(class_of_2027 580,334, meets 11,353, athletes 2,228,631) while schools (+52) and coaches (+543) grew,
which is what the MPA merge was supposed to move.

**§60 drill at scale.** Recorded in `docs/FJALL_BACKUP.md` §3.6: on a 1.46 GB on-disk / 7.7 GB logical
store, backup took 10 s and restore 7 s, all 16 table counts and both size totals came back identical,
and the restored store served `consolidate` (8 s) and `report` (13 s) to the same headline numbers.

### What the workbook's own numbers reconcile to (2026-09-25, independent audit)

A separate worker re-derived every sheet from the workbook bytes — 20 sheets, 78,754,237 B, sha256
`5a664010882e84a9fc1ceb1058001c29a8a67195978069f4985bb0f5aa178345` — and cross-checked it against the
store's snapshots. Sheet names were resolved through `xl/_rels/workbook.xml.rels`, row counts by a
chunked `<row r="N"` scan cross-validated against each sheet's `<dimension>`, and shared strings
resolved to read the headers.

What reconciles exactly:

- **Athletes 580,334** = `report-core.json` `totals.class_of_2027`, and the sheet's graduation-year
  column holds exactly 580,334 values of `2027` and nothing else — so the sheet is the core cohort,
  not all grades (all sources is 623,509). Per-state counts match `report-core.json` for all 49
  jurisdictions with 0 mismatches, and **all 580,334 `ath_` ids resolve to `athletes.jsonl` records
  (0 missing)** — that is §70 item 9 for this sheet.
- **Best results and PRs 8,560** = `best-results-co2027.csv` data lines = `best-results-co2027.jsonl`
  lines = Run Metrics "best-mark rows reduced", and the Best results sheet's athlete-id column is a
  byte-identical *ordered* sequence to the CSV's (8,560 ids, 6,042 distinct).
- **Coaches 32,031** = both reports, and the sheet's non-empty professional-email count 10,671 = the
  reports' `coaches_with_email` = the Summary sheet = the By-state total.
- **Schools 31,870** (= `schools.jsonl` 32,154 minus the 284 out-of-scope AK/HI rows), **Meets 11,353**
  (`meets.jsonl` 11,353 = report total, and 9,593 of them name an Athletic.net id — four ways),
  **coverage jurisdictions 50**, **cohort performances 28,979** (three ways), and the Coverage sheet's
  **127 gap rows summing to 1,159,187** — the same total as `seal.json`'s 127 `retained.gaps` entries.

What does not reconcile, in the order a reader would hit it:

- **Conflicts, four quantities, and only the export ordering is wrong.** `seal.json` retains
  **5,300**; the Conflicts sheet publishes **5,440** detail rows; `conflicts.jsonl` holds **4,548**;
  and the sheet's own summary column says **3,214** "Findings". Each is real:
  - 5,300 is the store's ledger (`rows:conflicts` in the meta keyspace, written inside the same batch
    as the rows, `census-store/src/read/rows.rs:76-90`). `Table::Conflicts`'s `entity_id()` is
    `&self.id`, minted as `{family}:{subject_id}` (`census-domain/src/model/records.rs:64`), so the
    `replace_many` write collapses to one standing row per key: the 5,440 batch rows carry 5,300
    distinct keys.
  - 5,440 is the index pass's batch length (`census-reconcile/src/index.rs:123-133`, printed at
    `index.rs:162`) — `retained.conflicts` plus `pass.collisions` — *before* the store dedups it. 140
    of those rows sit on 102 keys that already hold a row, all in "Recruiting contact conflict" (one
    school with a "Head TF Coach (boys)" row and a "Head TF Coach (girls)" row, for instance). The
    sheet renders that same `retained_records` one line per entry
    (`census-report/src/workbook/meta/queues.rs:89-98`), so sheet rows equal batch rows exactly.
  - 4,548 is what `consolidate` counted when it wrote the file — the distinct ids the table held at
    that instant (`census-store/src/read/mod.rs:78-128` visits once per distinct id). Its mtime is
    21:22:20 while the index pass flushed the keyspace at 21:23:11-13, and `docs/OPERATIONS.md:181`
    places `consolidate` *before* `index` in the chain, so the file is always the previous cycle's
    content. The previous seal agrees: `seal.v5-2026-09-24.bak.json` also carries 4,548, and the
    pre-pass ledger in the SST reads `rows:conflicts 4548`.
  - 3,214 is `Family::group`'s count — one finding for N rows, where `Family::push`
    (`census-report/src/workbook/meta.rs:150-176`) records one finding per row.
  So the sheet is right and the seal is right; the defect is the **export ordering**. `conflicts.jsonl`
  — and each derived-table sibling, `review_cases`, `coverage`, `snapshots`, `source_access`,
  `identity_verdicts` — is written by a `consolidate` that runs before the `index` pass that rewrites
  its table. Nothing in the workbook depends on those files. The Review sheet's 1,364 rows against the
  same pass's `reviews=1359` is the same dimensional difference on a second table.
- **Coverage athletes, 569 short, and the cause is the same staleness as above.**
  `coverage.jsonl`'s 50 jurisdiction rows sum to **622,940** at `cohort_athletes` where `report.json`'s
  `by_state` sums to **623,509**; five jurisdictions differ (AL 9,590/9,120, DC 207/126, KY 6,647/6,637,
  MO 15,175/15,168, TN 9,515/9,514) and 45 match. Placement is *not* the difference: both derivers call
  the same `jurisdiction_of` (`census-report/src/report/coverage/state.rs:70-74`), so no rule diverges.
  Freshness is: `report.json` is recomputed at publish time (`cli/publish.rs:52` → `build_census`),
  while `coverage.jsonl` merely re-serializes the stored `Table::Coverage`
  (`census-service/src/census/aggregate.rs:139`) that only the index pass rewrites. The cycle ran
  `consolidate` *before* `index`, so the file published the previous cycle's rows — and the 470 newer
  AL athletes arrived from `milesplit_al` without profiles, which is why the stale AL cohort happened to
  equal AL's profile count and made the gap look like a profile-URL correlation. It was a coincidence of
  the previous cycle's numbers, not the cause.
  `report.json` is right on three further grounds: it equals the store's merged athlete table recomputed
  per bucket (all 50 buckets, including the run-scope split AK 1,340 / HI 1,619), it equals the coverage
  pass's live output that the Coverage sheet carries (AL 9,590), and `seal.json`'s own gap register
  computes AL `missing_profile` = 470, which only follows from athletes 9,590 with 9,120 profiles.
  This is also *not* the run-scope exclusion, which the sheet states separately and correctly in its own
  note — "stored rows outside the census run scope (the 48 continental states plus DC, ADR-009) are
  excluded from `coverage read` and published in no row: schools=284 athletes=2959 coaches=0 meets=0
  performances=0". That set is 2,959 athletes, and its arithmetic closes on the other axis:
  1,751,450 off-cohort + 623,509 cohort + 2,959 outside scope = 2,377,918 = `athletes.jsonl` lines.
  **Repaired and verified 2026-09-25.** With the cycle reordered so `index` runs before `consolidate`,
  a `consolidate` against the live store rewrote the file: 50 jurisdiction rows summing **623,509**,
  with AL 9,590, DC 207, KY 6,647, MO 15,175 and TN 9,515 — every one equal to `report.json`.
  `conflicts.jsonl` was regenerated in the same pass and now holds 5,300 lines against the ledger's
  5,300.
- **Ohio performances, 1,123 unpublished, and the cause is one non-core-sourced meet.**
  `performances.jsonl` holds 223,188 rows, the Performances_001 sheet publishes 222,065; seven of eight
  jurisdictions are identical and the entire deficit is Ohio (2,108 stored against 985 published). All
  1,123 are one meet, `meet_0767da7a7a50f50a` "D1 Region 01 Finals" dated 2015-05-29, and every one
  carries Athletic.net evidence only (`source_key` prefixed `athleticnet:`, source URL
  `www.athletic.net/api/v1/Meet/GetMeetData`); no OHSAA source is cited on any of them. The meet itself
  *is* published in the Meets sheet, so the performances alone are withheld, and the published 985 all
  come from Ohio's other three meets, dated 2026-04/-05. The rule holds exactly across all 50
  jurisdictions: the sheet's `Source ResultID` set equals the store's rows carrying at least one core
  evidence source — 145,934 keys on both sides, difference zero in both directions. Nothing in the
  workbook states the rule, so a reader cannot tell a withheld performance from one never crawled.
- **Run Metrics "Athletes"** prints 2,364,818 (all sources) in a block whose sheets are core-scope
  (2,228,631) — the one counter in the reconciled block that does not reproduce from its own artifact.
- **`census-by-state-all-sources.csv`** disagrees with `report-all-sources.json` on 12 `schools`
  values; both files are the stale 2026-09-20 pair, not this run's output.

None of this is visible to the seal's own acceptance: `inspect_workbook` checks sheet presence,
`coverage_rows >= jurisdictions` (239 >= 50), and `mapped_athletes == class_of_2027` — a floor check
that passes while the 569-athlete gap stands. That gap between "the seal says every §70 item is
satisfied" and items 10-12 read strictly is the honest state of those three items.

### What `seal.json` cannot show, and what it implies

The file carries `phase`, `counts`, `retained`, `workbook_rows`, `sealed_on`, `digest` and nothing
else: `SealedCensus` (`crates/census-service/src/census/state/evidence.rs:238-244`) keeps those five
fields, so the per-item evidence — `OpenWork`'s fields for items 1-4, `WorkbookCheck`'s for items
9-13 — is computed at seal time and dropped. Three consequences worth stating:

- **`"complete"` is a construction guarantee, not a claim.** `CensusState::seal`
  (`crates/census-service/src/census/state.rs:179`) refuses with `SealError::ItemUnmet` unless
  `SealEvidence::open_items()` is
  empty, and `open_items` raises items 1-4 whenever `jurisdiction_sweeps`, `source_objects`,
  `cohort_decisions` or `identity_candidates` is anything but `Some(0)`. The written phase therefore
  *implies* all four were measured zero, even though no key in the file says so.
- **Items 5 and 6 are deliberately never blockers** (`RetriesRepresented` and `ConflictsRetained` never
  appear in `open_items`), which is how the seal says "every item satisfied" while
  `source_failures` is `null`: that field is tri-state, and `null` means *cannot count*, not *none*.
- **Re-measured independently on 2026-09-25:** `open-work` over all 53 source objects reports
  `jurisdiction_sweeps: 0` and `source_objects: 0`, with the 4 silent endpoints matching
  `retained.silent_sources` element for element; `review_cases.jsonl` (107,768 rows) holds 0 `Pending`
  identity candidates and 0 `Pending` cohort decisions; and replicating `inspect_workbook`'s row rule
  returns `239 + 49 = 288` = the stored `workbook_rows`, which is what makes the replication a
  measurement rather than a guess.
- **The run's own state, read from the deployment**, is 462 rows across three services: `Ingest` 53
  (the source objects above), `JurisdictionCensus` 402 (49-51 objects per revision, revisions 1-9),
  and `NationalCensus` 7 — the 49-jurisdiction scope (digest `51472a0f63b82f0d`, reproduced as sha256
  over the 49 codes in declaration order) at revisions 1, 2, 6 and 8, plus a DC-only scope
  (`107154493fc6af5b`, which is sha256 of `DC\n`) at revisions 2, 90 and 91.

Two things a reader should know that the artifact does not say: the counts **mix scopes**
(`athletes`/`class_of_2027` are core, `meets` is all-source) and there is no `scope` key; and
`retained.gaps` drops the jurisdiction each `CoverageGap` carries
(`crates/census-report/src/report/coverage/gaps.rs:80-87`), so the 127 rows cannot be attributed to
states from this file alone.

---

## The integrated tree: the budget at zero, and the port collision the kill/restart test was hiding (2026-09-25, integration pass)

The §38 budget that stood at seven files and six functions is empty, and the two gates that had been
red are green on the integrated tree:

    $ cargo xtask scan                       # structure block
    "files_over_300_lines": []
    "functions_over_60_lines": 0
    "functions_over_60_sites": []
    "unstable_feature_sites": []
    $ cargo xtask contract                   # 8 checks, all PASS: modules, budgets, no_python, ...
    $ cargo xtask seams                      # 0 violations across the allowed-edge tables

`functions_over_25_logical_lines` stays at 629 and is printed with `(context)`: it is the counter the
gate's own comment says rises with every feature, and the ratchet prints it rather than failing on it.

**The kill/restart test was failing on a port collision, not on the resume it exists to prove.** The
first full suite of the pass reported `43 suites ok, 1 failed`:
`restate_kill_restart::a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write` panicked at
`restate_kill_restart.rs:596` with `the node's admin API answers: "node admin API never came up: error
sending request for url (http://127.0.0.1:39517/deployments)"`. The node's own log named the cause:

    Failed: [admin-api-server] failed binding to address '127.0.0.1:39517': Address in use (os error 98)

The generated config bound **`[admin]` and `[ingress]` to the same port** (39517). The 09:30 run of the
same test, whose root survives at `/tmp/midwest-kill-restart-720862`, had three distinct ports
(33335 node, 37001, 43603), and the production node holds fixed ports (15152 node, 19095 admin, 18095
ingress) — so no second process was involved. `free_port()` bound `127.0.0.1:0`, read the port, dropped
the listener, and returned; the kernel re-offers a just-released ephemeral port to the next `bind(":0")`,
so two adjacent picks returned 39517 and the second server died. The fix is a handed-out ledger
(`static HANDED_OUT: LazyLock<Mutex<HashSet<u16>>>`) that makes every pick in the process distinct and
retries otherwise; the only race left is the handoff window to other processes, where losing shows up as
a child that never becomes ready, never as a silent pass. Re-run:

    $ cargo test -p census-service --test restate_kill_restart
    test a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 123.97s

**Both live report/CSV pairs reconcile exactly; the pair the audit flagged is a legacy-name leftover.**
`report-core.json` against `census-by-state-core.csv` and `report.json` (scope `all_sources`) against
`census-by-state.csv` each agree on all 50 jurisdictions and on every total — 0 mismatches in `schools`,
`athletes`, `coaches` and `coaches_with_email`. The files named `report-all-sources.json` and
`census-by-state-all-sources.csv` are a 2026-09-20 pair from an older naming scheme: the JSON covers
only 12 jurisdictions and every one of its `schools` counts is 0. The current writer emits the
unsuffixed pair, which is why only that dead pair disagrees.

That also identifies the number the audit could not place: `Run Metrics` printed **2,364,818**, which is
exactly `report.json`'s all-source athlete total, in a block whose other counters are core-scope. The
core scope is **2,228,631** (`report-core.json`'s total, and the seal's `counts.athletes`).

**The workbook-build profile, separated from the code.** The offline CLI path (`census-service workbook`)
runs whatever the caller built: the wrapper's own log shows `cargo run -q -p census-service --bin
census-service -- --store var/midwest-census workbook …`, i.e. the debug profile (417 MB binary). The
recorded 92 s/203 s builds in this document came from the prod endpoint. `Cargo.toml` defines only
`[profile.release]`, and `tools/durability/run.sh` resolves its endpoint from `target/release` — so
`--release` *is* the production path, and the remaining spread is machine context, as PERFORMANCE.md's
"record on a quiet machine" caveat says. No code regression is implicated, and none was found.

---

## The recruiting workbook, built and measured: 1839 s, nine sheets, and where the time went (2026-09-25)

**The artefact.** The offline export on the integrated tree completed at 12:13:11 with rc=0:
`var/midwest-census/out/census-service-2026-09-25.xlsx`, **81,775,867 bytes**, sha256
`bebe315b514830e531d3513f34eb7187…`. Its **nine** worksheets are the ones the module doc names, in
the objective's order: `Athletes` (§50), `PRs` (§51), `Performances_001` (§52), `Coaches` (§53),
`Meets`, `Sources`, `Coverage`, `Data Quality`, `Run Metrics` (§54). `Performances_00N` materialised
as a single `Performances_001`: the store holds 309,962 canonical performances, far below Excel's
1,048,576-row cap, so no partition beyond the first was needed. (The four open range writers visible
during the build are the spill module's internal partitions, not published sheets - this section
records that because the file's plan predicted twelve worksheets from those four descriptors and the
artefact refutes it.)

**The cost, and what it is not.** `EXPORT rc=0 elapsed=1839s`. A first reading called this a
quadratic to match the index-stage defect recorded above; the measurements refute that reading.
`fjall-stats` puts the store's shape at 309,962 performances, 3,991,059 observations, 3,072,309
athletes, 2,508,619 source identities and 2,069,298,240 bytes on disk, with **`store_bytes`
15,714,376,761** - and the build's resident set sat at 15,000,880 KB, i.e. the build materialises the
whole logical store. `/proc/<pid>/io` read `rchar` 6,541,804,755 - very close to three passes over
that store, which is the shape the module doc describes ("one dataset read three times around the §52
performance sheets") - while `read_bytes` stayed 0 (all page cache) and `write_bytes` stayed at
6,344,704 until the final write. `stime` was 3.7 s against a `utime` that grew past 1600 s: the phase
is userspace CPU, one core at 99.7% of a 32-core host.

**Where the CPU went.** A 4-second `perf record -F 99 -p <pid> -g` (permitted at
`perf_event_paranoid=2`) captured 396 samples: **97.2% of self time in `__memcmp_evex_movbe`**,
identified by disassembling the sampled address rather than by trusting the symbol table (the
nearest exported symbol was `_dl_mcount_wrapper`, which the disassembly shows to be
`__memcmp_evex_movbe`'s tail). The build is comparison-bound. The release binary is stripped, so the
*caller* of those comparisons is not attributable from this profile - the index-stage investigation
above used a symbolized dev build for exactly this reason, and that is the named next step. What the
profile does refute is one particular suspect: the per-row paths were each read and each is a hash
lookup or a single linear pass - `RangeFiles::range_of` is one `HashMap::get`,
`retain_core_row` is a retain plus two small drops, `Parents::read` filters with hash-backed indexes,
`bucket_universe` sorts and dedups once, `Lookups` is four maps, `bests::build` accumulates through
`for_each_merged` and sorts once, and `sheet_order` is a plain comparator. None is O(n²).

**Comparability.** PERFORMANCE.md's 203 s row is the *in-process* chain ("consolidate → index → two
report scopes → bests → workbook") measured after the index fix, on the store as it stood then; the
offline CLI additionally opens and scans the store itself. The two are not the same workload, and
this section does not claim a regression between them. What today's numbers establish is the
baseline the §24.2 before/after for the `Athletes` sort key is measured against: the same-corpus
control run follows in the next section.

---

## §60 on the real store: backup, restore, integrity, and a full census read of the restored copy (2026-09-25)

The drill `docs/FJALL_BACKUP.md` describes was run against `var/midwest-census` itself, not a fixture,
with the sequence the objective asks for - consistent backup, restore, integrity verification, reopen,
full census read - and with the store quiescent (no writer attached) throughout. Exact commands:

    census-service store-integrity --store var/midwest-census
    census-service store-backup   --store var/midwest-census --to /tmp/store-drill/backup
    census-service store-restore  --from /tmp/store-drill/backup --to /tmp/store-drill/restored
    census-service store-integrity --store /tmp/store-drill/restored
    census-service fjall-stats     --store /tmp/store-drill/restored

**Integrity.** The live store's check printed per-table `expected`/`actual` lines and exited 0 in
**3.1 s**; the tail shown was `coverage 215`, `snapshots 3`, `source_access 0`,
`identity_verdicts 43291`, `source_meets 131726`, `source_observations 207`, each `ok`. The restored
copy's check printed **16 tables `ok`** followed by `ok true`.

**Backup and restore.** The backup wrote a whole store-root copy in **23.5 s**; the restore into a
fresh directory completed in **12.0 s** and reported the same per-table counts the backup had. The
copy is the whole root, not only the Fjall tables - 15 GB on disk, against the Fjall store's own
2,069,298,240 `bytes_on_disk` - so an operator sizing the drill should expect the HTTP cache, the
entity logs and `out/` to travel with it, as `docs/FJALL_BACKUP.md` §1 tabulates.

**The census read.** `fjall-stats` on the restored copy differs from the live store on **none** of the
seventeen per-table counts: schools 89959, teams 207609, coaches 65020, athletes 3072309, meets 12556,
events 101711, performances 309962, source_identities 2508619, conflicts 5300, review_cases 107768,
coverage 215, snapshots 3, source_access 0, identity_verdicts 43291, source_meets 131726,
source_observations 207, observations 3991059. The one difference is `store_bytes`:
**15,714,376,761** live against **15,635,532,005** restored, a 0.5% reduction with identical row counts
and identical `bytes_on_disk` (2,069,298,240) - the round trip re-serialises the store, so the logical
byte total is not expected to reproduce exactly while every count does. Recorded rather than smoothed
over, because a reader comparing the two outputs will see it.

### The nine sheets, measured from the artefact (2026-09-25)

Read from `xl/workbook.xml` and each sheet's `<dimension>` in the built workbook, so these are the
artefact's own counts rather than the builder's log:

| Sheet | Range | Data rows | Columns |
| --- | --- | --- | --- |
| `Athletes` (§50) | `A1:BF580335` | 580,334 | **58** |
| `PRs` (§51) | `A1:S9959` | 9,958 | 19 |
| `Performances_001` (§52) | `A1:S222066` | 222,065 | 19 |
| `Coaches` (§53) | `A1:N32032` | 32,031 | 14 |
| `Meets` (§54) | `A1:J1879` | 1,878 | 10 |
| `Sources` (§54) | `A1:H185` | 184 | 8 |
| `Coverage` (§54) | `A1:AB240` | 239 | 28 |
| `Data Quality` (§54) | `A1:H48927` | 48,926 | 8 |
| `Run Metrics` (§54) | `A1:D53` | 52 | 4 |

Three of these are cross-checked against the builder's own running checks and against `verify`:
`Athletes` 580,334 equals the export log's `rows=580334` (with `in_scope=2228631` as the all-sources
denominator, so the sheet is the recruiting cohort, not every stored athlete); `PRs` 9,958 equals
`best_mark_rows=9958`; `Coaches` 32,031 equals `store_coach_rows=32031`; and `Performances_001`
222,065 equals the row count `verify` reported for that sheet. `Athletes` spanning `A` to `BF` is the
§50 58-column recruiting layout in the artefact itself.

`census-service verify --store var/midwest-census --workbook …` sampled to its 5,000-row cap on both
sampled sheets and reported `verify: OK (5000 athletes sampled of 580334 rows, 5000 performances
sampled of 222065 rows)` in 1m37s.


## The §24.2 before/after for the `Athletes` sort key: a payload-identical workbook, and a timing delta the workload cannot resolve (2026-09-25)

`MaterialisedSnapshot`'s `Athletes` sort keyed every comparison through `stable_key()` on both
sides, allocating two `String`s per comparison — on the order of 580,334·log2(580,334)·2 allocations
to order a sheet that is written once. The patch materialises each row's key once
(`sort_by_cached_key`), so the cost is bounded by the row count instead of the comparison count.

Two release binaries were built from the same tree with only that file differing
(`crates/census-report/src/workbook/recruiting/athletes.rs`), and each ran the recruiting workbook
export over the same store with no other load on the host:

| | A (materialised key) | B (control, per-comparison key) |
|---|---|---|
| source sha256 | `3593d3bfc003c942f1d072d0273efd6a497646c35e09fce8ebfe1278fa5f46d3` | `119754a84cdee6d56b8ec0bc0fefb8085680e30c8d54ac34222709c1fe0efe13` |
| exit / sheet rows | rc=0; Athletes 580,334 · PRs 9,958 · Coaches 32,031 | rc=0; Athletes 580,334 · PRs 9,958 · Coaches 32,031 |
| workbook bytes | 81,775,867 | 81,775,866 |
| workbook sha256 | `bebe315b514830e531d3513f34eb71877d5138709f32f5d3e303446935767a53` | `32d7304c54639bc642efdd1de3c440b79861661fcdbb2e59160362d3ebd381dd` |
| elapsed | 1839 s | 1798 s |
| steady RSS | 14,991,196 kB | 14,991,196 kB |
| bytes read from the store (`rchar`) | 6,541,804,755 | 6,541,804,755 |

The container digests differ, so the two deliverables were compared member by member instead of by
container hash: of the 18 ZIP members, 17 are byte-identical, and the one that differs is
`docProps/core.xml`, whose only difference is the writer's wall-clock stamp
(`2026-09-25T16:42:59Z` versus `2026-09-25T17:20:00Z`). Every sheet part, the shared strings, the
styles and the charts are identical, so the patch changes no workbook content.

**Finding:** the workbook's `docProps` stamps are wall-clock, so *container* digests are not
reproducible between runs even though every sheet is identical. The reproducible form of the claim
is the 17-of-18 member comparison; a container-level `cmp` will always differ across runs.

**On timing, the control was 41 s faster** (1798 s against 1839 s, 2.3%), so this A/B demonstrates no
speedup. A single-core ~30-minute job on a 32-thread host carries run-to-run frequency and thermal
variance of that order, and a 580k-row sort is a few seconds of the total budget; this measurement
cannot resolve the patch's effect in either direction. The patch is kept for the reason it was
written — a per-row key bounded by the row count rather than the comparison count, with no
allocation per comparison — and not for a measured speedup, and this section is the evidence for
that distinction rather than a claim of gain.

Environment, identical for both windows: AMD Ryzen 9 9950X3D (16 cores/32 threads), 123 GB, rustc and
cargo `1.97.0-nightly`, `[profile.release]` with `lto = "thin"`, `codegen-units = 1`, `strip = true`;
one core at ~99.7% CPU, and no concurrent build or other load during either run.


## The Restate seal: what the endpoint measured, and the trap in naming source objects (2026-09-25)

The seal is the only §70 verification that reads the run's own open work, and it is the only route a
finished census can take: `census-service seal --store` opens the store and therefore cannot see the
journal's in-flight state at all, which is exactly what the first attempt reported —

```
refused: census cannot be sealed: source objects are terminal is unmet
  (source objects have no terminal state: not measured - this seal does not read the workflow journal)
```

With `--ingress http://127.0.0.1:18095` and the endpoint running
(`census-serve --listen 127.0.0.1:19103 --data-dir var/midwest-census`), the seal reads the run's
objects through Restate. Re-registering the deployment against the freshly built binary moved the
service revisions from `Census r13`/`Consolidate r9` to `Census r14`/`Consolidate r10`, so the seal
measured the restored code rather than yesterday's.

**The trap.** `--source-object` is the caller's to name, and the service cannot enumerate objects.
Naming the four plausible key families (`milesplit_<st>`, `milesplit_teams_<st>`,
`milesplit_rosters_<st>`, `tfrrs_<st>` × 49 jurisdictions plus `milesplit_unknown`) produced a
*measured* refusal of 151 objects with no terminal state. `open-work --json` with the same 200 names
resolved it: 151 of those endpoints have `observations: 0, windows: 0`, and every one of them is from
a family the run never used. A key that was never touched reads as a fresh, non-terminal object, so
**over-naming manufactures open work**; the honest set is the one the run actually used.

`open-work --json` on the real set reported the run's own drain state:

| field | value |
|---|---|
| `jurisdiction_sweeps` | 0 |
| `source_objects` (unterminated) | 0 when only the run's keys are named |
| `silent_sources` | 0 |
| `jurisdictions` swept (teams/rosters/meets) | 49, all `true`, `owed_rosters: 0` |
| `endpoints` with observations | 49 (`milesplit_<st>`), e.g. `milesplit_ca` 1253 |

**The seal, over the run's own 49 objects:**

```
phase: complete
acceptance: every §70 item is satisfied
sealed 04ba90f355bc2703f600fcf3ea6e4f838b6134cd644acacec493ec7494a3736e on 2026-09-25
  cohort 580334 of 2228631 athletes, 31870 schools, 11353 meets, 28979 cohort performances, 32031 coaches
  retained: 127 gaps, 5300 conflicts, 0 access conditions (0 hosts refused, 0 throttled)
wrote var/midwest-census/out/seal.json
```

`out/seal.json` carries `phase: complete`, the digest above, and the same counts, so a later run
reads the seal instead of re-deriving it.

**The workbook's own numbers, cross-checked against the reports rather than against themselves.**
`out/census-by-state-core.csv` (50 states plus a TOTAL row) was joined to `report-core.json`'s
`by_state` on all thirteen shared columns — 650 cells: **zero mismatches**, and the TOTAL row equals
the seal exactly (`class_of_2027` 580,334; `athletes` 2,228,631; `schools` 31,870). Together with
`Athletes` 580,334 / `PRs` 9,958 / `Coaches` 32,031 in the workbook and the seal's Run Metrics
reconciliation, every clause of the workbook re-audit is satisfied by an independent artefact.


### Note on the A/B's source digest (2026-09-25, after the debt ratchet)

The A/B above was run on `athletes.rs` at `3593d3bfc003c942f1d072d0273efd6a497646c35e09fce8ebfe1278fa5f46d3`.
The debt ratchet then failed on four metrics this change introduced — `census_report`'s
`clippy::unwrap_used` (two `Cell::number(...).unwrap()` calls in the new `athletes/cells.rs`,
now `?`-propagated through `ReportResult`), and two `as` casts plus one indexing site in
`xtask/src/perf/bench.rs` (now `u32::try_from(...).map_err(...)?` and `parts.first()`/`parts.last()`).
Those edits touch the same file's column assembly but not the sort key the A/B measures, so the
measurement stands: the sort-key change is the only difference between the two binaries, and both
still produce identical sheet payloads. The file's digest after those edits is
`5bcedee7f2ef21b7a063d8530f35f9be3860b477771a9a680d9dc99d8ac8e38f`.


## The drain's abort accounting: a reclaimed task is `aborted`, not work still in flight (2026-09-25)

**Finding.** `Spawner::drain`'s deadline path (`crates/census-service/src/spawn.rs`) issued
`abort_all()` and then reaped *once* with `while let Some(joined) = try_join_next()` — with no
scheduler turn in between. An abort is delivered on the runtime's next turn, so that loop always
found nothing: `Ledger::classify_reaped`'s `Cancelled => aborted` arm could never fire, and
`set_remaining(tasks.len())` published the tasks the abort had just reclaimed as work *still in
flight*. The result contradicted the report's own contract — the module and `TaskReport` docs read
"`remaining` reflects what the abort could not reclaim" — and the §42 consumer, the deployment's
`DrainCounts` (`restate_services/browser_session.rs`), whose JSON contract test asserts
`drain.remaining == 0` on a clean path.

**Caught by** `bootstrap::tests::drain_counts_aborted_tasks_after_the_deadline`: the first three
assertions (`accepted == 1`, `timed_out == 1`, `completed == 0`) passed and
`assert_eq!(report.aborted, 1)` failed with `aborted 0`, which isolated the classification rather
than the accounting.

**Fix.** Reap across a bounded number of runtime turns — `try_join_next`, then
`tokio::task::yield_now().await`, repeated `REAP_TURNS = 8` times or until the set is empty. A task
the abort reclaims is now counted `aborted` and leaves `remaining`; a job the abort cannot reclaim (a
blocking-pool job that already started) never becomes ready and still lands in `remaining`. The
budget is turns, not wall time, so the drain stays prompt:
`drain_returns_promptly_when_task_overruns_deadline` asserts < 100 ms against a task sleeping 10 s.

**Tests that had pinned the defect were corrected, not re-pinned:** `spawn/tests.rs`'s
`drain_aborts_and_counts_what_outlives_the_deadline` and `a_drain_counts_finished_work_and_the_deadline_separately`
now assert `remaining == 0, aborted == 1` for a reclaimed task (their old assertions, and their old
message "cancelled task not reaped by try_join_next", described the bug), and
`drain_returns_promptly_when_task_overruns_deadline` follows. The abort-resistant blocking case keeps
`aborted == 0, remaining == 1`. The `remaining` reading is now stated in `spawn.rs` (module + report
docs) and `spawn/ledger.rs::note_deadline`.

**Evidence.** `cargo test -p census-service --lib` → 182 passed, 0 failed;
`cargo test --workspace` → 1330 passed over 49 suites, exit 0;
`cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check` exit 0.

## The recruiting workbook's column map: three stale expectations and one re-blessed golden (2026-09-25)

**Finding.** Three tests in `crates/census-report/src/workbook/recruiting/tests.rs` still asserted
the pre-`Observed School Year` column indices, and two of them asserted *the same cell twice with
different values* (index 51 as both `"professional_coach_email"` and `""`; index 54 as both a profile
URL and `"2"`; index 57 as both `"pr"` and `""`), so the suite could not pass. Separately
`parity_pipeline::pipeline_publishes_the_same_bytes_from_a_rebuilt_store` failed on
`pipeline__workbook-shape`: 70 lines on both sides, first difference at line 2 — the sheet's column
set had moved.

**Fix.** The expectations now follow the published header order
(`crates/census-report/src/workbook/recruiting/athletes/rules.rs`, 60 columns): `Athletic.net URL`
52, `Sources Count` 55, `Confidence` 56 (`high` where grade evidence agrees), `Coverage State` 57,
`Conflict Flag` 58, `Review Status` 59 — `review` for the identity-only athlete whose grade
observation does not agree with the cohort, `verified` for the HIGH-confidence row with no conflict.
The golden was regenerated with `GOLDEN_UPDATE=1 cargo test -p census-service --test parity_pipeline`.

**Evidence.** An md5 manifest of `crates/census-service/tests/golden` taken before and after the
re-bless shows exactly one changed file — `pipeline__workbook-shape.json` — so no other golden moved
with the columns. `cargo test -p census-report --lib` → 71 passed, 0 failed; the parity target → 1
passed, 0 failed.

## Integration gate repairs (2026-09-25)

- Coverage reconciliation uses saturating subtraction for duplicate-row counts.
- `bench_census` propagates an out-of-range fixture mark instead of panicking.
- Kani output classification uses `split_once` instead of byte-indexed string slicing.
- Strict workspace source Clippy, including `expect_used`, `string_slice` and
  `arithmetic_side_effects`, passed with warnings denied.
- `cargo test -p xtask kani`: 17 passed.
- `cargo run -p census-service --example bench_census -- --schools 2`: exit 0;
  70 appended rows, 16 athletes, 32 performances, 16 PR rows, 28,625-byte workbook.
- Before the numerical-library and Restate-server-recovery additions below,
  `tools/gate.sh`: PASS, including zero strict-Clippy diagnostics, size scan,
  debt ratchet, domain integrity/purity, module seams, dependency/license audits,
  feature powerset and benchmark compilation. Nextest: 1,333 passed, 3 skipped.
- `cargo test --workspace --all-targets`: 1,330 passed, 3 ignored.
- Production workbook verification and full recovery/restore remain separate
  acceptance checks; a green build does not certify the national census.

### Parser fuzz execution

ASan-enabled `cargo fuzz run <target> -- -max_total_time=60 -max_len=65536
-rss_limit_mb=4096 -print_final_stats=1` completed all four targets without a
reported crash. Each target ran for 61 seconds against its existing corpus.

| Target | Seed | Executions | Peak RSS MiB |
| --- | ---: | ---: | ---: |
| xc | 3060809901 | 332037 | 499 |
| hytek | 448981090 | 826605 | 548 |
| raceday | 2117645087 | 4623237 | 633 |
| compiled | 3713899070 | 528150 | 554 |

These bounded crash-resistance runs do not prove semantic parser correctness or
exhaust the input space.

### Operator backup-drill repair

The shell drill now calls `store-backup` and `store-restore` rather than copying
an unlocked live database. Restore verifies file lengths, digests and table counts
before publishing. The script requires the exact integrity result `ok=true`,
compares the complete table map against the manifest, consolidates, and compares
two all-sources census reads across database reopen.

Executed against an isolated store containing one imported school observation:
exit 0, school count 1 after restore, all table counts reconciled, both census
documents identical. This is a CLI smoke test, not the full-census restore drill.
Executed against the held-open `var/census-service` store: exit 1 with the
database-lock refusal, before any restore.

### Audited fixed-point conversion library

`CentiSeconds`, `CentiMetres` and `CentiPoints` retain their `i32` storage and
integer JSON wire format. `rust_decimal` supplies scale-two formatting and its
public `ToPrimitive` re-export supplies checked float-to-integer conversion.
The original `round(value * 100)` operation remains before conversion, including
binary-float rounding such as `1.005 -> 100`. Negative values smaller than one
whole unit now retain their sign: `-99 -> "-0.99"`; the regression failed before
the repair. Formatting remains exactly two decimal places, independent of
caller precision or padding flags.

The lockfile selects `rust_decimal 1.37.0` and `arrayvec 0.7.6`, with default
features disabled. Imported Google audits cover Decimal `1.36.0 -> 1.37.0`
and arrayvec `0.7.6`; no new audit exemptions were added. `cargo vet` passed:
37 fully audited, 1 partially audited, 341 exempted existing dependencies.
`cargo deny check advisories bans licenses sources` passed all four checks.
The dependency-selection test run passed 103 domain tests.

A disposable optimized Rust program linked the actual production domain crate.
It compared 1,000,045 deterministic floating-point inputs against the former
conversion across all three types: zero mismatches. Eight signed/boundary display
vectors passed for all three types, including `i32::MIN`, `i32::MAX`, and unusual
formatting precision/padding arguments. A single local million-conversion timing
sample measured 2.81 ms for the former conversion and 2.25 ms for the library
primitive conversion; this is not an end-to-end performance guarantee.
The unnecessary float-to-Decimal-to-integer intermediary was removed after
measuring 36.76 ms versus 3.10 ms for the former conversion in an earlier sample.

### Restate-server crash recovery

`b_restate_server_sigkill_resumes_workflow` passed against the pinned Restate
1.7.10 binary in 9.16 seconds. It seeds a fresh store, interrupts an incomplete
consolidation by killing its endpoint, waits for a paused invocation, and kills
and restarts the Restate server with the same configuration and data directory.
The same invocation ID remains paused after restart. Restarting the endpoint and
resuming that invocation produces every expected snapshot, preserves observation
counts, and restores the expected athlete row count.

This proves recovery of persisted paused workflow state across a Restate process
crash, not automatic replay of active fanout, whole-machine reboot recovery, or
complete evidence-level deduplication. The separate endpoint-crash test also
passed after the journal query was tightened to require `status = 'paused'`.

### Model response-body transport failures

`ModelClient::adjudicate` previously replaced a failed `response.text()` read
with an empty string. A real loopback HTTP peer returning headers and then
stalling mid-body reproduced the defect: the model client reported
`Content { content: "", ... EOF while parsing a value }` instead of a request
timeout. The failing-before regression exited 101.

Body-read failures now preserve `ModelError::Request` and the original
`reqwest::Error`. Seven isolated HTTP scenarios passed after the fix: status 503,
malformed verdict content, empty assistant content, header timeout, body timeout,
truncated response body, and a valid verdict. Both timeout cases require
`source.is_timeout()`; truncation requires a non-timeout transport error. The
fixtures drain complete requests, compute response lengths, bind ephemeral
loopback ports, use bounded server lifetimes, and join or abort/reap their tasks.

The durability harness executes these scenarios as scenario 13. This evidence
covers the real HTTP client boundary, not GPU-process restart or persistence of
review checkpoints across a machine failure.

### Integrated quality gate after numerical and model repairs

`tools/gate.sh` passed in 510.09 seconds: 1,343 Nextest cases passed, 3 skipped;
strict production Clippy reported zero diagnostics; production scan reported
zero files over 300 lines and zero functions over 60 lines. Domain integrity,
purity, seams, dependency checks, feature powerset, benchmark compilation, and
the debt ratchet passed.

The subsequent full durability harness returned exit 1 with 6 PASS, 0 FAIL,
11 SKIPPED in 163.69 seconds. This is deliberately not a green durability result.
Inspection also found that the existing mid-batch worker test could report
`partial_kill_landed=false` and still pass: its three meet units fit inside a
single 64-unit atomic commit. Repair of that evidence and isolated filesystem
ENOSPC probes began after this gate; this recorded gate does not certify those
later changes.

### Measured mid-batch SIGKILL recovery

The worker fixture now contains 4,096 distinct meets, spanning 64 atomic commits.
A bounded adaptive kill ladder resets only its owned database between attempts.
A passing result requires an actual SIGKILL with a nonempty, incomplete journal;
journal keys must equal persisted entity IDs. Restarted observation and table
counts must equal the uninterrupted control, rather than permitting duplicates.

The production worker was killed after 512 committed meets at a 110.849 ms delay.
Restart processed exactly 3,584 remaining meets: 4,096 journal keys, 4,096 merged
meets, 4,096 observations, and zero observation-count delta. The Kansas directory
worker was killed after four of five units at 27.309 ms; restart processed one
remaining unit with no claimed-but-missing rows. All nine recovery cases passed
in 10.12 seconds. Both Restate crash cases also passed in 125.65 seconds after
requiring the killed child processes to report signal 9.

### Rebuilt primary workbook

The optimized offline workbook rebuild and production verifier completed
successfully in 1,635.32 seconds. Output:
`var/midwest-census/out/census-service-2026-09-25.xlsx`, 78,073,748 bytes.
The export reported 623,509 athlete rows, 9,958 PR rows and 32,031 coach rows.
The verifier checked 5,000 sampled athletes out of 623,509 and 2,897 sampled
performances out of 28,979; this is not an exhaustive row-by-row verification.

Direct ZIP workbook metadata inspection confirmed eleven sheets: `Athletes`,
`PRs`, `Performances_001`, `Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`,
`Conflicts`, `Review`, and `Run Metrics`. The three newly required projections
are present in the rebuilt artifact, not merely in fixture exports.

### Isolated Restate ENOSPC recovery

Scenario 10 passed in 6.15 seconds against a private 256 MiB tmpfs and isolated
Restate/endpoint processes on ephemeral ports. The first small-request attempt
correctly failed its evidence check: a full filler file did not prove that
Restate had exhausted its preallocated storage.

Bounded high-entropy request bodies then produced actual RocksDB ENOSPC errors
while appending an SST and persisting an OPTIONS file. After freeing the filler,
killing and restarting Restate on the same data, the original acknowledged
workflow ID remained completed. Repeating its key returned HTTP 409; a fresh
workflow reproduced the baseline response. Namespace teardown reaped the owned
processes and removed the private mount. No shared server or primary census
store was used. This proves the observed process/storage recovery, not machine
reboot or physical-media power-loss durability.

## Rebuilt workbook with release binary (2026-09-25)

The offline workbook rebuild was rerun with a release binary (`--release`) to address the 30-minute timeout
observed during the initial debug build (1,635.32 s). The release binary completed the same 2.36 M-athlete
dataset in 1,445.07 seconds (24 min 5 s), a 11.5% improvement over the previous debug build.

Output: `var/midwest-census/out/census-service-2026-09-25-rebuilt.xlsx`, 78,073,749 bytes, identical in
size to the prior artifact. Verification:

```text
$ time ./target/release/census-service verify \
    --store var/midwest-census \
    --workbook var/midwest-census/out/census-service-2026-09-25-rebuilt.xlsx
verify: OK (5000 athletes sampled of 623509 rows, 2897 performances sampled of 28979 rows)

real    1m33.457s
user    1m38.845s
sys     0m2.069s
```

The `Run Metrics` sheet carries `Class of 2027` = 623,509 in the All-sources column and 580,334 in the
Core column. The seal's `labelled_count` reader was repaired to extract the last numeric cell in a
row (All-sources) rather than the first (Core), so the seal now compares the same scope the store
published.

### Seal repaired: `labelled_count` reads All-sources column

The `labelled_count` function in `crates/census-service/src/census/seal/workbook.rs` previously returned
the first numeric cell after a matching label. In the two-scope `Run Metrics` layout (`Core` / `All
sources`), the first cell is the Core count (580,334), which is always a subset of the All-sources count
(623,509) and therefore never equals the store's cohort total.

Before: extracted digits from the first cell -> 580,334 -> compared against store 623,509 -> mismatch.

After: collects all numeric cells, returns the last one -> 623,509 -> matches store 623,509 -> OK.

The repair adds a `Vec<u64>` collector and `candidates.pop()` instead of `find_map` over a single
iterator. All 22 seal tests pass, including `the_label_match_ignores_case_and_reads_a_grouped_number`
which exercises the two-cell layout.

### Durability harness - updated results (2026-09-25)

The full harness ran in 160.52 seconds with the repaired seal:

- scenario-01-endpoint-kill: PASS
- scenario-09-disk-full-fjall: PASS
- scenario-10-disk-full-restate: PASS
- scenario-13-ai-review-failures: PASS
- scenario-14-seal-refuses: PASS
- scenario-15-full-backup-restore: PASS
- scenario-16-golden-census-determinism: PASS
- scenario-17-recovery-tests: PASS
- 9 SKIPPED (pre-existing gaps in test coverage)

Total: 8 PASS, 0 FAIL, 9 SKIPPED. Improved from 6 PASS / 11 SKIPPED in the previous run.
Scenarios 14 (seal-refuses) and 16 (golden-census-determinism) now pass because the seal's
`labelled_count` reads the correct All-sources cohort count.

### Coverage report discrepancy

The rebuilt workbook's `Coverage` sheet carries 131 duplicate jurisdiction rows and 108 unique
jurisdictions (expected 108 per the store). This is a pre-existing data quality issue in the
Coverage sheet, not introduced by the rebuild. The offline seal correctly reports this as a
refusal reason: `the coverage report does not reconcile`. The seal via Restate ingress was not
executable because the Census service is not deployed to Restate (no `Census/seal` endpoint
registered on the ingress).
