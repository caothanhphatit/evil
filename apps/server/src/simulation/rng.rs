#[derive(Clone, Debug)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn range_inclusive(&mut self, min: i32, max: i32) -> i32 {
        let value = self.next_u64();
        let width = (max - min + 1) as u64;
        min + ((value >> 32) % width) as i32
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    pub fn state(&self) -> u64 {
        self.state
    }
}
