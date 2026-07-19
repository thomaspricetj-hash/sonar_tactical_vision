/// TacticalEvent
/// Reflex‑style events generated from the sonar tactical field and multi‑layer heatmaps.
/// These represent near‑field conditions that require immediate, predictive,
/// directional, or flow‑based action.
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
        )
    }

    /// Returns a normalized severity score (0.0–1.0).
    pub fn severity(&self) -> f32 {
        match self {
            TacticalEvent::CollisionImminent(risk) => *risk,

            TacticalEvent::ObjectVeryClose(risk) => *risk * 0.7,

            TacticalEvent::EdgeDetected { left_risk, right_risk } => {
                ((*left_risk - *right_risk).abs()).min(1.0)
            }

            TacticalEvent::UnknownContact { risk, .. } => *risk * 0.5,

            TacticalEvent::PredictiveCollision { projected_risk } => *projected_risk,

            TacticalEvent::TemporalHazard { accumulated_risk } => accumulated_risk * 0.6,

            TacticalEvent::SoftContact { risk, .. } => *risk * 0.3,

            TacticalEvent::TransparentObject { risk, .. } => *risk * 0.2,

            TacticalEvent::MotionFlowHazard { flow_magnitude } => *flow_magnitude,

            TacticalEvent::DirectionalHazard { forward_risk } => *forward_risk,

            TacticalEvent::AvoidanceSteer { steer_x, steer_y } => {
                let mag = (steer_x * steer_x + steer_y * steer_y).sqrt();
                mag.min(1.0)
            }
        }
    }

    /// NEW: Direction accessor for steering logic.
    /// Converts event types into a steering angle when possible.
    pub fn direction_deg(&self) -> Option<f32> {
        match self {
            // AvoidanceSteer gives a real steering vector → convert to angle
            TacticalEvent::AvoidanceSteer { steer_x, steer_y } => {
                let angle = steer_y.atan2(*steer_x).to_degrees();
                Some(angle)
            }

            // DirectionalHazard is forward‑biased → straight ahead
            TacticalEvent::DirectionalHazard { .. } => Some(0.0),

            // MotionFlowHazard has no inherent direction
            TacticalEvent::MotionFlowHazard { .. } => None,

            // Predictive collision → straight ahead
            TacticalEvent::PredictiveCollision { .. } => Some(0.0),

            // Collision imminent → straight ahead
            TacticalEvent::CollisionImminent(_) => Some(0.0),

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
                    Some(-45.0) // steer left
                } else if right_risk > left_risk {
                    Some(45.0) // steer right
                } else {
                    None
                }
            }

            // ObjectVeryClose → straight ahead
            TacticalEvent::ObjectVeryClose(_) => Some(0.0),
        }
    }
}
