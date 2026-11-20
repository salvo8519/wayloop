//! Helpers shared by the tests of this crate, written once.

/// `SplitMix64`: a generator of pseudo-random numbers whose fixed seed gives the same sequence
/// on every run and platform, so that a randomised test is reproducible.
pub(crate) struct SplitMix64(pub(crate) u64);

impl SplitMix64 {
    /// The next number of the sequence.
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number below `bound`; `bound` is not zero.
    pub(crate) fn below(&mut self, bound: u64) -> usize {
        usize::try_from(self.next() % bound).unwrap()
    }
}

mod tests {
    use super::SplitMix64;

    #[test]
    fn the_sequence_is_the_published_one() {
        // The first outputs of SplitMix64 seeded with 1234567, from the reference
        // implementation of Vigna (prng.di.unimi.it/splitmix64.c).
        let mut random = SplitMix64(1_234_567);
        assert_eq!(random.next(), 6_457_827_717_110_365_317);
        assert_eq!(random.next(), 3_203_168_211_198_807_973);
    }
}
