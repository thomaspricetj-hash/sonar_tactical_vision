use crate::engine::TacticalVisionEngine;
use crate::events::TacticalEvent;
use crate::sonar_signature::{SonarSignatureExtractor, SonarSignature};
use crate::semantic_layer::{SemanticLayer, SemanticResult};
use crate::reflex_pipeline::{ReflexPipeline, ReflexAction};
use crate::hazard_map::HazardMap;
use crate::heatmap::HeatLayer;

/// Cross‑section mapping: multi‑axis precision + motion + temporal slices.
#[derive(Debug, Clone)]
pub struct CrossSectionMap {
    pub entropy: f32,
    pub volatility: f32,
    pub drift: f32,
    pub hazard: f32,
    pub semantic_weight: f32,
    pub fused_precision: f32,

    // Motion‑vector drift
    pub motion_dx: f32,
    pub motion_dy: f32,

    // Temporal stability
    pub temporal_stability: f32,

    // Multi‑layer cross‑sections (spatial slices)
    pub front_intensity: f32,
    pub back_intensity: f32,
    pub left_intensity: f32,
    pub right_intensity: f32,
}

impl CrossSectionMap {
    pub fn from_layer(
        layer: &HeatLayer,
        semantic: &SemanticResult,
        hazard_map: &HazardMap,
        prev_cells: Option<&[f32]>,
    ) -> Self {
        let fused = &layer.cells;

        // --- entropy slice ---
        let entropy = fused.iter().map(|v| v.abs()).sum::<f32>() / fused.len().max(1) as f32;

        // --- volatility slice ---
        let mut volatility = 0.0;
        for w in fused.windows(2) {
            volatility += (w[1] - w[0]).abs();
        }
        volatility /= fused.len().max(1) as f32;

        // --- drift slice ---
        let drift = fused
            .iter()
            .enumerate()
            .map(|(i, v)| (i as f32 * 0.01) * v)
            .sum::<f32>();

        // --- hazard slice ---
        let hazard = if hazard_map.cells.is_empty() {
            0.0
        } else {
            hazard_map
                .cells
                .iter()
                .map(|c| c.intensity)
                .sum::<f32>()
                / hazard_map.cells.len() as f32
        };

        // --- semantic slice ---
        let semantic_weight = semantic.confidence;

        // --- motion‑vector drift (approx gradient) ---
        let mut sum_dx = 0.0;
        let mut sum_dy = 0.0;
        let mut count = 0;

        let w = layer.width as usize;
        let h = layer.height as usize;

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let v = fused.get(idx).copied().unwrap_or(0.0);

                if x + 1 < w {
                    let right = fused.get(idx + 1).copied().unwrap_or(0.0);
                    sum_dx += right - v;
                    count += 1;
                }
                if y + 1 < h {
                    let down = fused.get(idx + w).copied().unwrap_or(0.0);
                    sum_dy += down - v;
                    count += 1;
                }
            }
        }

        let motion_dx = if count == 0 { 0.0 } else { sum_dx / count as f32 };
        let motion_dy = if count == 0 { 0.0 } else { sum_dy / count as f32 };

        // --- temporal stability (difference vs previous frame) ---
        let temporal_stability = if let Some(prev) = prev_cells {
            if prev.len() == fused.len() && !fused.is_empty() {
                let diff = fused
                    .iter()
                    .zip(prev.iter())
                    .map(|(a, b)| (a - b).abs())
                    .sum::<f32>()
                    / fused.len() as f32;
                1.0 / (1.0 + diff) // higher diff → lower stability
            } else {
                1.0
            }
        } else {
            1.0
        };

        // --- multi‑layer spatial cross‑sections ---
        let mut front_sum = 0.0;
        let mut back_sum = 0.0;
        let mut left_sum = 0.0;
        let mut right_sum = 0.0;
        let mut front_count = 0;
        let mut back_count = 0;
        let mut left_count = 0;
        let mut right_count = 0;

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let v = fused.get(idx).copied().unwrap_or(0.0);

                // front/back: split on Y
                if y < h / 2 {
                    front_sum += v;
                    front_count += 1;
                } else {
                    back_sum += v;
                    back_count += 1;
                }

                // left/right: split on X
                if x < w / 2 {
                    left_sum += v;
                    left_count += 1;
                } else {
                    right_sum += v;
                    right_count += 1;
                }
            }
        }

        let front_intensity = if front_count == 0 { 0.0 } else { front_sum / front_count as f32 };
        let back_intensity = if back_count == 0 { 0.0 } else { back_sum / back_count as f32 };
        let left_intensity = if left_count == 0 { 0.0 } else { left_sum / left_count as f32 };
        let right_intensity = if right_count == 0 { 0.0 } else { right_sum / right_count as f32 };

        // --- fused precision ---
        let fused_precision = (1.0 / (1.0 + entropy))
            * (1.0 / (1.0 + volatility))
            * (1.0 - hazard.clamp(0.0, 1.0))
            * semantic_weight.clamp(0.0, 1.0)
            * temporal_stability;

        CrossSectionMap {
            entropy,
            volatility,
            drift,
            hazard,
            semantic_weight,
            fused_precision,
            motion_dx,
            motion_dy,
            temporal_stability,
            front_intensity,
            back_intensity,
            left_intensity,
            right_intensity,
        }
    }
}

