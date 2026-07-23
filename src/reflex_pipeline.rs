use crate::events::TacticalEvent;
use crate::semantic_layer::SemanticResult;
use crate::sonar_fusion::FusionState;
use crate::sonar_router::CrossSectionMap;

/// Reflex actions driven by sonar + fusion + roundabout routing.
#[derive(Debug, Clone)]
pub enum ReflexAction {
    EmergencyStop,
    SlowDown,
    SteerAway { angle_deg: f32 },
    MarkHazard,
    None,
}

/// MAX‑tier reflex pipeline with roundabout routing.
pub struct ReflexPipeline {
    sensitivity: f32,
}

impl ReflexPipeline {
    pub fn new(sensitivity: f32) -> Self {
        Self {
            sensitivity: sensitivity.clamp(0.1, 10.0),
        }
    }

    /// Legacy sonar‑only reflex (kept for fallback).
    pub fn handle(
        &self,
        event: &TacticalEvent,
        semantic: Option<&SemanticResult>,
    ) -> ReflexAction {
        let sev = event.severity() * self.sensitivity;

        if sev >= 0.9 {
            ReflexAction::EmergencyStop
        } else if sev >= 0.5 {
            ReflexAction::SlowDown
        } else if sev >= 0.2 {
            let angle = event.direction_deg().unwrap_or(0.0);
            ReflexAction::SteerAway { angle_deg: angle }
        } else {
            if let Some(sem) = semantic {
                if matches!(
                    sem.label,
                    crate::semantic_layer::SemanticLabel::HighRiskZone
                        | crate::semantic_layer::SemanticLabel::PersistentObstacle
                        | crate::semantic_layer::SemanticLabel::DirectionalHazard
                ) {
                    return ReflexAction::MarkHazard;
                }
            }
            ReflexAction::None
        }
    }

    /// MAX‑tier reflex decision using:
    /// - fusion confidence
    /// - roundabout routing
    /// - hazard pressure
    /// - temporal stability
    /// - motion drift
    /// - semantic hazard zones
    pub fn handle_roundabout(
        &self,
        events: &[TacticalEvent],
        semantic: &SemanticResult,
        fusion: Option<&FusionState>,
        cross: &CrossSectionMap,
    ) -> ReflexAction {
        // --- 1. Fusion emergency override ---
        if let Some(f) = fusion {
            if matches!(f.recommended_reflex, ReflexAction::EmergencyStop)
                && f.fused_confidence >= 0.5
            {
                return ReflexAction::EmergencyStop;
            }
        }

        // --- 2. Critical sonar events override ---
        for event in events {
            if event.is_critical() {
                return ReflexAction::EmergencyStop;
            }
        }

        // --- 3. Highest severity sonar event ---
        let mut best: Option<&TacticalEvent> = None;
        let mut best_sev = 0.0;

        for event in events {
            let sev = event.severity() * self.sensitivity;
            if sev > best_sev {
                best_sev = sev;
                best = Some(event);
            }
        }

        // --- 4. Precision + stability override ---
        if cross.fused_precision < 0.25 || cross.temporal_stability < 0.4 {
            return ReflexAction::SlowDown;
        }

        // --- 5. Motion drift override (roundabout drift detection) ---
        if cross.motion_dx.abs() > 0.5 || cross.motion_dy.abs() > 0.5 {
            return ReflexAction::SteerAway { angle_deg: 45.0 };
        }

        // --- 6. Hazard pressure override ---
        if cross.front_intensity > cross.back_intensity && cross.hazard > 0.5 {
            return ReflexAction::SlowDown;
        }

        // --- 7. Roundabout routing: exit‑bias steering ---
        if cross.roundabout_score > 0.5 {
            let angle = cross.exit_bias_deg;

            // Strong bias → commit to steering
            if angle.abs() >= 15.0 {
                return ReflexAction::SteerAway {
                    angle_deg: angle.clamp(-90.0, 90.0),
                };
            }
        }

        // --- 8. Fusion slow‑down override ---
        if let Some(f) = fusion {
            if matches!(f.recommended_reflex, ReflexAction::SlowDown)
                && f.fused_confidence >= 0.5
                && best_sev < 0.5
            {
                return ReflexAction::SlowDown;
            }
        }

        // --- 9. Semantic hazard fallback ---
        if matches!(
            semantic.label,
            crate::semantic_layer::SemanticLabel::HighRiskZone
                | crate::semantic_layer::SemanticLabel::PersistentObstacle
                | crate::semantic_layer::SemanticLabel::DirectionalHazard
        ) {
            return ReflexAction::MarkHazard;
        }

        // --- 10. Fall back to legacy reflex pipeline ---
        if let Some(event) = best {
            return self.handle(event, Some(semantic));
        }

        ReflexAction::None
    }

    /// Backwards‑compatible fusion handler (now roundabout‑aware).
    pub fn handle_with_fusion(
        &self,
        events: &[TacticalEvent],
        semantic: &SemanticResult,
        fusion: Option<&FusionState>,
        cross: &CrossSectionMap,
    ) -> ReflexAction {
        self.handle_roundabout(events, semantic, fusion, cross)
    }
}

