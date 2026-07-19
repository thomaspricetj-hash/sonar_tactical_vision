use crate::events::TacticalEvent;
use crate::semantic_layer::SemanticResult;
use crate::sonar_fusion::FusionState;

/// Reflex actions driven by sonar + fusion.
#[derive(Debug, Clone)]
pub enum ReflexAction {
    EmergencyStop,
    SlowDown,
    SteerAway { angle_deg: f32 },
    MarkHazard,
    None,
}

/// Fusion‑aware reflex pipeline.
pub struct ReflexPipeline {
    sensitivity: f32,
}

impl ReflexPipeline {
    pub fn new(sensitivity: f32) -> Self {
        Self {
            sensitivity: sensitivity.clamp(0.1, 10.0),
        }
    }

    /// Original sonar‑only reflex handler.
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

    /// Fusion‑aware reflex decision.
    pub fn handle_with_fusion(
        &self,
        events: &[TacticalEvent],
        semantic: &SemanticResult,
        fusion: Option<&FusionState>,
    ) -> ReflexAction {
        // 1. Fusion emergency override.
        if let Some(f) = fusion {
            if matches!(f.recommended_reflex, ReflexAction::EmergencyStop)
                && f.fused_confidence >= 0.5
            {
                return ReflexAction::EmergencyStop;
            }
        }

        // 2. Critical sonar events override.
        for event in events {
            if event.is_critical() {
                return ReflexAction::EmergencyStop;
            }
        }

        // 3. Highest‑severity sonar event.
        let mut best: Option<&TacticalEvent> = None;
        let mut best_sev = 0.0;

        for event in events {
            let sev = event.severity();
            if sev > best_sev {
                best_sev = sev;
                best = Some(event);
            }
        }

        // 4. Fusion slow‑down override.
        if let Some(f) = fusion {
            if matches!(f.recommended_reflex, ReflexAction::SlowDown)
                && f.fused_confidence >= 0.5
                && best_sev < 0.5
            {
                return ReflexAction::SlowDown;
            }
        }

        // 5. Fall back to sonar‑only reflex.
        if let Some(event) = best {
            return self.handle(event, Some(semantic));
        }

        // 6. Semantic hazard fallback.
        if matches!(
            semantic.label,
            crate::semantic_layer::SemanticLabel::HighRiskZone
                | crate::semantic_layer::SemanticLabel::PersistentObstacle
                | crate::semantic_layer::SemanticLabel::DirectionalHazard
        ) {
            ReflexAction::MarkHazard
        } else {
            ReflexAction::None
        }
    }
}

