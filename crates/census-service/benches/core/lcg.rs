
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const INCREMENT: u64 = 1_442_695_040_888_963_407;

pub struct Lcg {
    state: u64,
}

impl Lcg {
    pub const fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_state(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        self.state
    }

    pub fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).unwrap_or(u64::MAX);
        if bound == 0 {
            return 0;
        }
        usize::try_from(self.next_state().checked_rem(bound).unwrap_or(0)).unwrap_or(0)
    }

    pub fn pick<T: Copy>(&mut self, items: &[T], fallback: T) -> T {
        items
            .get(self.below(items.len()))
            .copied()
            .unwrap_or(fallback)
    }
}
