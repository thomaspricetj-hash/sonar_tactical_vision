use crate::{
    SonarArray,
    TacticalField,
    TacticalEvent,
    SonarDevice,
};
use crate::heatmap::{HeatLayer, MultiLayerHeatmap};

/// TacticalVisionEngine
/// Now includes multi‑layered dot‑matrix heatmaps for synthetic tactile vision,
/// temporal memory, predictive risk, and motion‑vector flow.
pub struct TacticalVisionEngine<D: SonarDevice> {
    pub array: SonarArray<D>,
    pub field: TacticalField,

    /// Multi‑layered spatial heatmap stack.
    pub heatmap: MultiLayerHeatmap,

    /// Collision threshold (0.0–1.0)
    pub collision_threshold: f32,

    /// Proximity threshold (0.0–1.0)
    pub proximity_threshold: f32,
}

impl<D: SonarDevice> TacticalVisionEngine<D> {
    /// Create a new engine with sonar array, tactical field, and heatmap stack.
    pub fn new(array: SonarArray<D>, field: TacticalField, heatmap: MultiLayerHeatmap) -> Self {
        Self {
            array,
            field,
            heatmap,
            collision_threshold: 0.85,
            proximity_threshold: 0.55,
        }
    }

    /// Perform a full sonar scan → update tactical field → update heatmap → generate events.
    pub fn update(&mut self) -> Vec<TacticalEvent> {
        let readings = self.array.ping_all();
        let mut events = Vec::new();

        // --- 1. Update tactical field from sonar readings ---
        for (i, distance) in readings.iter().enumerate() {
            self.field.update_cell(i, *distance);
        }

        // --- 2. Build heatmap layers from sonar + tactical field ---
        self.build_heatmap_layers(&readings);

        // Smooth all layers for spatial coherence
        self.heatmap.smooth_all();

        // Fuse layers into a composite heatmap
        let fused = self.heatmap.fuse();

        // Use fused heatmap for temporal accumulation, predictive projection, and motion vectors
        self.heatmap.update_temporal(&fused, 0.9, 0.6);
        self.heatmap.update_predictive(&fused);
        self.heatmap.update_motion_vectors(&fused);

        // --- 3. Generate reflex events from fused heatmap + tactical field ---

        let fused_max = fused.cells.iter().copied().fold(0.0, f32::max);
        let temporal_max = self.heatmap.temporal_layer.cells.iter().copied().fold(0.0, f32::max);
        let predictive_max = self.heatmap.predictive_layer.cells.iter().copied().fold(0.0, f32::max);

        // 3A — Fused heatmap severity → collision reflex
        if fused_max >= self.collision_threshold {
            events.push(TacticalEvent::CollisionImminent(fused_max));
        }

        // 3B — Predictive collision reflex (forward‑projected risk)
        if predictive_max >= self.collision_threshold * 0.9 {
            events.push(TacticalEvent::PredictiveCollision {
                projected_risk: predictive_max,
            });
        }

        // 3C — Temporal memory reflex (persistent hazard)
        if temporal_max >= self.proximity_threshold {
            events.push(TacticalEvent::TemporalHazard {
                accumulated_risk: temporal_max,
            });
        }

        // 3D — Soft‑contact detection (low‑distance + low‑risk gradient)
        for (i, dist) in readings.iter().enumerate() {
            if *dist < 0.12 && self.field.cells[i] < 0.35 {
                events.push(TacticalEvent::SoftContact {
                    distance: *dist,
                    risk: self.field.cells[i],
                });
            }
        }

        // 3E — Transparent object detection (distance low, risk low)
        for (i, dist) in readings.iter().enumerate() {
            let risk = self.field.cells[i];
            if *dist < 0.25 && risk < 0.15 {
                events.push(TacticalEvent::TransparentObject {
                    distance: *dist,
                    risk,
                });
            }
        }

        // 3F — Motion‑vector reflex: global flow hazard + steering suggestion
        let (flow_mag, steer_x, steer_y) = self.compute_flow_metrics(&fused);
        if flow_mag >= 0.4 {
            events.push(TacticalEvent::MotionFlowHazard {
                flow_magnitude: flow_mag,
            });

            events.push(TacticalEvent::AvoidanceSteer {
                steer_x,
                steer_y,
            });
        }

        // 3G — Directional hazard prediction (front‑biased risk)
        let directional_risk = self.compute_directional_risk(&fused);
        if directional_risk >= self.proximity_threshold {
            events.push(TacticalEvent::DirectionalHazard {
                forward_risk: directional_risk,
            });
        }

        // --- Existing proximity + edge + unknown contact logic ---

        // Proximity detection
        let max_risk = self.field.max_risk();
        if max_risk >= self.proximity_threshold && max_risk < self.collision_threshold {
            events.push(TacticalEvent::ObjectVeryClose(max_risk));
        }

        // Edge detection (risk discontinuity between adjacent cells)
        for i in 0..self.field.resolution {
            let left = self.field.cells[i];
            let right = self.field.cells[(i + 1) % self.field.resolution];

            let diff = (left - right).abs();
            if diff > 0.35 {
                events.push(TacticalEvent::EdgeDetected {
                    left_risk: left,
                    right_risk: right,
                });
            }
        }

        // Unknown contact detection (unexpected patterns)
        for (i, distance) in readings.iter().enumerate() {
            if *distance < 0.05 {
                events.push(TacticalEvent::UnknownContact {
                    distance: *distance,
                    risk: self.field.cells[i],
                });
            }
        }

        events
    }

