/// TacticalEvent
/// Reflex‑style events generated from the sonar tactical field and multi‑layer heatmaps.
/// These represent near‑field conditions that require immediate, predictive,
/// directional, or flow‑based action, including roundabout‑aware, multi‑layer
/// precision and sector pressure.
#[derive(Debug, Clone)]
pub enum TacticalEvent {
    /// Collision is imminent. Value = risk level (0.0–1.0)
    CollisionImminent(f32),

    /// An object is very close but not yet at collision threshold.
    ObjectVeryClose(f32),

    /// A sharp change in risk between adjacent cells (edge detection).
    EdgeDetected {
        left_risk: f32,
        right_risk: f32,
    },

    /// A sonar reading that does not match expected patterns.
    UnknownContact {
        distance: f32,
        risk: f32,
    },

    /// Predictive forward‑projected collision (from predictive heatmap layer).
    PredictiveCollision {
        projected_risk: f32,
    },

    /// Temporal accumulation hazard (from temporal heatmap layer).
    TemporalHazard {
        accumulated_risk: f32,
    },

    /// Soft‑material contact (low‑distance + low‑risk gradient).
    SoftContact {
        distance: f32,
        risk: f32,
    },

    /// Transparent object detected (distance low, risk low).
    TransparentObject {
        distance: f32,
        risk: f32,
    },

    /// Motion‑vector flow hazard (global flow magnitude).
    MotionFlowHazard {
        flow_magnitude: f32,
    },

    /// Directional hazard prediction (front‑biased risk).
    DirectionalHazard {
        forward_risk: f32,
    },

    /// Flow‑based avoidance steering vector.
    /// steer_x, steer_y are normalized (-1.0..1.0)
    AvoidanceSteer {
        steer_x: f32,
        steer_y: f32,
    },

    /// Roundabout: forward pressure from cross‑sections (front + outer ring).
    RoundaboutForwardPressure {
        forward_pressure: f32,
    },

    /// Roundabout: lateral escape lane score (left/right intensity vs hazard).
    RoundaboutEscapeLane {
        escape_score: f32,
        steer_x: f32,
        steer_y: f32,
    },

    /// Multi‑layer precision index from fused cross‑sections.
    /// fused_precision: global confidence of spatial model
    /// fractal_precision: multi‑scale complexity score
    PrecisionIndex {
        fused_precision: f32,
        fractal_precision: f32,
    },
}

impl TacticalEvent {
    /// Returns true if this event indicates immediate danger.
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            TacticalEvent::CollisionImminent(_)
                | TacticalEvent::PredictiveCollision { .. }
                | TacticalEvent::MotionFlowHazard { .. }
                | TacticalEvent::DirectionalHazard { .. }
                | TacticalEvent::RoundaboutForwardPressure { .. }
        )
    }

    /// Returns a normalized severity score (0.0–1.0).
    pub fn severity(&self) -> f32 {
        match self {
            TacticalEvent::CollisionImminent(risk) => *risk,

            TacticalEvent::ObjectVeryClose(risk) => *risk * 0.7_f32,

            TacticalEvent::EdgeDetected { left_risk, right_risk } => {
                ((*left_risk - *right_risk).abs()).min(1.0_f32)
            }

            TacticalEvent::UnknownContact { risk, .. } => *risk * 0.5_f32,

            TacticalEvent::PredictiveCollision { projected_risk } => *projected_risk,

            TacticalEvent::TemporalHazard { accumulated_risk } => accumulated_risk * 0.6_f32,

            TacticalEvent::SoftContact { risk, .. } => *risk * 0.3_f32,

            TacticalEvent::TransparentObject { risk, .. } => *risk * 0.2_f32,

            TacticalEvent::MotionFlowHazard { flow_magnitude } => *flow_magnitude,

            TacticalEvent::DirectionalHazard { forward_risk } => *forward_risk,

            TacticalEvent::AvoidanceSteer { steer_x, steer_y } => {
                let mag = (steer_x * steer_x + steer_y * steer_y).sqrt();
                mag.min(1.0_f32)
            }

            TacticalEvent::RoundaboutForwardPressure { forward_pressure } => *forward_pressure,

            TacticalEvent::RoundaboutEscapeLane { escape_score, .. } => *escape_score,

            TacticalEvent::PrecisionIndex {
                fused_precision,
                fractal_precision,
            } => ((*fused_precision + *fractal_precision) * 0.5_f32).min(1.0_f32),
        }
    }

    /// NEW: Direction accessor for steering logic.
    /// Converts event types into a steering angle when possible.
    pub fn direction_deg(&self) -> Option<f32> {
        match self {
            // AvoidanceSteer gives a real steering vector → convert to angle
            TacticalEvent::AvoidanceSteer { steer_x, steer_y }
            | TacticalEvent::RoundaboutEscapeLane { steer_x, steer_y, .. } => {
                let angle = steer_y.atan2(*steer_x).to_degrees();
                Some(angle)
            }

            // DirectionalHazard is forward‑biased → straight ahead
            TacticalEvent::DirectionalHazard { .. } => Some(0.0_f32),

            // Roundabout forward pressure → straight ahead
            TacticalEvent::RoundaboutForwardPressure { .. } => Some(0.0_f32),

            // MotionFlowHazard has no inherent direction
            TacticalEvent::MotionFlowHazard { .. } => None,

            // Predictive collision → straight ahead
            TacticalEvent::PredictiveCollision { .. } => Some(0.0_f32),

            // Collision imminent → straight ahead
            TacticalEvent::CollisionImminent(_) => Some(0.0_f32),

            // TemporalHazard → no specific steering direction, just risk over time
            TacticalEvent::TemporalHazard { .. } => None,

            // UnknownContact → no direction
            TacticalEvent::UnknownContact { .. } => None,

            // SoftContact → no direction
            TacticalEvent::SoftContact { .. } => None,

            // TransparentObject → no direction
            TacticalEvent::TransparentObject { .. } => None,

            // EdgeDetected → direction depends on risk gradient
            TacticalEvent::EdgeDetected { left_risk, right_risk } => {
                if left_risk > right_risk {
                    Some(-45.0_f32) // steer left
                } else if right_risk > left_risk {
                    Some(45.0_f32) // steer right
                } else {
                    None
                }
            }

            // ObjectVeryClose → straight ahead
            TacticalEvent::ObjectVeryClose(_) => Some(0.0_f32),

            // PrecisionIndex has no direct steering direction
            TacticalEvent::PrecisionIndex { .. } => None,
        }
    }
}

