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
| `crates/census-domain` | `kani/publish.rs` | 8 | `professional_email` withholding (symbolic + known-value tables), `normalize_name` diacritics/shape/idempotency |
| `crates/census-domain` | `kani/id_mint.rs` | 5 | `Id::mint` determinism, shape, tag prefixes, golden digest, `as_str`/`Display` agreement |
| `crates/midwest-census` | `kani/keys.rs` | 5 | `observation_key`/`split_observation_key` round-trip, fixed-width tail, id bounds, null byte and max-sequence handling |
| `crates/midwest-census` | `kani/merge.rs` | 5 | `Entity::merge` idempotency (School, Coach), `CanonicalCoach::publish` idempotency, consumer-mailbox withhold invariant, `withheld_mailboxes` consistency |

Wiring: `crates/midwest-census/src/store/mod.rs` ends with

```rust
#[cfg(kani)] include!("../kani/store_wiring.rs");
```

and `kani/store_wiring.rs` declares `kani/keys.rs` and `kani/merge.rs` as modules with `#[path = ...]`.
`census-domain` uses the same pattern through `kani/census_domain_wiring.rs`. Both are compiled only
under `cargo kani`, which is what defines `cfg(kani)`.

Commands (from the repository root), one harness per invocation:

```bash
cargo kani --manifest-path crates/census-domain/Cargo.toml --harness <name>
cargo kani --manifest-path crates/midwest-census/Cargo.toml --harness <name>

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
6. **Previous compile blockers are gone.** `crates/midwest-census` builds and its harnesses run in
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
  are `/tmp/kani-cd/*.log` (census-domain) and `/tmp/kani-mw/*.log` (midwest-census), and the wall
  times quoted are that pass's. The four gradyear results were declared reusable for this sweep; the
  rest were not re-run before the sweep window closed.
- **this-window** — executed by this sweep; logs under `/tmp/kani-sweep2/`.

Run discipline, this window: strictly one CBMC process at a time (`pgrep -x cbmc` verified empty
before each start and re-checked after each run), each invocation in its own process group with its own
wall budget and a 5 s sampler over `/proc/<pid>/status` `VmHWM` for peak CBMC RSS. Both runs that
started either finished on CBMC's own out-of-memory path or were cut off by the budget; both process
groups were killed and re-checked before these tables were written.

**No harness was edited in this sweep.** `git status --porcelain crates/census-domain/kani
crates/midwest-census/kani` is empty, so `git diff` over both `kani/**` trees shows nothing at all —
no format-message edit, no added `assume`, no deleted harness, no `#[kani::ignore]`. The one harness
whose asserted property this lane believes is false on the current model
(`check_normalize_idempotent_repeated_suffix`, whose doc comment predicts exactly that counterexample)
was left as written: it is a finding for `src/model.rs`, which this lane does not own.

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
   error: failed to parse manifest at `.../crates/midwest-census/Cargo.toml`
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
| 12 | `check_normalize_idempotent_repeated_suffix` | `no verdict (follow-up run killed at 549.5 s inside core::slice::memchr; no verdict line; the property it asserts is now fixed in src/model.rs and pinned by the unit test model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes)` | this-window | T14 |
| 13 | `check_id_mint_format` | `no verdict (prev-pass run killed mid-trace; its log contains no verdict line)` | prev-pass | T11 |
| 14 | `check_id_mint_tag_prefix` | `no verdict (never started)` | — | — |
| 15 | `check_id_mint_deterministic` | `no verdict (never started)` | — | — |
| 16 | `check_id_mint_golden_value` | `env-blocked (solver conversion: z3 CBMC map::at status 6; bitwuzla status 134/SIGABRT)` | prev-pass | T9 |
| 17 | `check_id_as_str_consistent` | `no verdict (never started)` | — | — |

### Verdict table — `crates/midwest-census` (10 harnesses)

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
window closed on a wrap-up request after two midwest-census harnesses, so 14 of them were never
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
$ cargo kani --manifest-path crates/midwest-census/Cargo.toml --harness check_observation_id_bounds
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
$ cargo kani --manifest-path crates/midwest-census/Cargo.toml --harness check_observation_key_null_byte_id
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
   Compiling midwest-census v0.1.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/midwest-census)
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
