//! The corpus's one stochastic source: the seed and the 32-bit LCG every synthetic value varies
//! through.
//!
//! Synthetic corpora have to be identical on every machine and every run, so nothing here reads
//! entropy or the clock. The generator only ever chooses a *variant* — which event an athlete runs
//! and how far their time is jittered from that event's base — and every value the harness asserts
//! against is derived from the recipe that built it, never read back out of the stream.

/// The corpus seed: this one number reproduces a run byte for byte.
pub(super) const SEED: u32 = 0x5eed_2027;

/// Deterministic 32-bit LCG (Numerical Recipes constants): no `rand` dependency, identical corpus
/// for a given seed on every machine.
pub(super) struct Lcg(u32);

impl Lcg {
    pub(super) fn new(seed: u32) -> Self {
        Self(seed)
    }

    pub(super) fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}
