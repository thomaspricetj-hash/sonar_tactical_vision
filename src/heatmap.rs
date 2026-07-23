use serde::{Serialize, Deserialize};
use crate::hazard_map::HazardMap;

/// Full cross‑section mapping results: spatial, temporal, motion, hazard, multi‑layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullCrossSectionSlices {
    // Core metrics
    pub entropy: f32,
    pub volatility: f32,
    pub drift: f32,

    // Motion‑vector drift
    pub motion_dx: f32,
    pub motion_dy: f32,

    // Temporal stability
    pub temporal_stability: f32,

    // Spatial slices
    pub front_intensity: f32,
    pub back_intensity: f32,
    pub left_intensity: f32,
    pub right_intensity: f32,

    // Quadrants
    pub q1_intensity: f32,
    pub q2_intensity: f32,
    pub q3_intensity: f32,
    pub q4_intensity: f32,

    // Radial rings
    pub inner_ring: f32,
    pub mid_ring: f32,
    pub outer_ring: f32,

    // Hazard slices
    pub hazard_front: f32,
    pub hazard_back: f32,
    pub hazard_left: f32,
    pub hazard_right: f32,

    pub hazard_q1: f32,
    pub hazard_q2: f32,
    pub hazard_q3: f32,
    pub hazard_q4: f32,

    pub hazard_inner: f32,
    pub hazard_mid: f32,
    pub hazard_outer: f32,

    // NEW: fractal multi‑scale complexity score
    pub fractal_precision: f32,

    // Fused precision score
    pub fused_precision: f32,
}

/// Roundabout multi‑layer index: compact routing metrics derived from cross‑sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundaboutIndex {
    pub roundabout_score: f32,
    pub exit_bias_deg: f32,
    pub lateral_escape_score: f32,
    pub forward_pressure_score: f32,
    pub stability_score: f32,
    pub hazard_pressure_score: f32,
}

/// Roundabout graph node: represents a spatial/hazard region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundaboutGraphNode {
    pub id: usize,
    pub label: String,
    pub weight: f32,
}

/// Roundabout graph: multi‑layer connectivity between regions (front/back/left/right + rings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundaboutGraph {
    pub nodes: Vec<RoundaboutGraphNode>,
    pub edges: Vec<(usize, usize, f32)>, // (from, to, weight)
}

impl FullCrossSectionSlices {
    /// Build a roundabout index from cross‑section slices.
    pub fn to_roundabout_index(&self) -> RoundaboutIndex {
        // Lateral escape: low hazard + low intensity on left/right
        let lateral_intensity = (self.left_intensity + self.right_intensity) * 0.5;
        let lateral_hazard = (self.hazard_left + self.hazard_right) * 0.5;
        let lateral_escape_score = (1.0 - lateral_intensity.clamp(0.0, 1.0))
            * (1.0 - lateral_hazard.clamp(0.0, 1.0));

        // Forward pressure: high front intensity + high front hazard
        let forward_pressure_score = (self.front_intensity.clamp(0.0, 1.0)
            * (0.5 + self.hazard_front.clamp(0.0, 1.0) * 0.5))
            .clamp(0.0, 1.0);

        // Hazard pressure: outer ring + front hazard
        let hazard_pressure_score = ((self.hazard_outer + self.hazard_front) * 0.5).clamp(0.0, 1.0);

        // Stability: temporal + fractal precision
        let stability_score = (self.temporal_stability * (0.5 + self.fractal_precision * 0.5))
            .clamp(0.0, 1.0);

        // Roundabout score: prefer high stability, low hazard, strong lateral escape, low forward pressure
        let roundabout_score = stability_score
            * (1.0 - hazard_pressure_score)
            * (0.5 + lateral_escape_score * 0.5)
            * (1.0 - forward_pressure_score)
            * (0.5 + self.fused_precision * 0.5);

        // Exit bias: steer toward safer side
        let lateral_diff = self.right_intensity - self.left_intensity;
        let forward_diff = self.front_intensity - self.back_intensity;

        let mut exit_bias_deg;

        if forward_diff > 0.2 && self.hazard_front > 0.3 {
            // front is hot → prefer lateral escape
            if lateral_diff > 0.05 {
                exit_bias_deg = 45.0; // steer right
            } else if lateral_diff < -0.05 {
                exit_bias_deg = -45.0; // steer left
            } else {
                exit_bias_deg = 90.0; // hard turn, choose side later
            }
        } else {
            exit_bias_deg = lateral_diff * 90.0;
        }

        RoundaboutIndex {
            roundabout_score: roundabout_score.clamp(0.0, 1.0),
            exit_bias_deg: exit_bias_deg.clamp(-90.0, 90.0),
            lateral_escape_score: lateral_escape_score.clamp(0.0, 1.0),
            forward_pressure_score: forward_pressure_score.clamp(0.0, 1.0),
            stability_score,
            hazard_pressure_score,
        }
    }

