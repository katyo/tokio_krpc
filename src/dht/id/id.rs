pub trait NodeId {
    /// Number of leading bits that are identical between two hashes
    fn equal_bits(&self, other: &Self) -> usize;

    fn nearest_of(&self, a: &Self, b: &Self) -> bool {
        self.equal_bits(a) > self.equal_bits(b)
    }
}
