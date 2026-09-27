pub(super) const SEED: u32 = 0x5eed_2027;

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
