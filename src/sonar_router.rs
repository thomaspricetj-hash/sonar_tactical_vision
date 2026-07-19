use crate::engine::TacticalVisionEngine;
use crate::events::TacticalEvent;
use crate::sonar_signature::{SonarSignatureExtractor, SonarSignature};
use crate::semantic_layer::{SemanticLayer, SemanticResult};
use crate::reflex_pipeline::{ReflexPipeline, ReflexAction};
use crate::hazard_map::HazardMap;

/// Unified output from the sonar router.
#[derive(Debug, Clone)]
pub struct SonarRouterOutput {
    pub events: Vec<TacticalEvent>,
    pub signature: SonarSignature,
    pub semantic: SemanticResult,
    pub reflex: ReflexAction,
}

/// The sonar router ties together:
/// - TacticalVisionEngine
/// - SignatureExtractor
/// - SemanticLayer
/// - ReflexPipeline
/// - HazardMap
///
/// Fully standalone. Future‑ready for SyntheticMind routing.
pub struct SonarRouter<D: crate::device::SonarDevice> {
    engine: TacticalVisionEngine<D>,
    signature_extractor: SonarSignatureExtractor,
    semantic_layer: SemanticLayer,
    reflex_pipeline: ReflexPipeline,
    hazard_map: HazardMap,
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
            hazard_map: HazardMap::new(hazard_width, hazard_height, hazard_decay, hazard_reinforce),
        }
    }

    /// Full routing cycle:
    /// 1. Engine update → events
    /// 2. Fused heatmap → signature
    /// 3. Signature → semantic meaning
    /// 4. Event + semantic → reflex
    /// 5. Hazard map update
    /// 6. Unified output
    pub fn route(&mut self, _timestamp: f64) -> SonarRouterOutput {
        // --- 1. Engine update ---
        let events = self.engine.update();

        // --- 2. Fused heatmap ---
        let fused = self.engine.heatmap.fuse();

        // --- 3. Signature extraction ---
        let signature = self.signature_extractor.extract(&fused);

        // --- 4. Semantic classification ---
        let semantic = self.semantic_layer.classify(&fused, signature.clone());

        // --- 5. Reflex decision ---
        let reflex = self.select_reflex(&events, &semantic);

        // --- 6. Hazard map update ---
        self.hazard_map.decay();
        self.hazard_map.reinforce(&fused, Some(&semantic), Some(&reflex));

        // --- 7. Unified output ---
        SonarRouterOutput {
            events,
            signature,
            semantic,
            reflex,
        }
    }

    /// Select highest‑priority reflex.
    fn select_reflex(
        &self,
        events: &[TacticalEvent],
        semantic: &SemanticResult,
    ) -> ReflexAction {
        // Critical events override everything
        for event in events {
            if event.is_critical() {
                return ReflexAction::EmergencyStop;
            }
        }

        // Highest severity event wins
        let mut best: Option<&TacticalEvent> = None;
        let mut best_sev = 0.0;

        for event in events {
            let sev = event.severity();
            if sev > best_sev {
                best_sev = sev;
                best = Some(event);
            }
        }

        if let Some(event) = best {
            return self.reflex_pipeline.handle(event, Some(semantic));
        }

        ReflexAction::None
    }

    /// Access hazard map
    pub fn hazard_map(&self) -> &HazardMap {
        &self.hazard_map
    }
}