    /// Build a roundabout graph from cross‑section slices.
    pub fn to_roundabout_graph(&self) -> RoundaboutGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Nodes: front/back/left/right + rings
        nodes.push(RoundaboutGraphNode {
            id: 0,
            label: "front".to_string(),
            weight: self.front_intensity,
        });
        nodes.push(RoundaboutGraphNode {
            id: 1,
            label: "back".to_string(),
            weight: self.back_intensity,
        });
        nodes.push(RoundaboutGraphNode {
            id: 2,
            label: "left".to_string(),
            weight: self.left_intensity,
        });
        nodes.push(RoundaboutGraphNode {
            id: 3,
            label: "right".to_string(),
            weight: self.right_intensity,
        });
        nodes.push(RoundaboutGraphNode {
            id: 4,
            label: "inner_ring".to_string(),
            weight: self.inner_ring,
        });
        nodes.push(RoundaboutGraphNode {
            id: 5,
            label: "mid_ring".to_string(),
            weight: self.mid_ring,
        });
        nodes.push(RoundaboutGraphNode {
            id: 6,
            label: "outer_ring".to_string(),
            weight: self.outer_ring,
        });

        // Edges: basic roundabout connectivity
        // front ↔ left/right
        edges.push((0, 2, 1.0 - self.hazard_left.clamp(0.0, 1.0)));
        edges.push((0, 3, 1.0 - self.hazard_right.clamp(0.0, 1.0)));

        // left ↔ inner/mid
        edges.push((2, 4, 1.0 - self.hazard_inner.clamp(0.0, 1.0)));
        edges.push((2, 5, 1.0 - self.hazard_mid.clamp(0.0, 1.0)));

        // right ↔ inner/mid
        edges.push((3, 4, 1.0 - self.hazard_inner.clamp(0.0, 1.0)));
        edges.push((3, 5, 1.0 - self.hazard_mid.clamp(0.0, 1.0)));

        // front ↔ outer
        edges.push((0, 6, 1.0 - self.hazard_outer.clamp(0.0, 1.0)));

        // back ↔ inner/mid
        edges.push((1, 4, 1.0 - self.hazard_inner.clamp(0.0, 1.0)));
        edges.push((1, 5, 1.0 - self.hazard_mid.clamp(0.0, 1.0)));

        RoundaboutGraph { nodes, edges }
    }
}

/// A single heatmap layer represented as a dot‑matrix grid.
/// Each cell stores a normalized risk value (0.0–1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatLayer {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<f32>,
}

