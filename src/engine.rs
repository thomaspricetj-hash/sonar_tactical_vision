use crate::{
    SonarArray,
    TacticalField,
    TacticalEvent,
    SonarDevice,
};
use crate::heatmap::{HeatLayer, MultiLayerHeatmap, FullCrossSectionSlices};
use crate::hazard_map::HazardMap;
use crate::array::EchoPacket; // FIX: correct module path

/// TacticalVisionEngine
/// Roundabout‑aware, multi‑layered synthetic tactile vision engine:
/// - sonar array with angular sectors, escape bias, forward pressure
/// - multi‑layer heatmap stack (raw, field, edge, temporal, predictive, motion)
/// - full cross‑section slices + fused precision index
pub struct TacticalVisionEngine<D: SonarDevice> {
    pub array: SonarArray<D>,
    pub field: TacticalField,

    /// Multi‑layered spatial heatmap stack.
    pub heatmap: MultiLayerHeatmap,

    /// Hazard map for fused spatial risk.
    pub hazard_map: HazardMap,

    /// Previous fused heatmap cells (for temporal stability).
    prev_fused: Vec<f32>,

    /// Collision threshold (0.0–1.0)
    pub collision_threshold: f32,

    /// Proximity threshold (0.0–1.0)
    pub proximity_threshold: f32,
}

impl<D: SonarDevice> TacticalVisionEngine<D> {
    /// Create a new engine with sonar array, tactical field, heatmap stack, and hazard map.
    pub fn new(
        array: SonarArray<D>,
        field: TacticalField,
        heatmap: MultiLayerHeatmap,
        hazard_map: HazardMap,
    ) -> Self {
        Self {
            array,
            field,
            heatmap,
            hazard_map,
            prev_fused: Vec::new(),
            collision_threshold: 0.85_f32,
            proximity_threshold: 0.55_f32,
        }
    }

