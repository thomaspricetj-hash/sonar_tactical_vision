use crate::heatmap::HeatLayer;

/// A lightweight, standalone signature extracted from sonar heatmaps.
/// Upgraded with:
/// - stability score
/// - fractal drift score
/// - edge‑sharpness score
/// - reversible‑collapse ready structure
#[derive(Debug, Clone)]
pub struct SonarSignature {
    /// A compact pattern ID (hash of spatial pattern)
    pub tag: u64,

    /// Confidence score (0.0–1.0)
    pub confidence: f32,

    /// Optional semantic label (soft-contact, transparent-object, etc.)
    pub label: Option<String>,

    /// NEW: stability score (0.0–1.0)
    pub stability_score: f32,

    /// NEW: fractal drift score (0.0–1.0)
    pub fractal_drift: f32,

    /// NEW: edge sharpness score (0.0–1.0)
    pub edge_sharpness: f32,
}

/// Signature extractor for sonar heatmaps.
/// Converts spatial patterns into compact tags and confidence scores.
pub struct SonarSignatureExtractor;

impl SonarSignatureExtractor {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate a signature from a fused heatmap.
    /// Upgraded with:
    /// - spatial hash
    /// - confidence score
    /// - stability score
    /// - fractal drift
    /// - edge sharpness
    pub fn extract(&self, layer: &HeatLayer) -> SonarSignature {
        let tag = self.hash_layer(layer);
        let confidence = self.compute_confidence(layer);
        let edge_sharpness = self.compute_edge_sharpness(layer);
        let stability_score = self.compute_stability(layer);
        let fractal_drift = self.compute_fractal_drift(layer);

        let label = self.assign_label(layer, confidence, edge_sharpness);

        SonarSignature {
            tag,
            confidence,
            label,
            stability_score,
            fractal_drift,
            edge_sharpness,
        }
    }

    /// Simple spatial hash of the heatmap.
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
    fn compute_confidence(&self, layer: &HeatLayer) -> f32 {
        let sum: f32 = layer.cells.iter().sum();
        let avg = sum / (layer.cells.len() as f32);
        avg.clamp(0.0, 1.0)
    }

    /// NEW: stability score — low volatility = high stability.
    fn compute_stability(&self, layer: &HeatLayer) -> f32 {
        let cells = &layer.cells;
        if cells.len() < 2 {
            return 1.0;
        }

        let mut volatility = 0.0;
        for w in cells.windows(2) {
            volatility += (w[1] - w[0]).abs();
        }
        volatility /= cells.len() as f32;

        (1.0 / (1.0 + volatility)).clamp(0.0, 1.0)
    }

    /// NEW: fractal drift — local curvature magnitude.
    fn compute_fractal_drift(&self, layer: &HeatLayer) -> f32 {
        let w = layer.width as usize;
        let h = layer.height as usize;

        if w < 3 || h < 3 {
            return 0.0;
        }

        let mut sum = 0.0;
        let mut count = 0;

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let c = layer.get(x, y);
                let l = layer.get(x - 1, y);
                let r = layer.get(x + 1, y);
                let u = layer.get(x, y - 1);
                let d = layer.get(x, y + 1);

                let curvature = (l + r + u + d - 4.0 * c).abs();
                sum += curvature;
                count += 1;
            }
        }

        (sum / count.max(1) as f32).clamp(0.0, 1.0)
    }

    /// NEW: edge sharpness — strong gradients = sharp edges.
    fn compute_edge_sharpness(&self, layer: &HeatLayer) -> f32 {
        let w = layer.width as usize;
        let h = layer.height as usize;

        let mut sum = 0.0;
        let mut count = 0;

        for y in 0..h {
            for x in 0..w {
                let c = layer.get(x, y);
                let r = if x + 1 < w { layer.get(x + 1, y) } else { c };
                sum += (c - r).abs();
                count += 1;
            }
        }

        (sum / count.max(1) as f32).clamp(0.0, 1.0)
    }

    /// Optional semantic label assignment.
    fn assign_label(
        &self,
        layer: &HeatLayer,
        confidence: f32,
        edge_sharpness: f32,
    ) -> Option<String> {
        // Transparent object: low risk + sharp edges
        if confidence < 0.15 && edge_sharpness > 0.25 {
            return Some("transparent_object".into());
        }

        // High‑risk zone
        if confidence > 0.6 {
            return Some("high_risk_zone".into());
        }

        None
    }
}