/// Unified output from the sonar router.
#[derive(Debug, Clone)]
pub struct SonarRouterOutput {
    pub events: Vec<TacticalEvent>,
    pub signature: SonarSignature,
    pub semantic: SemanticResult,
    pub reflex: ReflexAction,
    pub cross_sections: CrossSectionMap,
}

pub struct SonarRouter<D: crate::device::SonarDevice> {
    engine: TacticalVisionEngine<D>,
    signature_extractor: SonarSignatureExtractor,
    semantic_layer: SemanticLayer,
    reflex_pipeline: ReflexPipeline,
    hazard_map: HazardMap,
    last_fused: Option<Vec<f32>>,
}

impl<D: crate::device::SonarDevice> SonarRouter<D> {
    pub fn new(
        engine: TacticalVisionEngine<D>,
        novelty_bits: usize,
        reflex_sensitivity: f32,
        hazard_width: usize,
        hazard_height: usize,
        hazard_decay: f32,
        hazard_reinforce: f32,
    ) -> Self {
        Self {
            engine,
            signature_extractor: SonarSignatureExtractor::new(),
            semantic_layer: SemanticLayer::new(novelty_bits),
            reflex_pipeline: ReflexPipeline::new(reflex_sensitivity),
            hazard_map: HazardMap::new(
                hazard_width,
                hazard_height,
                hazard_decay,
                hazard_reinforce,
            ),
            last_fused: None,
        }
    }

    pub fn route(&mut self, _timestamp: f64) -> SonarRouterOutput {
        // --- 1. Engine update ---
        let events = self.engine.update();

        // --- 2. Fused heatmap (HeatLayer) ---
        let fused_layer = self.engine.heatmap.fuse();

        // --- 3. Signature extraction (expects HeatLayer) ---
        let signature = self.signature_extractor.extract(&fused_layer);

        // --- 4. Semantic classification (expects HeatLayer) ---
        let semantic = self.semantic_layer.classify(&fused_layer, signature.clone());

        // --- 5. Cross‑section mapping with motion + temporal + spatial slices ---
        let cross_sections = CrossSectionMap::from_layer(
            &fused_layer,
            &semantic,
            &self.hazard_map,
            self.last_fused.as_deref(),
        );

        // update temporal memory
        self.last_fused = Some(fused_layer.cells.clone());

        // --- 6. Reflex decision ---
        let reflex = self.select_reflex(&events, &semantic, &cross_sections);

        // --- 7. Hazard map update (expects HeatLayer) ---
        self.hazard_map.decay();
        self.hazard_map.reinforce(&fused_layer, Some(&semantic), Some(&reflex));

        SonarRouterOutput {
            events,
            signature,
            semantic,
            reflex,
            cross_sections,
        }
    }

    fn select_reflex(
        &self,
        events: &[TacticalEvent],
        semantic: &SemanticResult,
        cross: &CrossSectionMap,
    ) -> ReflexAction {
        // Critical override
        for event in events {
            if event.is_critical() {
                return ReflexAction::EmergencyStop;
            }
        }

        // Highest severity
        let mut best: Option<&TacticalEvent> = None;
        let mut best_sev = 0.0;

        for event in events {
            let sev = event.severity();
            if sev > best_sev {
                best_sev = sev;
                best = Some(event);
            }
        }

        // Precision + temporal stability override
        if cross.fused_precision < 0.25 || cross.temporal_stability < 0.4 {
            return ReflexAction::SlowDown;
        }

        // Motion‑vector drift override
        if cross.motion_dx.abs() > 0.5 || cross.motion_dy.abs() > 0.5 {
            return ReflexAction::SteerAway { angle_deg: 45.0 };
        }

        // Spatial cross‑section bias: front high + hazard → brake
        if cross.front_intensity > cross.back_intensity && cross.hazard > 0.5 {
            return ReflexAction::SlowDown;
        }

        if let Some(event) = best {
            return self.reflex_pipeline.handle(event, Some(semantic));
        }

        ReflexAction::None
    }

    pub fn hazard_map(&self) -> &HazardMap {
        &self.hazard_map
    }
}