    /// Full roundabout update:
    /// 1) ping sonar with angular sectors
    /// 2) update tactical field
    /// 3) build multi‑layer heatmaps
    /// 4) fuse + cross‑sections + precision index
    /// 5) generate tactical events
    pub fn update(&mut self) -> Vec<TacticalEvent> {
        // --- 1. Roundabout‑aware sonar ping ---
        let packets = self.array.ping_roundabout();
        let readings: Vec<f32> = packets.iter().map(|p| p.clamped_distance).collect();

        let mut events = Vec::new();

        // --- 2. Update tactical field from sonar readings ---
        for (i, distance) in readings.iter().enumerate() {
            self.field.update_cell(i, *distance);
        }

        // --- 3. Build heatmap layers from roundabout packets + tactical field ---
        self.build_heatmap_layers_roundabout(&packets);

        // Smooth all layers for spatial coherence
        self.heatmap.smooth_all();

        // Fuse layers into a composite heatmap
        let fused = self.heatmap.fuse();

        // Prepare previous fused snapshot for cross‑sections
        let prev_opt = if self.prev_fused.is_empty() {
            None
        } else {
            Some(&self.prev_fused[..])
        };

        // --- 4. Full cross‑section slices + fused precision index ---
        let slices: FullCrossSectionSlices =
            self.heatmap.full_cross_sections(prev_opt, Some(&self.hazard_map));

        // Store current fused for next temporal stability computation
        self.prev_fused = fused.cells.clone();

        // Use fused heatmap for temporal accumulation, predictive projection, and motion vectors
        self.heatmap.update_temporal(&fused, 0.9_f32, 0.6_f32);
        self.heatmap.update_predictive(&fused);
        self.heatmap.update_motion_vectors(&fused);

        // --- 5. Generate reflex events from fused heatmap + cross‑sections + tactical field ---

        let fused_max = fused.cells.iter().copied().fold(0.0_f32, f32::max);
        let temporal_max = self
            .heatmap
            .temporal_layer
            .cells
            .iter()
            .copied()
            .fold(0.0_f32, f32::max);
        let predictive_max = self
            .heatmap
            .predictive_layer
            .cells
            .iter()
            .copied()
            .fold(0.0_f32, f32::max);

        // Adaptive thresholds using fused precision:
        // higher precision → slightly lower thresholds (more sensitive)
        let precision = slices.fused_precision;
        let collision_thr =
            (self.collision_threshold * (1.0_f32 - 0.15_f32 * precision)).clamp(0.6_f32, 0.9_f32);
        let proximity_thr =
            (self.proximity_threshold * (1.0_f32 - 0.10_f32 * precision)).clamp(0.4_f32, 0.8_f32);

        // 5A — Fused heatmap severity → collision reflex
        if fused_max >= collision_thr {
            events.push(TacticalEvent::CollisionImminent(fused_max));
        }

        // 5B — Predictive collision reflex (forward‑projected risk)
        if predictive_max >= collision_thr * 0.9_f32 {
            events.push(TacticalEvent::PredictiveCollision {
                projected_risk: predictive_max,
            });
        }

        // 5C — Temporal memory reflex (persistent hazard)
        if temporal_max >= proximity_thr {
            events.push(TacticalEvent::TemporalHazard {
                accumulated_risk: temporal_max,
            });
        }

        // 5D — Soft‑contact detection (low‑distance + low‑risk gradient)
        for (i, dist) in readings.iter().enumerate() {
            if *dist < 0.12_f32 && self.field.cells[i] < 0.35_f32 {
                events.push(TacticalEvent::SoftContact {
                    distance: *dist,
                    risk: self.field.cells[i],
                });
            }
        }

        // 5E — Transparent object detection (distance low, risk low)
        for (i, dist) in readings.iter().enumerate() {
            let risk = self.field.cells[i];
            if *dist < 0.25_f32 && risk < 0.15_f32 {
                events.push(TacticalEvent::TransparentObject {
                    distance: *dist,
                    risk,
                });
            }
        }

        // 5F — Motion‑vector reflex: global flow hazard + steering suggestion
        let (flow_mag, steer_x, steer_y) = self.compute_flow_metrics(&fused);
        if flow_mag >= 0.4_f32 {
            events.push(TacticalEvent::MotionFlowHazard {
                flow_magnitude: flow_mag,
            });

            events.push(TacticalEvent::AvoidanceSteer {
                steer_x,
                steer_y,
            });
        }

        // 5G — Directional hazard prediction (front‑biased risk)
        let directional_risk = self.compute_directional_risk(&fused);
        if directional_risk >= proximity_thr {
            events.push(TacticalEvent::DirectionalHazard {
                forward_risk: directional_risk,
            });
        }

        // --- 5H — Roundabout‑aware risk shaping from cross‑sections ---

        // Forward pressure: outer hazard + front hazard
        let forward_pressure =
            ((slices.hazard_front + slices.hazard_outer) * 0.5_f32).clamp(0.0_f32, 1.0_f32);
        if forward_pressure >= 0.5_f32 {
            events.push(TacticalEvent::DirectionalHazard {
                forward_risk: forward_pressure,
            });
        }

        // Lateral escape bias: left/right hazard vs intensity
        let lateral_hazard = (slices.hazard_left + slices.hazard_right) * 0.5_f32;
        let lateral_intensity = (slices.left_intensity + slices.right_intensity) * 0.5_f32;
        let escape_score =
            (lateral_intensity - lateral_hazard).clamp(0.0_f32, 1.0_f32);

        if escape_score >= 0.3_f32 {
            // existing event type: use AvoidanceSteer with lateral bias
            events.push(TacticalEvent::AvoidanceSteer {
                steer_x: if slices.hazard_left < slices.hazard_right { -1.0_f32 } else { 1.0_f32 },
                steer_y: 0.0_f32,
            });
        }

        // --- Existing proximity + edge + unknown contact logic ---

        // Proximity detection
        let max_risk = self.field.max_risk();
        if max_risk >= proximity_thr && max_risk < collision_thr {
            events.push(TacticalEvent::ObjectVeryClose(max_risk));
        }

        // Edge detection (risk discontinuity between adjacent cells)
        for i in 0..self.field.resolution {
            let left = self.field.cells[i];
            let right = self.field.cells[(i + 1) % self.field.resolution];

            let diff = (left - right).abs();
            if diff > 0.35_f32 {
                events.push(TacticalEvent::EdgeDetected {
                    left_risk: left,
                    right_risk: right,
                });
            }
        }

        // Unknown contact detection (unexpected patterns)
        for (i, distance) in readings.iter().enumerate() {
            if *distance < 0.05_f32 {
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

        let mut total_mag = 0.0_f32;
        let mut count = 0;
        let mut steer_x = 0.0_f32;
        let mut steer_y = 0.0_f32;

        for y in 0..h {
            for x in 0..w {
                let (vx, vy) = self.heatmap.motion_layer.get(x, y);
                let mag = (vx * vx + vy * vy).sqrt();

                if mag > 0.0_f32 {
                    total_mag += mag;
                    count += 1;

                    // Steering is opposite of flow (move away from hazard flow)
                    let weight = fused.get(x, y);
                    steer_x -= vx * weight;
                    steer_y -= vy * weight;
                }
            }
        }

        let avg_mag = if count > 0 {
            (total_mag / count as f32).min(1.0_f32)
        } else {
            0.0_f32
        };

        // Normalize steering vector
        let norm = (steer_x * steer_x + steer_y * steer_y).sqrt();
        if norm > 0.0_f32 {
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
        let mut sum = 0.0_f32;
        let mut count = 0;

        for y in 0..(h / 2) {
            for x in 0..w {
                let v = fused.get(x, y);
                sum += v;
                count += 1;
            }
        }

        if count > 0 {
            (sum / count as f32).min(1.0_f32)
        } else {
            0.0_f32
        }
    }

    /// Build multi‑layer heatmaps from roundabout sonar packets and tactical field.
    fn build_heatmap_layers_roundabout(&mut self, packets: &[EchoPacket]) {
        self.heatmap.layers.clear();

        let w = 32;
        let h = 32;

        // --- Layer 1: Raw sonar distance → risk heat (stability‑weighted) ---
        let mut raw_layer = HeatLayer::new(w, h);
        for (i, p) in packets.iter().enumerate() {
            let base_risk = 1.0_f32 - (p.clamped_distance / self.field.radius);
            let risk =
                (base_risk * p.stability_score).clamp(0.0_f32, 1.0_f32);

            let x = i % w;
            let y = (i / w).min(h - 1);
            raw_layer.set(x, y, risk);
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

            edge_layer.set(x, y, diff.clamp(0.0_f32, 1.0_f32));
        }
        self.heatmap.add_layer(edge_layer);

        // --- Layer 4: Forward‑pressure layer (front sector emphasis) ---
        let mut forward_layer = HeatLayer::new(w, h);
        for (i, p) in packets.iter().enumerate() {
            let x = i % w;
            let y = (i / w).min(h - 1);
            let v = p.forward_pressure.clamp(0.0_f32, 1.0_f32);
            forward_layer.set(x, y, v);
        }
        self.heatmap.add_layer(forward_layer);

        // --- Layer 5: Escape‑lane layer (lateral sectors, low hazard) ---
        let mut escape_layer = HeatLayer::new(w, h);
        for (i, p) in packets.iter().enumerate() {
            let x = i % w;
            let y = (i / w).min(h - 1);
            let v = (1.0_f32 - p.escape_bias).clamp(0.0_f32, 1.0_f32);
            escape_layer.set(x, y, v);
        }
        self.heatmap.add_layer(escape_layer);
    }
}

