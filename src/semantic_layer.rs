use crate::heatmap::HeatLayer;
use crate::sonar_signature::SonarSignature;
use crate::novelty::NoveltyDetector;

/// A semantic label assigned to sonar patterns.
/// This mirrors your SyntheticMind semantic lane,
/// but remains fully standalone.
#[derive(Debug, Clone)]
pub enum SemanticLabel {
    TransparentObject,
    SoftContact,
    HighRiskZone,
    MotionFlowHazard,
    DirectionalHazard,
    PersistentObstacle,
    NovelPattern,
    Unknown,
}

/// Semantic result containing:
/// - label
/// - confidence
/// - optional signature
#[derive(Debug, Clone)]
pub struct SemanticResult {
    pub label: SemanticLabel,
    pub confidence: f32,
    pub signature: SonarSignature,
}

/// Standalone semantic classifier.
/// Uses:
/// - fused heatmap
/// - signature
/// - novelty detector
/// - simple heuristics
///
/// Later, SyntheticMind can replace this with:
/// - PTS semantic routing
/// - latent semantic inference
/// - multi‑layer semantic fusion
pub struct SemanticLayer {
    novelty: NoveltyDetector,
}

impl SemanticLayer {
    pub fn new(novelty_bits: usize) -> Self {
        Self {
            novelty: NoveltyDetector::new(novelty_bits),
        }
    }

    /// Produce a semantic classification from:
    /// - fused heatmap
    /// - signature
    pub fn classify(&mut self, fused: &HeatLayer, sig: SonarSignature) -> SemanticResult {
        let confidence = sig.confidence;

        // Transparent object: low risk + sharp edges
        if let Some(label) = sig.label.clone() {
            if label == "transparent_object" {
                return SemanticResult {
                    label: SemanticLabel::TransparentObject,
                    confidence,
                    signature: sig,
                };
            }
        }

        // High‑risk zone
        if confidence > 0.65 {
            return SemanticResult {
                label: SemanticLabel::HighRiskZone,
                confidence,
                signature: sig,
            };
        }

        // Soft contact: low risk + localized spike
        if confidence < 0.25 && self.detect_soft_contact(fused) {
            return SemanticResult {
                label: SemanticLabel::SoftContact,
                confidence,
                signature: sig,
            };
        }

        // Motion flow hazard: strong directional gradient
        if self.detect_motion_flow(fused) {
            return SemanticResult {
                label: SemanticLabel::MotionFlowHazard,
                confidence,
                signature: sig,
            };
        }

        // Directional hazard: front‑biased risk
        if self.detect_directional_hazard(fused) {
            return SemanticResult {
                label: SemanticLabel::DirectionalHazard,
                confidence,
                signature: sig,
            };
        }

        // Novel pattern detection
        if self.novelty.is_novel(&sig) {
            return SemanticResult {
                label: SemanticLabel::NovelPattern,
                confidence,
                signature: sig,
            };
        }

        // Persistent obstacle: repeated signature
        if self.novelty.repeat_total() > 5 {
            return SemanticResult {
                label: SemanticLabel::PersistentObstacle,
                confidence,
                signature: sig,
            };
        }

        // Default fallback
        SemanticResult {
            label: SemanticLabel::Unknown,
            confidence,
            signature: sig,
        }
    }

    /// Detect soft‑contact pattern: small, localized high‑gradient region.
    fn detect_soft_contact(&self, fused: &HeatLayer) -> bool {
        let mut spikes = 0;

        for y in 0..fused.height {
            for x in 0..fused.width {
                let v = fused.get(x, y);
                if v > 0.3 && v < 0.5 {
                    spikes += 1;
                }
            }
        }

        spikes > (fused.width * fused.height) / 50
    }

    /// Detect motion‑flow hazard: strong directional gradient.
    fn detect_motion_flow(&self, fused: &HeatLayer) -> bool {
        let mut grad_sum = 0.0;

        for y in 0..fused.height {
            for x in 0..fused.width {
                let center = fused.get(x, y);
                let right = if x < fused.width - 1 { fused.get(x + 1, y) } else { center };
                grad_sum += (center - right).abs();
            }
        }

        grad_sum / (fused.width * fused.height) as f32 > 0.15
    }

    /// Detect directional hazard: front‑biased risk.
    fn detect_directional_hazard(&self, fused: &HeatLayer) -> bool {
        let h2 = fused.height / 2;
        let mut sum = 0.0;
        let mut count = 0;

        for y in 0..h2 {
            for x in 0..fused.width {
                sum += fused.get(x, y);
                count += 1;
            }
        }

        (sum / count as f32) > 0.4
    }
}