/// High‑level summary over a batch of tactical events.
/// This is a multi‑layer index view: collision, proximity, flow, direction.
#[derive(Debug, Clone)]
pub struct TacticalSummary {
    pub max_severity: f32,
    pub critical_count: usize,
    pub collision_risk: f32,
    pub forward_risk: f32,
    pub flow_magnitude: f32,
    pub escape_score: f32,
    pub fused_precision: f32,
    pub fractal_precision: f32,
}

impl TacticalSummary {
    pub fn from_events(events: &[TacticalEvent]) -> Self {
        let mut max_severity = 0.0_f32;
        let mut critical_count = 0_usize;
        let mut collision_risk = 0.0_f32;
        let mut forward_risk = 0.0_f32;
        let mut flow_magnitude = 0.0_f32;
        let mut escape_score = 0.0_f32;
        let mut fused_precision = 0.0_f32;
        let mut fractal_precision = 0.0_f32;

        for e in events {
            let sev = e.severity();
            if sev > max_severity {
                max_severity = sev;
            }
            if e.is_critical() {
                critical_count += 1;
            }

            match e {
                TacticalEvent::CollisionImminent(r) => {
                    collision_risk = collision_risk.max(*r);
                }
                TacticalEvent::PredictiveCollision { projected_risk } => {
                    collision_risk = collision_risk.max(*projected_risk);
                }
                TacticalEvent::DirectionalHazard { forward_risk: fr } => {
                    forward_risk = forward_risk.max(*fr);
                }
                TacticalEvent::RoundaboutForwardPressure { forward_pressure } => {
                    forward_risk = forward_risk.max(*forward_pressure);
                }
                TacticalEvent::MotionFlowHazard { flow_magnitude: fm } => {
                    flow_magnitude = flow_magnitude.max(*fm);
                }
                TacticalEvent::RoundaboutEscapeLane { escape_score: es, .. } => {
                    escape_score = escape_score.max(*es);
                }
                TacticalEvent::PrecisionIndex {
                    fused_precision: fp,
                    fractal_precision: fracp,
                } => {
                    fused_precision = fused_precision.max(*fp);
                    fractal_precision = fractal_precision.max(*fracp);
                }
                _ => {}
            }
        }

        TacticalSummary {
            max_severity,
            critical_count,
            collision_risk,
            forward_risk,
            flow_magnitude,
            escape_score,
            fused_precision,
            fractal_precision,
        }
    }
}
