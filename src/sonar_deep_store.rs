use crate::heatmap::{HeatLayer, MultiLayerHeatmap};
use crate::sonar_signature::{SonarSignature, SonarSignatureExtractor};

/// A single stored sonar snapshot.
/// This mirrors your SyntheticMind DeepStore item structure,
/// but remains completely standalone.
#[derive(Debug, Clone)]
pub struct SonarSnapshot {
    /// Fused spatial heatmap (raw + field + edge + temporal + predictive + motion)
    pub fused: HeatLayer,

    /// Temporal memory layer at the time of capture
    pub temporal: HeatLayer,

    /// Predictive forward-projection layer
    pub predictive: HeatLayer,

    /// Motion vector magnitude layer (converted to heatmap for storage)
    pub motion_mag: HeatLayer,

    /// Compact signature (PTS-inspired)
    pub signature: SonarSignature,

    /// Timestamp (seconds since start)
    pub timestamp: f64,

    /// MAX‑tier: fused precision score
    pub fused_precision: f32,

    /// MAX‑tier: fractal complexity score
    pub fractal_complexity: f32,

    /// MAX‑tier: temporal stability score
    pub temporal_stability: f32,

    /// MAX‑tier: roundabout routing score
    pub roundabout_score: f32,
}

/// Standalone deep store for sonar snapshots.
/// This is intentionally simple and independent.
/// Later, SyntheticMind can replace this with its own DeepStore adapter.
pub struct SonarDeepStore {
    snapshots: Vec<SonarSnapshot>,
    extractor: SonarSignatureExtractor,

    /// Optional max snapshot count (auto‑trim)
    max_snapshots: Option<usize>,
}

impl SonarDeepStore {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            extractor: SonarSignatureExtractor::new(),
            max_snapshots: None,
        }
    }

    /// Set a maximum number of stored snapshots (oldest are trimmed).
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.max_snapshots = Some(limit);
        self
    }

    /// Store a snapshot of the current sonar state.
    /// This mirrors your memory system’s "write" operation,
    /// but stays fully standalone.
    pub fn store(&mut self, heatmap: &MultiLayerHeatmap, timestamp: f64) {
        let fused = heatmap.fuse();
        let temporal = heatmap.temporal_layer.clone();
        let predictive = heatmap.predictive_layer.clone();

        // Convert motion vectors into magnitude heatmap for storage
        let mut motion_mag =
            HeatLayer::new(heatmap.motion_layer.width, heatmap.motion_layer.height);
        for y in 0..heatmap.motion_layer.height {
            for x in 0..heatmap.motion_layer.width {
                let (vx, vy) = heatmap.motion_layer.get(x, y);
                let mag = (vx * vx + vy * vy).sqrt().min(1.0);
                motion_mag.set(x, y, mag);
            }
        }

        let signature = self.extractor.extract(&fused);

        // Local MAX‑tier metrics derived from fused layer only
        let (fused_precision, fractal_complexity, temporal_stability, roundabout_score) =
            Self::compute_metrics(&fused);

        let snapshot = SonarSnapshot {
            fused,
            temporal,
            predictive,
            motion_mag,
            signature,
            timestamp,
            fused_precision,
            fractal_complexity,
            temporal_stability,
            roundabout_score,
        };

        self.snapshots.push(snapshot);

        // Auto‑trim oldest snapshots if limit is set
        if let Some(limit) = self.max_snapshots {
            if self.snapshots.len() > limit {
                let excess = self.snapshots.len() - limit;
                self.snapshots.drain(0..excess);
            }
        }
    }

    /// Internal helper: compute precision/complexity/stability/roundabout metrics
    fn compute_metrics(layer: &HeatLayer) -> (f32, f32, f32, f32) {
        let cells = &layer.cells;
        if cells.is_empty() {
            return (0.0, 0.0, 1.0, 0.0);
        }

        let len = cells.len() as f32;

        // Entropy‑like slice
        let entropy = cells.iter().map(|v| v.abs()).sum::<f32>() / len;

        // Volatility slice
        let mut volatility = 0.0;
        for w in cells.windows(2) {
            volatility += (w[1] - w[0]).abs();
        }
        volatility /= len;

        // Simple hazard proxy: average intensity
        let hazard = cells.iter().map(|c| c.clamp(0.0, 1.0)).sum::<f32>() / len;

        // Fused precision: low entropy, low volatility, low hazard
        let fused_precision = (1.0 / (1.0 + entropy))
            * (1.0 / (1.0 + volatility))
            * (1.0 - hazard.clamp(0.0, 1.0));

        // Fractal complexity: interaction of entropy and volatility
        let fractal_complexity = (entropy * volatility).clamp(0.0, 1.0);

        // Temporal stability: DeepStore has no previous frame context → assume stable
        let temporal_stability = 1.0;

        // Roundabout score: prefer high precision, low hazard, moderate complexity
        let roundabout_score = fused_precision
            * (1.0 - hazard.clamp(0.0, 1.0))
            * (0.5 + 0.5 * (1.0 - fractal_complexity));

        (
            fused_precision.clamp(0.0, 1.0),
            fractal_complexity,
            temporal_stability,
            roundabout_score.clamp(0.0, 1.0),
        )
    }

    /// Retrieve the most recent snapshot.
    pub fn latest(&self) -> Option<&SonarSnapshot> {
        self.snapshots.last()
    }

    /// Retrieve all snapshots.
    pub fn all(&self) -> &[SonarSnapshot] {
        &self.snapshots
    }

    /// Retrieve snapshots matching a signature tag.
    pub fn find_by_tag(&self, tag: u64) -> Vec<&SonarSnapshot> {
        self.snapshots
            .iter()
            .filter(|snap| snap.signature.tag == tag)
            .collect()
    }

    /// Retrieve snapshots with high confidence (semantic memory).
    pub fn high_confidence(&self, threshold: f32) -> Vec<&SonarSnapshot> {
        self.snapshots
            .iter()
            .filter(|snap| snap.signature.confidence >= threshold)
            .collect()
    }

    /// Retrieve snapshots with high fused precision.
    pub fn high_precision(&self, threshold: f32) -> Vec<&SonarSnapshot> {
        self.snapshots
            .iter()
            .filter(|snap| snap.fused_precision >= threshold)
            .collect()
    }

    /// Retrieve snapshots with strong roundabout routing score.
    pub fn strong_roundabout(&self, threshold: f32) -> Vec<&SonarSnapshot> {
        self.snapshots
            .iter()
            .filter(|snap| snap.roundabout_score >= threshold)
            .collect()
    }

    /// Count stored snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

