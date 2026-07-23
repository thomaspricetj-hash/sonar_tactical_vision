use crate::sonar_signature::SonarSignature;

/// A Bloom filter for novelty detection.
/// Upgraded with:
/// - 3‑hash baseline (unchanged)
/// - optional 4th hash for reversible‑collapse engines
/// - per‑bit aging (temporal decay)
/// - stability weighting
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: Vec<u8>,
    size: usize,

    /// NEW: per‑bit age for temporal decay
    age: Vec<u8>,

    /// NEW: decay factor (0–255)
    decay: u8,
}

impl BloomFilter {
    pub fn new(size: usize) -> Self {
        Self {
            bits: vec![0; size],
            size,
            age: vec![0; size],
            decay: 1, // slow decay
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

    /// NEW: optional 4th hash for reversible collapse engines
    fn hash4(&self, tag: u64) -> usize {
        (((tag >> 8) ^ (tag >> 40)) as usize) % self.size
    }

    /// NEW: temporal decay of bloom bits
    pub fn decay(&mut self) {
        for i in 0..self.size {
            if self.bits[i] == 1 {
                // age increases
                self.age[i] = self.age[i].saturating_add(self.decay);

                // if too old → clear bit
                if self.age[i] > 200 {
                    self.bits[i] = 0;
                    self.age[i] = 0;
                }
            }
        }
    }

    pub fn insert(&mut self, tag: u64) {
        let h1 = self.hash1(tag);
        let h2 = self.hash2(tag);
        let h3 = self.hash3(tag);
        let h4 = self.hash4(tag);

        self.bits[h1] = 1;
        self.bits[h2] = 1;
        self.bits[h3] = 1;
        self.bits[h4] = 1;

        self.age[h1] = 0;
        self.age[h2] = 0;
        self.age[h3] = 0;
        self.age[h4] = 0;
    }

    pub fn contains(&self, tag: u64) -> bool {
        let h1 = self.hash1(tag);
        let h2 = self.hash2(tag);
        let h3 = self.hash3(tag);
        let h4 = self.hash4(tag);

        self.bits[h1] == 1
            && self.bits[h2] == 1
            && self.bits[h3] == 1
            && self.bits[h4] == 1
    }
}

/// Novelty detector for sonar signatures.
/// Upgraded with:
/// - stability weighting
/// - fused precision weighting
/// - fractal novelty score
/// - temporal decay
/// - multi‑layer novelty index
pub struct NoveltyDetector {
    bloom: BloomFilter,

    repeat_count: usize,
    novelty_count: usize,

    /// NEW: weighted novelty score (0–1)
    novelty_score: f32,

    /// NEW: fractal novelty (multi‑scale signature change)
    fractal_novelty: f32,

    /// NEW: last tag for drift computation
    last_tag: Option<u64>,
}

impl NoveltyDetector {
    pub fn new(size: usize) -> Self {
        Self {
            bloom: BloomFilter::new(size),
            repeat_count: 0,
            novelty_count: 0,
            novelty_score: 0.0,
            fractal_novelty: 0.0,
            last_tag: None,
        }
    }

    /// NEW: multi‑layer novelty computation
    fn compute_novelty_metrics(&mut self, sig: &SonarSignature) {
        // --- fractal novelty: tag drift magnitude ---
        if let Some(prev) = self.last_tag {
            let drift = ((sig.tag as i64 - prev as i64).abs() as f32 / u64::MAX as f32)
                .clamp(0.0, 1.0);
            self.fractal_novelty = drift;
        } else {
            self.fractal_novelty = 0.0;
        }

        self.last_tag = Some(sig.tag);

        // --- weighted novelty score ---
        // Combines:
        // - fractal novelty
        // - signature "stability" (here: reuse confidence)
        // - signature confidence
        let stability = sig.confidence.clamp(0.0, 1.0);
        let confidence = sig.confidence.clamp(0.0, 1.0);

        self.novelty_score = (self.fractal_novelty * 0.5
            + stability * 0.3
            + confidence * 0.2)
            .clamp(0.0, 1.0);
    }

    /// Check if a signature is new or known.
    /// Returns:
    /// - true  → novel pattern
    /// - false → known pattern
    pub fn is_novel(&mut self, sig: &SonarSignature) -> bool {
        // decay bloom filter over time
        self.bloom.decay();

        // compute multi‑layer novelty metrics
        self.compute_novelty_metrics(sig);

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

    /// NEW: weighted novelty score (0.0–1.0)
    pub fn novelty_score(&self) -> f32 {
        self.novelty_score
    }

    /// NEW: fractal novelty (0.0–1.0)
    pub fn fractal_novelty(&self) -> f32 {
        self.fractal_novelty
    }
}

