//! The one stochastic source in these benches: a seeded 64-bit linear congruential generator.
//!
//! Synthetic corpora have to be identical on every machine and every run, so nothing here reads
//! entropy or the clock. The stream is `state = state * MULTIPLIER + INCREMENT`, started from the
//! seed a corpus documents, and the generator only ever chooses a *variant* (a relay squad letter,
//! a publication order). Every value a bench asserts against is derived from the recipe that built
//! it, never read back out of the stream.

/// LCG multiplier (Knuth's MMIX constant, the base the PCG family starts from).
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
/// LCG increment (the MMIX constant).
const INCREMENT: u64 = 1_442_695_040_888_963_407;

/// A seeded linear congruential generator; one value per call, no other state.
pub struct Lcg {
    state: u64,
}

impl Lcg {
    /// The stream whose first state is `seed`, so recording one number reproduces a corpus exactly.
    pub const fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advance the stream and return the new state.
    fn next_state(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        self.state
    }

    /// A value in `0..bound`; a zero bound yields 0 rather than dividing by zero.
    pub fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).unwrap_or(u64::MAX);
        if bound == 0 {
            return 0;
        }
        usize::try_from(self.next_state() % bound).unwrap_or(0)
    }

    /// The stream-chosen element of `items`; `fallback` answers an empty table.
    pub fn pick<T: Copy>(&mut self, items: &[T], fallback: T) -> T {
        items
            .get(self.below(items.len()))
            .copied()
            .unwrap_or(fallback)
    }
}
