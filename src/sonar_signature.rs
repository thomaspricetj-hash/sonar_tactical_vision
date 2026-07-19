use crate::heatmap::HeatLayer;

/// A lightweight, standalone signature extracted from sonar heatmaps.
/// Inspired by SyntheticMind PTS, but NOT tied to the memory system.
///
/// This allows the sonar engine to produce semantic tags, pattern IDs,
/// and confidence scores without requiring SyntheticMind to be installed.
#[derive(Debug, Clone)]
pub struct SonarSignature {
    /// A compact pattern ID (hash of spatial pattern)
    pub tag: u64,

    /// Confidence score (0.0–1.0)
    pub confidence: f32,

    /// Optional semantic label (soft-contact, transparent-object, etc.)
    pub label: Option<String>,
}

/// Signature extractor for sonar heatmaps.
/// Converts spatial patterns into compact tags and confidence scores.
pub struct SonarSignatureExtractor;

impl SonarSignatureExtractor {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate a signature from a fused heatmap.
    /// This is intentionally simple and standalone:
    /// - Computes a spatial hash
    /// - Computes a confidence score
    /// - Optionally assigns a semantic label
    pub fn extract(&self, layer: &HeatLayer) -> SonarSignature {
        let tag = self.hash_layer(layer);
        let confidence = self.compute_confidence(layer);
        let label = self.assign_label(layer, confidence);

        SonarSignature {
            tag,
            confidence,
            label,
        }
    }

    /// Simple spatial hash of the heatmap.
    /// This mirrors your PTS "pattern → signature" idea,
    /// but stays independent from SyntheticMind.
    fn hash_layer(&self, layer: &HeatLayer) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis

        for v in &layer.cells {
            let byte = (v.clamp(0.0, 1.0) * 255.0) as u8;
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        hash
    }

    /// Confidence score based on average risk.
    /// This mirrors your memory system’s "pattern strength" concept.
    fn compute_confidence(&self, layer: &HeatLayer) -> f32 {
        let sum: f32 = layer.cells.iter().sum();
        let avg = sum / (layer.cells.len() as f32);
        avg.clamp(0.0, 1.0)
    }

    /// Optional semantic label assignment.
    /// This is intentionally simple and standalone.
    fn assign_label(&self, layer: &HeatLayer, confidence: f32) -> Option<String> {
        // Transparent object signature: low risk but sharp edges
        let mut edge_sum = 0.0;
        for y in 0..layer.height {
            for x in 0..layer.width {
                let center = layer.get(x, y);
                let right = if x < layer.width - 1 { layer.get(x + 1, y) } else { center };
                let diff = (center - right).abs();
                edge_sum += diff;
            }
        }

        let edge_avg = edge_sum / (layer.width * layer.height) as f32;

        if confidence < 0.15 && edge_avg > 0.25 {
            return Some("transparent_object".into());
        }

        if confidence > 0.6 {
            return Some("high_risk_zone".into());
        }

        None
    }
}
