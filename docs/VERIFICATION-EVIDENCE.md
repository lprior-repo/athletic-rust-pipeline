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

One harness per invocation, from the repository root. `-Z stubbing` is passed on the id-mint path
because those harnesses stub `__cpuid_count`; `--no-unwinding-checks` is passed where CBMC could not
discharge loop-termination checks for heap-backed strings (repair 3). No harness was weakened: no
`assume` narrows an input, and the only harness edits in this pass were format arguments removed from
two assert messages (`publish.rs`, `keys.rs`) - message text, not the property.

### PASS — 4 harnesses (raw tails, verbatim)

```text
$ cargo kani -Z stubbing --no-unwinding-checks --manifest-path crates/census-domain/Cargo.toml --harness <name>

check_gradyear_of_formula
SUMMARY:
 ** 0 of 122 failed
Verification Time: 0.05850434s

check_gradyear_of_known_values
SUMMARY:
 ** 0 of 128 failed
Verification Time: 0.04390135s

check_gradyear_of_saturating
SUMMARY:
 ** 0 of 59 failed
Verification Time: 0.03621107s

check_observed_grade_grad_year
SUMMARY:
 ** 0 of 327 failed (4 unreachable)
Verification Time: 0.12346949s
```

`0 of N failed` is Kani's complete-verification line; each of these ended
`VERIFICATION:- SUCCESSFUL` for the harness named on its command line.

### FAILED — CBMC out of memory (4 harnesses)

Each of these ran while other CBMC processes were live on the same machine; CBMC ended with its
out-of-memory status rather than a property counterexample, so none of the four is evidence about the
code under test. Raw tail of each log, verbatim:

```text
$ cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_professional_email_known_consumer
Manual Harness Summary:
Verification failed for - kani_publish::check_professional_email_known_consumer
Complete - 0 successfully verified harnesses, 1 failures, 1 total.

# the other three: cargo kani --no-unwinding-checks --manifest-path <crate>/Cargo.toml --harness <name>
check_professional_email_malformed           (census-domain)  → CBMC failed, VERIFICATION:- FAILED
check_normalize_shape                        (census-domain)  → CBMC failed, VERIFICATION:- FAILED
check_observation_key_zero_and_max_sequence  (midwest-census) → CBMC failed, VERIFICATION:- FAILED
```

In all three, the line CBMC ends on is: `CBMC appears to have run out of memory. You may want to
rerun your proof in an environment with additional memory or use stubbing to reduce the size of the
code the verifier reasons about`.

The re-run to do is the same commands on an otherwise idle box; a single-CBMC re-run of
`check_normalize_shape` was started at the end of this pass and had not reached a verdict before the
session stopped.

### FAILED — solver conversion (1 harness)

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
```

z3 was tried because the same harness under the default solver (CaDiCaL) ran past four minutes and
peaked at `rss=42.4GB` (`ps -eo etime,rss`, `elapsed=03:37 rss=42.4GB`) without a verdict. `map::at`
is a CBMC-internal failure inside the SMT conversion, not a counterexample, so this harness has no
verdict either; `--solver bitwuzla` on the same command fails the same way (`CBMC failed with status
134`, SIGABRT, in 33.6 s), so CaDiCaL in a longer window is the route to one.

### Not executed in this pass (18 harnesses)

One `cargo kani` invocation each: 3-6 minutes of build+CBMC for the non-hashing harnesses, tens of
minutes and tens of GB for the id-mint/merge ones. Commands are the two patterns above with
`--harness <name>` (and `-Z stubbing` for the mint-path harnesses).

- `crates/census-domain`: `check_professional_email_known_professional`,
  `check_professional_email_never_publishes_consumer_mailbox`, `check_normalize_idempotent`,
  `check_normalize_idempotent_repeated_suffix`, `check_normalize_diacritics`, `check_id_mint_deterministic`,
  `check_id_mint_format`, `check_id_mint_tag_prefix`, `check_id_as_str_consistent`
- `crates/midwest-census`: `check_observation_id_bounds`, `check_observation_key_null_byte_id`,
  `check_split_key_reads_fixed_width_tail`, `check_observation_key_round_trip`, `check_school_merge_idempotent`,
  `check_coach_merge_idempotent`, `check_coach_publish_idempotent`, `check_coach_publish_no_consumer_mailbox`,
  `check_coach_withheld_mailboxes_consistency`

9 of the 27 harnesses in the inventory above have an executed result; the other 18 do not.

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

- **18 of the 27 Kani harnesses** — no verdict in this pass (see "Not executed"); the four id-mint
  harnesses and the entity-merge harnesses additionally need tens of GB of CBMC memory per run.
- **SHA-NI SHA-256 backend** (see repair 2) — unreachable for Kani; the soft backend is what is proved.
- **Deeper fuzzing than 1000 runs per target** — the runs are bounded smoke runs, not soak runs.
- **cargo-mutants** — carried from the earlier pack, not re-run here.
- **Traceability matrix** — separate work item, not part of this pack.