impl HeatLayer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![0.0; width * height],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = value.clamp(0.0, 1.0);
        }
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x]
        } else {
            0.0
        }
    }

    /// Apply a simple smoothing pass (3×3 kernel).
    pub fn smooth(&mut self) {
        let mut new_cells = self.cells.clone();

        for y in 0..self.height {
            for x in 0..self.width {
                let mut sum = 0.0;
                let mut count = 0;

                for dy in [-1, 0, 1] {
                    for dx in [-1, 0, 1] {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;

                        if nx >= 0 && ny >= 0 &&
                           nx < self.width as isize &&
                           ny < self.height as isize
                        {
                            sum += self.get(nx as usize, ny as usize);
                            count += 1;
                        }
                    }
                }

                new_cells[y * self.width + x] = (sum / count as f32).clamp(0.0, 1.0);
            }
        }

        self.cells = new_cells;
    }

    /// Decay all cells by a factor (temporal fading).
    pub fn decay(&mut self, factor: f32) {
        for v in &mut self.cells {
            *v = (*v * factor).clamp(0.0, 1.0);
        }
    }

    /// Add another layer into this one (accumulation).
    pub fn accumulate(&mut self, other: &HeatLayer, weight: f32) {
        if self.width != other.width || self.height != other.height {
            return;
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let v = self.cells[idx] + other.cells[idx] * weight;
                self.cells[idx] = v.clamp(0.0, 1.0);
            }
        }
    }

    /// NEW: Fractal multi‑scale complexity analysis.
    fn compute_fractal_precision(&self) -> f32 {
        let w = self.width;
        let h = self.height;

        // Multi‑scale windows: 1×1, 3×3, 5×5
        let scales = [1, 3, 5];
        let mut total_complexity = 0.0;
        let mut samples = 0;

        for &s in &scales {
            let radius = s / 2;

            if radius == 0 {
                // 1×1 scale: just use raw cell variance across the map
                let mean = self.cells.iter().sum::<f32>() / self.cells.len().max(1) as f32;
                let var = self.cells.iter().map(|v| (v - mean).abs()).sum::<f32>()
                    / self.cells.len().max(1) as f32;
                total_complexity += var;
                samples += 1;
                continue;
            }

            for y in radius..(h - radius) {
                for x in radius..(w - radius) {
                    let mut local_vals = Vec::new();

                    for dy in -(radius as isize)..=(radius as isize) {
                        for dx in -(radius as isize)..=(radius as isize) {
                            let nx = (x as isize + dx) as usize;
                            let ny = (y as isize + dy) as usize;
                            local_vals.push(self.get(nx, ny));
                        }
                    }

                    let mean = local_vals.iter().sum::<f32>() / local_vals.len() as f32;
                    let var = local_vals.iter().map(|v| (v - mean).abs()).sum::<f32>()
                        / local_vals.len() as f32;

                    total_complexity += var;
                    samples += 1;
                }
            }
        }

        if samples == 0 {
            return 0.0;
        }

        (total_complexity / samples as f32).clamp(0.0, 1.0)
    }

    /// Compute full cross‑section slices (multi‑layer + hazard‑aware).
    pub fn compute_full_cross_sections(
        &self,
        motion: Option<&MotionVectorLayer>,
        prev: Option<&[f32]>,
        hazard: Option<&HazardMap>,
    ) -> FullCrossSectionSlices {
        let w = self.width;
        let h = self.height;
        let fused = &self.cells;

        // --- entropy ---
        let entropy = fused.iter().map(|v| v.abs()).sum::<f32>() / fused.len().max(1) as f32;

        // --- volatility ---
        let mut volatility = 0.0;
        for win in fused.windows(2) {
            volatility += (win[1] - win[0]).abs();
        }
        volatility /= fused.len().max(1) as f32;

        // --- drift ---
        let drift = fused
            .iter()
            .enumerate()
            .map(|(i, v)| (i as f32 * 0.01) * v)
            .sum::<f32>();

        // --- motion‑vector drift ---
        let (motion_dx, motion_dy) = if let Some(m) = motion {
            let mut dx_sum = 0.0;
            let mut dy_sum = 0.0;
            let mut count = 0;

            for y in 0..h {
                for x in 0..w {
                    let (dx, dy) = m.get(x, y);
                    dx_sum += dx;
                    dy_sum += dy;
                    count += 1;
                }
            }

            if count == 0 {
                (0.0, 0.0)
            } else {
                (dx_sum / count as f32, dy_sum / count as f32)
            }
        } else {
            (0.0, 0.0)
        };

        // --- temporal stability ---
        let temporal_stability = if let Some(prev_cells) = prev {
            if prev_cells.len() == fused.len() {
                let diff = fused
                    .iter()
                    .zip(prev_cells.iter())
                    .map(|(a, b)| (a - b).abs())
                    .sum::<f32>()
                    / fused.len() as f32;
                1.0 / (1.0 + diff)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // --- spatial slices ---
        let mut front_sum = 0.0;
        let mut back_sum = 0.0;
        let mut left_sum = 0.0;
        let mut right_sum = 0.0;

        let mut front_count = 0;
        let mut back_count = 0;
        let mut left_count = 0;
        let mut right_count = 0;

        // Quadrants
        let mut q1 = 0.0;
        let mut q2 = 0.0;
        let mut q3 = 0.0;
        let mut q4 = 0.0;

        let mut q1c = 0;
        let mut q2c = 0;
        let mut q3c = 0;
        let mut q4c = 0;

        // Radial rings
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let max_r = (cx * cx + cy * cy).sqrt().max(1.0);

        let mut inner_sum = 0.0;
        let mut mid_sum = 0.0;
        let mut outer_sum = 0.0;

        let mut inner_count = 0;
        let mut mid_count = 0;
        let mut outer_count = 0;

        // Hazard slices
        let mut hz_front = 0.0;
        let mut hz_back = 0.0;
        let mut hz_left = 0.0;
        let mut hz_right = 0.0;

        let mut hz_q1 = 0.0;
        let mut hz_q2 = 0.0;
        let mut hz_q3 = 0.0;
        let mut hz_q4 = 0.0;

        let mut hz_inner = 0.0;
        let mut hz_mid = 0.0;
        let mut hz_outer = 0.0;

        let mut hz_front_c = 0;
        let mut hz_back_c = 0;
        let mut hz_left_c = 0;
        let mut hz_right_c = 0;

        let mut hz_q1c = 0;
        let mut hz_q2c = 0;
        let mut hz_q3c = 0;
        let mut hz_q4c = 0;

        let mut hz_inner_c = 0;
        let mut hz_mid_c = 0;
        let mut hz_outer_c = 0;

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let v = fused[idx];

                // front/back
                if y < h / 2 {
                    front_sum += v;
                    front_count += 1;
                } else {
                    back_sum += v;
                    back_count += 1;
                }

                // left/right
                if x < w / 2 {
                    left_sum += v;
                    left_count += 1;
                } else {
                    right_sum += v;
                    right_count += 1;
                }

                // quadrants
                if x < w / 2 && y < h / 2 {
                    q1 += v;
                    q1c += 1;
                } else if x >= w / 2 && y < h / 2 {
                    q2 += v;
                    q2c += 1;
                } else if x < w / 2 && y >= h / 2 {
                    q3 += v;
                    q3c += 1;
                } else {
                    q4 += v;
                    q4c += 1;
                }

                // radial rings
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let r = (dx * dx + dy * dy).sqrt() / max_r;

                if r < 0.33 {
                    inner_sum += v;
                    inner_count += 1;
                } else if r < 0.66 {
                    mid_sum += v;
                    mid_count += 1;
                } else {
                    outer_sum += v;
                    outer_count += 1;
                }

                // hazard slices
                if let Some(hz) = hazard {
                    let hv = hz.get_intensity(x, y);

                    if y < h / 2 {
                        hz_front += hv;
                        hz_front_c += 1;
                    } else {
                        hz_back += hv;
                        hz_back_c += 1;
                    }

                    if x < w / 2 {
                        hz_left += hv;
                        hz_left_c += 1;
                    } else {
                        hz_right += hv;
                        hz_right_c += 1;
                    }

                    if x < w / 2 && y < h / 2 {
                        hz_q1 += hv;
                        hz_q1c += 1;
                    } else if x >= w / 2 && y < h / 2 {
                        hz_q2 += hv;
                        hz_q2c += 1;
                    } else if x < w / 2 && y >= h / 2 {
                        hz_q3 += hv;
                        hz_q3c += 1;
                    } else {
                        hz_q4 += hv;
                        hz_q4c += 1;
                    }

                    if r < 0.33 {
                        hz_inner += hv;
                        hz_inner_c += 1;
                    } else if r < 0.66 {
                        hz_mid += hv;
                        hz_mid_c += 1;
                    } else {
                        hz_outer += hv;
                        hz_outer_c += 1;
                    }
                }
            }
        }

        // NEW: fractal multi‑scale complexity
        let fractal_precision = self.compute_fractal_precision();

        // fused precision score (adaptive: penalize high entropy/volatility/hazard)
        let fused_precision =
            (1.0 / (1.0 + entropy)) *
            (1.0 / (1.0 + volatility)) *
            temporal_stability *
            (1.0 - hz_outer.min(1.0)) *
            (0.5 + fractal_precision * 0.5);

        FullCrossSectionSlices {
            entropy,
            volatility,
            drift,
            motion_dx,
            motion_dy,
            temporal_stability,

            front_intensity: front_sum / front_count.max(1) as f32,
            back_intensity: back_sum / back_count.max(1) as f32,
            left_intensity: left_sum / left_count.max(1) as f32,
            right_intensity: right_sum / right_count.max(1) as f32,

            q1_intensity: q1 / q1c.max(1) as f32,
            q2_intensity: q2 / q2c.max(1) as f32,
            q3_intensity: q3 / q3c.max(1) as f32,
            q4_intensity: q4 / q4c.max(1) as f32,

            inner_ring: inner_sum / inner_count.max(1) as f32,
            mid_ring: mid_sum / mid_count.max(1) as f32,
            outer_ring: outer_sum / outer_count.max(1) as f32,

            hazard_front: hz_front / hz_front_c.max(1) as f32,
            hazard_back: hz_back / hz_back_c.max(1) as f32,
            hazard_left: hz_left / hz_left_c.max(1) as f32,
            hazard_right: hz_right / hz_right_c.max(1) as f32,

            hazard_q1: hz_q1 / hz_q1c.max(1) as f32,
            hazard_q2: hz_q2 / hz_q2c.max(1) as f32,
            hazard_q3: hz_q3 / hz_q3c.max(1) as f32,
            hazard_q4: hz_q4 / hz_q4c.max(1) as f32,

            hazard_inner: hz_inner / hz_inner_c.max(1) as f32,
            hazard_mid: hz_mid / hz_mid_c.max(1) as f32,
            hazard_outer: hz_outer / hz_outer_c.max(1) as f32,

            fractal_precision,
            fused_precision,
        }
    }
}