    /// Compute global flow magnitude and avoidance steering vector from motion layer.
    fn compute_flow_metrics(&self, fused: &HeatLayer) -> (f32, f32, f32) {
        let w = fused.width;
        let h = fused.height;

        let mut total_mag = 0.0;
        let mut count = 0;
        let mut steer_x = 0.0;
        let mut steer_y = 0.0;

        for y in 0..h {
            for x in 0..w {
                let (vx, vy) = self.heatmap.motion_layer.get(x, y);
                let mag = (vx * vx + vy * vy).sqrt();

                if mag > 0.0 {
                    total_mag += mag;
                    count += 1;

                    // Steering is opposite of flow (move away from hazard flow)
                    steer_x -= vx * fused.get(x, y);
                    steer_y -= vy * fused.get(x, y);
                }
            }
        }

        let avg_mag = if count > 0 {
            (total_mag / count as f32).min(1.0)
        } else {
            0.0
        };

        // Normalize steering vector
        let norm = (steer_x * steer_x + steer_y * steer_y).sqrt();
        if norm > 0.0 {
            steer_x /= norm;
            steer_y /= norm;
        }

        (avg_mag, steer_x, steer_y)
    }

    /// Compute directional hazard risk (front‑biased).
    fn compute_directional_risk(&self, fused: &HeatLayer) -> f32 {
        let w = fused.width;
        let h = fused.height;

        // Assume "front" is the upper half of the grid.
        let mut sum = 0.0;
        let mut count = 0;

        for y in 0..(h / 2) {
            for x in 0..w {
                let v = fused.get(x, y);
                sum += v;
                count += 1;
            }
        }

        if count > 0 {
            (sum / count as f32).min(1.0)
        } else {
            0.0
        }
    }

    /// Build multi‑layer heatmaps from sonar readings and tactical field.
    fn build_heatmap_layers(&mut self, readings: &[f32]) {
        self.heatmap.layers.clear();

        let w = 32;
        let h = 32;

        // --- Layer 1: Raw sonar distance → heat ---
        let mut raw_layer = HeatLayer::new(w, h);
        for (i, dist) in readings.iter().enumerate() {
            let risk = 1.0 - (dist / self.field.radius);
            let x = i % w;
            let y = (i / w).min(h - 1);
            raw_layer.set(x, y, risk.clamp(0.0, 1.0));
        }
        self.heatmap.add_layer(raw_layer);

        // --- Layer 2: Tactical field risk projected into 2D ---
        let mut field_layer = HeatLayer::new(w, h);
        for (i, risk) in self.field.cells.iter().enumerate() {
            let x = i % w;
            let y = (i / w).min(h - 1);
            field_layer.set(x, y, *risk);
        }
        self.heatmap.add_layer(field_layer);

        // --- Layer 3: Edge detection layer ---
        let mut edge_layer = HeatLayer::new(w, h);
        for i in 0..self.field.resolution {
            let left = self.field.cells[i];
            let right = self.field.cells[(i + 1) % self.field.resolution];
            let diff = (left - right).abs();

            let x = i % w;
            let y = (i / w).min(h - 1);

            edge_layer.set(x, y, diff.clamp(0.0, 1.0));
        }
        self.heatmap.add_layer(edge_layer);
    }
}
