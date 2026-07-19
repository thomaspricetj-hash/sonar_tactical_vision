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
}

/// Standalone deep store for sonar snapshots.
/// This is intentionally simple and independent.
/// Later, SyntheticMind can replace this with its own DeepStore adapter.
pub struct SonarDeepStore {
    snapshots: Vec<SonarSnapshot>,
    extractor: SonarSignatureExtractor,
}

impl SonarDeepStore {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            extractor: SonarSignatureExtractor::new(),
        }
    }

    /// Store a snapshot of the current sonar state.
    /// This mirrors your memory system’s "write" operation,
    /// but stays fully standalone.
    pub fn store(&mut self, heatmap: &MultiLayerHeatmap, timestamp: f64) {
        let fused = heatmap.fuse();
        let temporal = heatmap.temporal_layer.clone();
        let predictive = heatmap.predictive_layer.clone();

        // Convert motion vectors into magnitude heatmap for storage
        let mut motion_mag = HeatLayer::new(heatmap.motion_layer.width, heatmap.motion_layer.height);
        for y in 0..heatmap.motion_layer.height {
            for x in 0..heatmap.motion_layer.width {
                let (vx, vy) = heatmap.motion_layer.get(x, y);
                let mag = (vx * vx + vy * vy).sqrt().min(1.0);
                motion_mag.set(x, y, mag);
            }
        }

        let signature = self.extractor.extract(&fused);

        let snapshot = SonarSnapshot {
            fused,
            temporal,
            predictive,
            motion_mag,
            signature,
            timestamp,
        };

        self.snapshots.push(snapshot);
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

    /// Count stored snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}