/// Motion vector layer: stores directional flow of risk.
/// Each cell stores a vector (dx, dy) encoded as two heat layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionVectorLayer {
    pub width: usize,
    pub height: usize,
    pub dx: Vec<f32>,
    pub dy: Vec<f32>,
}

impl MotionVectorLayer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            dx: vec![0.0; width * height],
            dy: vec![0.0; width * height],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, vx: f32, vy: f32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.dx[idx] = vx.clamp(-1.0, 1.0);
            self.dy[idx] = vy.clamp(-1.0, 1.0);
        }
    }

    pub fn get(&self, x: usize, y: usize) -> (f32, f32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            (self.dx[idx], self.dy[idx])
        } else {
            (0.0, 0.0)
        }
    }

    /// Smooth motion vectors by averaging neighbors.
    pub fn smooth(&mut self) {
        let mut new_dx = self.dx.clone();
        let mut new_dy = self.dy.clone();

        for y in 0..self.height {
            for x in 0..self.width {
                let mut sx = 0.0;
                let mut sy = 0.0;
                let mut count = 0;

                for dy in [-1, 0, 1] {
                    for dx in [-1, 0, 1] {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;

                        if nx >= 0 && ny >= 0 &&
                           nx < self.width as isize &&
                           ny < self.height as isize
                        {
                            let (vx, vy) = self.get(nx as usize, ny as usize);
                            sx += vx;
                            sy += vy;
                            count += 1;
                        }
                    }
                }

                let idx = y * self.width + x;
                new_dx[idx] = (sx / count as f32).clamp(-1.0, 1.0);
                new_dy[idx] = (sy / count as f32).clamp(-1.0, 1.0);
            }
        }

        self.dx = new_dx;
        self.dy = new_dy;
    }

    /// Update motion vectors from a heat layer (gradient field).
    pub fn update_from_heat(&mut self, current: &HeatLayer) {
        let w = current.width;
        let h = current.height;

        for y in 0..h {
            for x in 0..w {
                let center = current.get(x, y);

                let left = if x > 0 { current.get(x - 1, y) } else { center };
                let right = if x < w - 1 { current.get(x + 1, y) } else { center };
                let dx = right - left;

                let up = if y > 0 { current.get(x, y - 1) } else { center };
                let down = if y < h - 1 { current.get(x, y + 1) } else { center };
                let dy = down - up;

                self.set(x, y, dx, dy);
            }
        }
    }
}

