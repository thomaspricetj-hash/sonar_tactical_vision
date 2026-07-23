use crate::heatmap::HeatLayer;
use crate::sonar_signature::SonarSignature;
use crate::novelty::NoveltyDetector;

/// MAX‑tier semantic labels with roundabout‑aware routing.
#[derive(Debug, Clone)]
pub enum SemanticLabel {
    TransparentObject,
    SoftContact,
    HighRiskZone,
    MotionFlowHazard,
    DirectionalHazard,
    PersistentObstacle,
    CurvatureExit,          // NEW: roundabout escape curvature zone
    LateralEscapeLane,      // NEW: left/right safe escape lane
    ForwardPressureHazard,  // NEW: strong front hazard pressure
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

/// MAX‑tier semantic classifier with:
/// - curvature inference
/// - centroid drift
/// - escape‑lane detection
/// - hazard pressure detection
/// - multi‑path gradient analysis
/// - novelty detection
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
    /// - curvature inference
    /// - centroid drift
    /// - escape‑lane detection
    /// - hazard pressure
    /// - novelty detection
    pub fn classify(&mut self, fused: &HeatLayer, sig: SonarSignature) -> SemanticResult {
        let mut confidence = sig.confidence;

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

        // --- High‑risk zone (signature‑based) ---
        if confidence > 0.65 {
            return SemanticResult {
                label: SemanticLabel::HighRiskZone,
                confidence,
                signature: sig,
            };
        }

        // --- Soft contact detection ---
        if confidence < 0.25 && self.detect_soft_contact(fused) {
            return SemanticResult {
                label: SemanticLabel::SoftContact,
                confidence,
                signature: sig,
            };
        }

        // --- Motion flow hazard ---
        if self.detect_motion_flow(fused) {
            return SemanticResult {
                label: SemanticLabel::MotionFlowHazard,
                confidence,
                signature: sig,
            };
        }

        // --- Directional hazard (front pressure) ---
        if self.detect_directional_hazard(fused) {
            return SemanticResult {
                label: SemanticLabel::DirectionalHazard,
                confidence,
                signature: sig,
            };
        }

        // --- Curvature exit detection (roundabout escape zone) ---
        if self.detect_curvature_exit(fused) {
            confidence *= 1.15; // boost confidence for escape lanes
            return SemanticResult {
                label: SemanticLabel::CurvatureExit,
                confidence,
                signature: sig,
            };
        }

        // --- Lateral escape lane detection ---
        if self.detect_lateral_escape_lane(fused) {
            confidence *= 1.10;
            return SemanticResult {
                label: SemanticLabel::LateralEscapeLane,
                confidence,
                signature: sig,
            };
        }

        // --- Forward pressure hazard (roundabout hazard zone) ---
        if self.detect_forward_pressure(fused) {
            confidence *= 0.85; // reduce confidence due to hazard pressure
            return SemanticResult {
                label: SemanticLabel::ForwardPressureHazard,
                confidence,
                signature: sig,
            };
        }

        // --- Novel pattern detection ---
        if self.novelty.is_novel(&sig) {
            return SemanticResult {
                label: SemanticLabel::NovelPattern,
                confidence,
                signature: sig,
            };
        }

        // --- Persistent obstacle ---
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

    /// NEW: Detect curvature exit (roundabout escape zone).
    fn detect_curvature_exit(&self, fused: &HeatLayer) -> bool {
        let w = fused.width as usize;
        let h = fused.height as usize;

        let mut curvature_sum = 0.0;
        let mut count = 0;

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let c = fused.get(x, y);
                let l = fused.get(x - 1, y);
                let r = fused.get(x + 1, y);
                let u = fused.get(x, y - 1);
                let d = fused.get(x, y + 1);

                let curvature = (l + r + u + d - 4.0 * c).abs();
                curvature_sum += curvature;
                count += 1;
            }
        }

        curvature_sum / count.max(1) as f32 > 0.12
    }

    /// NEW: Detect lateral escape lane (left/right safe zones).
    fn detect_lateral_escape_lane(&self, fused: &HeatLayer) -> bool {
        let w = fused.width as usize;
        let h = fused.height as usize;

        let mut left_sum = 0.0;
        let mut right_sum = 0.0;
        let mut left_count = 0;
        let mut right_count = 0;

        for y in 0..h {
            for x in 0..w {
                let v = fused.get(x, y);
                if x < w / 2 {
                    left_sum += v;
                    left_count += 1;
                } else {
                    right_sum += v;
                    right_count += 1;
                }
            }
        }

        let left_avg = left_sum / left_count.max(1) as f32;
        let right_avg = right_sum / right_count.max(1) as f32;

        // Escape lane = low intensity compared to global average
        let global_avg = (left_avg + right_avg) * 0.5;

        left_avg < global_avg * 0.75 || right_avg < global_avg * 0.75
    }

    /// NEW: Detect forward pressure hazard (roundabout hazard zone).
    fn detect_forward_pressure(&self, fused: &HeatLayer) -> bool {
        let w = fused.width as usize;
        let h = fused.height as usize;

        let mut front_sum = 0.0;
        let mut front_count = 0;

        for y in 0..h / 2 {
            for x in 0..w {
                front_sum += fused.get(x, y);
                front_count += 1;
            }
        }

        let front_avg = front_sum / front_count.max(1) as f32;

        front_avg > 0.45
    }
}


