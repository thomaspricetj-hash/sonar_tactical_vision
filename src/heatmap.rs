use serde::{Serialize, Deserialize};

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
        let w = current.width;
        let h = current.height;

        for y in 0..h {
            for x in 0..w {
                let center = current.get(x, y);

                // Compute gradient in X
                let left = if x > 0 { current.get(x - 1, y) } else { center };
                let right = if x < w - 1 { current.get(x + 1, y) } else { center };
                let dx = right - left;

                // Compute gradient in Y
                let up = if y > 0 { current.get(x, y - 1) } else { center };
                let down = if y < h - 1 { current.get(x, y + 1) } else { center };
                let dy = down - up;

                self.motion_layer.set(x, y, dx, dy);
            }
        }
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
}

