use crate::sonar_signature::SonarSignature;

/// A simple Bloom filter implementation for novelty detection.
/// Inspired by your SyntheticMind bloom routing, but fully standalone.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: Vec<u8>,
    size: usize,
}

impl BloomFilter {
    pub fn new(size: usize) -> Self {
        Self {
            bits: vec![0; size],
            size,
        }
    }

    fn hash1(&self, tag: u64) -> usize {
        (tag as usize) % self.size
    }

    fn hash2(&self, tag: u64) -> usize {
        ((tag >> 16) as usize) % self.size
    }

    fn hash3(&self, tag: u64) -> usize {
        ((tag >> 32) as usize) % self.size
    }

    pub fn insert(&mut self, tag: u64) {
        let h1 = self.hash1(tag);
        let h2 = self.hash2(tag);
        let h3 = self.hash3(tag);

        self.bits[h1] = 1;
        self.bits[h2] = 1;
        self.bits[h3] = 1;
    }

    pub fn contains(&self, tag: u64) -> bool {
        let h1 = self.hash1(tag);
        let h2 = self.hash2(tag);
        let h3 = self.hash3(tag);

        self.bits[h1] == 1 && self.bits[h2] == 1 && self.bits[h3] == 1
    }
}

/// Novelty detector for sonar signatures.
/// Determines whether a signature is new, repeated, or recurring.
pub struct NoveltyDetector {
    bloom: BloomFilter,
    repeat_count: usize,
    novelty_count: usize,
}

impl NoveltyDetector {
    pub fn new(size: usize) -> Self {
        Self {
            bloom: BloomFilter::new(size),
            repeat_count: 0,
            novelty_count: 0,
        }
    }

    /// Check if a signature is new or known.
    /// Returns:
    /// - true  → novel pattern
    /// - false → known pattern
    pub fn is_novel(&mut self, sig: &SonarSignature) -> bool {
        let tag = sig.tag;

        if self.bloom.contains(tag) {
            self.repeat_count += 1;
            return false;
        }

        self.bloom.insert(tag);
        self.novelty_count += 1;
        true
    }

    /// Total number of novel patterns detected.
    pub fn novelty_total(&self) -> usize {
        self.novelty_count
    }

    /// Total number of repeated patterns detected.
    pub fn repeat_total(&self) -> usize {
        self.repeat_count
    }

    /// Novelty ratio (0.0–1.0)
    pub fn novelty_ratio(&self) -> f32 {
        let total = self.novelty_count + self.repeat_count;
        if total == 0 {
            return 0.0;
        }
        self.novelty_count as f32 / total as f32
    }
}
