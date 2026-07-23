use crate::hazard_map::HazardMap;
use crate::reflex_pipeline::ReflexAction;

/// Abstracted external sensor inputs.
/// Later you can plug real camera/LiDAR/radar here.
#[derive(Debug, Clone)]
pub struct FusionSensors {
    /// Normalized obstacle confidence from vision (0.0–1.0)
    pub vision_obstacle_confidence: f32,
    /// Normalized obstacle confidence from LiDAR (0.0–1.0)
    pub lidar_obstacle_confidence: f32,
    /// Normalized obstacle confidence from radar (0.0–1.0)
    pub radar_obstacle_confidence: f32,
    /// Optional estimated distance to nearest obstacle (meters)
    pub nearest_distance_m: Option<f32>,
}

/// Fused situational awareness.
/// This is the “global picture” your robot/car sees.
#[derive(Debug, Clone)]
pub struct FusionState {
    pub fused_hazard_level: f32,   // 0.0–1.0
    pub fused_confidence: f32,     // 0.0–1.0
    pub recommended_reflex: ReflexAction,
}

/// Multi‑sensor fusion over:
/// - sonar hazard map
/// - external sensors (vision, LiDAR, radar)
/// - distance‑aware shaping
/// - roundabout‑tier reflex blending
pub struct SonarFusion {
    /// Weight of sonar vs other sensors
    sonar_weight: f32,
    vision_weight: f32,
    lidar_weight: f32,
    radar_weight: f32,

    /// Thresholds for reflex decisions
    emergency_threshold: f32,
    slow_threshold: f32,

    /// NEW: distance shaping factor
    distance_weight: f32,

    /// NEW: roundabout reflex blending
    roundabout_blend: f32,
}

impl SonarFusion {
    pub fn new() -> Self {
        Self {
            // Sonar is last‑meter truth → give it strong weight
            sonar_weight: 0.5,
            vision_weight: 0.2,
            lidar_weight: 0.2,
            radar_weight: 0.1,

            emergency_threshold: 0.8,
            slow_threshold: 0.4,

            // NEW: distance shaping (closer obstacles increase hazard)
            distance_weight: 0.25,

            // NEW: roundabout reflex blending (0.0–1.0)
            roundabout_blend: 0.35,
        }
    }

    /// Compute a scalar hazard level from the sonar hazard map.
    fn sonar_hazard_level(&self, map: &HazardMap) -> f32 {
        if map.width == 0 || map.height == 0 {
            return 0.0;
        }

        let mut sum = 0.0;
        let mut count = 0.0;

        for cell in &map.cells {
            sum += cell.intensity;
            count += 1.0;
        }

        if count == 0.0 {
            0.0
        } else {
            (sum / count).clamp(0.0, 1.0)
        }
    }

    /// NEW: distance‑aware hazard shaping
    fn distance_hazard(&self, nearest: Option<f32>) -> f32 {
        match nearest {
            None => 0.0,
            Some(d) => {
                // Closer → higher hazard
                let scaled = (1.0 / (1.0 + d)).clamp(0.0, 1.0);
                scaled * self.distance_weight
            }
        }
    }

    /// Fuse sonar + external sensors into a single hazard level and reflex.
    pub fn fuse(
        &self,
        map: &HazardMap,
        sensors: &FusionSensors,
    ) -> FusionState {
        let sonar_level = self.sonar_hazard_level(map);

        let fused_hazard =
            sonar_level * self.sonar_weight +
            sensors.vision_obstacle_confidence * self.vision_weight +
            sensors.lidar_obstacle_confidence * self.lidar_weight +
            sensors.radar_obstacle_confidence * self.radar_weight +
            self.distance_hazard(sensors.nearest_distance_m);

        let fused_hazard = fused_hazard.clamp(0.0, 1.0);

        // Confidence: more agreement between sensors → higher confidence
        let mut variance = 0.0;
        let vals = [
            sonar_level,
            sensors.vision_obstacle_confidence,
            sensors.lidar_obstacle_confidence,
            sensors.radar_obstacle_confidence,
        ];
        let mean = vals.iter().sum::<f32>() / vals.len() as f32;
        for v in vals.iter() {
            let d = *v - mean;
            variance += d * d;
        }
        variance /= vals.len() as f32;
        let fused_confidence = (1.0 - variance).clamp(0.0, 1.0);

        // Reflex recommendation based on fused hazard
        let mut recommended_reflex = if fused_hazard >= self.emergency_threshold {
            ReflexAction::EmergencyStop
        } else if fused_hazard >= self.slow_threshold {
            ReflexAction::SlowDown
        } else {
            ReflexAction::None
        };

        // NEW: roundabout reflex blending
        // If hazard is moderate, blend toward steering away
        if fused_hazard > 0.35 && fused_hazard < 0.65 {
            let steer_strength = (fused_hazard * 90.0 * self.roundabout_blend)
                .clamp(-90.0, 90.0);

            if steer_strength.abs() > 10.0 {
                recommended_reflex = ReflexAction::SteerAway {
                    angle_deg: steer_strength,
                };
            }
        }

        FusionState {
            fused_hazard_level: fused_hazard,
            fused_confidence,
            recommended_reflex,
        }
    }
}