/// Multi‑layer heatmap stack.
/// Includes temporal accumulation, predictive layer, and motion‑vector layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLayerHeatmap {
    pub layers: Vec<HeatLayer>,

    /// Temporal accumulation (memory heat).
    pub temporal_layer: HeatLayer,

    /// Predictive forward‑projected risk.
    pub predictive_layer: HeatLayer,

    /// Motion vector flow field.
    pub motion_layer: MotionVectorLayer,
}

impl MultiLayerHeatmap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            layers: Vec::new(),
            temporal_layer: HeatLayer::new(width, height),
            predictive_layer: HeatLayer::new(width, height),
            motion_layer: MotionVectorLayer::new(width, height),
        }
    }

    pub fn add_layer(&mut self, layer: HeatLayer) {
        self.layers.push(layer);
    }

    /// Smooth all layers (including temporal, predictive, and motion).
    pub fn smooth_all(&mut self) {
        for layer in &mut self.layers {
            layer.smooth();
        }
        self.temporal_layer.smooth();
        self.predictive_layer.smooth();
        self.motion_layer.smooth();
    }

    /// Update temporal accumulation:
    /// - decay old memory
    /// - accumulate current fused heat.
    pub fn update_temporal(&mut self, current: &HeatLayer, decay_factor: f32, weight: f32) {
        self.temporal_layer.decay(decay_factor);
        self.temporal_layer.accumulate(current, weight);
    }

    /// Update predictive layer by projecting current heat forward.
    pub fn update_predictive(&mut self, current: &HeatLayer) {
        let w = current.width;
        let h = current.height;
        self.predictive_layer = HeatLayer::new(w, h);

        for y in 0..h {
            for x in 0..w {
                let v = current.get(x, y);
                if v > 0.0 {
                    let nx = (x + 1).min(w - 1);
                    let ny = (y + 1).min(h - 1);
                    let projected = (v * 0.8).clamp(0.0, 1.0);
                    self.predictive_layer.set(nx, ny, projected);
                }
            }
        }
    }

    /// Update motion vector layer by computing directional risk flow.
    pub fn update_motion_vectors(&mut self, current: &HeatLayer) {
        self.motion_layer.update_from_heat(current);
    }

    /// Fuse all layers into a single composite heatmap.
    pub fn fuse(&self) -> HeatLayer {
        let w = self.temporal_layer.width;
        let h = self.temporal_layer.height;
        let mut fused = HeatLayer::new(w, h);

        for y in 0..h {
            for x in 0..w {
                let mut max_val = 0.0;

                // Raw + field + edge layers
                for layer in &self.layers {
                    let v = layer.get(x, y);
                    if v > max_val {
                        max_val = v;
                    }
                }

                // Temporal memory
                let t = self.temporal_layer.get(x, y);
                if t > max_val {
                    max_val = t;
                }

                // Predictive forward projection
                let p = self.predictive_layer.get(x, y);
                if p > max_val {
                    max_val = p;
                }

                // Motion vector magnitude
                let (vx, vy) = self.motion_layer.get(x, y);
                let motion_mag = (vx.abs() + vy.abs()).min(1.0);
                if motion_mag > max_val {
                    max_val = motion_mag;
                }

                fused.set(x, y, max_val);
            }
        }

        fused
    }

    /// Compute full cross‑sections from fused heatmap + motion + hazard.
    pub fn full_cross_sections(
        &self,
        prev_fused: Option<&[f32]>,
        hazard: Option<&HazardMap>,
    ) -> FullCrossSectionSlices {
        let fused = self.fuse();
        fused.compute_full_cross_sections(Some(&self.motion_layer), prev_fused, hazard)
    }

    /// Compute roundabout index from multi‑layer fused heatmap.
    pub fn roundabout_index(
        &self,
        prev_fused: Option<&[f32]>,
        hazard: Option<&HazardMap>,
    ) -> RoundaboutIndex {
        let slices = self.full_cross_sections(prev_fused, hazard);
        slices.to_roundabout_index()
    }

    /// Compute roundabout graph from multi‑layer fused heatmap.
    pub fn roundabout_graph(
        &self,
        prev_fused: Option<&[f32]>,
        hazard: Option<&HazardMap>,
    ) -> RoundaboutGraph {
        let slices = self.full_cross_sections(prev_fused, hazard);
        slices.to_roundabout_graph()
    }
}

