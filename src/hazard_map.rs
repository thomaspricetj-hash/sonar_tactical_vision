use crate::heatmap::HeatLayer;
use crate::semantic_layer::{SemanticLabel, SemanticResult};
use crate::reflex_pipeline::ReflexAction;

/// A single hazard cell in the hazard map.
#[derive(Debug, Clone)]
pub struct HazardCell {
    /// Hazard intensity (0.0–1.0)
    pub intensity: f32,

    /// Optional semantic label (transparent object, soft contact, etc.)
    pub label: Option<SemanticLabel>,

    /// Last reflex action associated with this cell
    pub last_action: Option<ReflexAction>,

    /// NEW: temporal stability score (0.0–1.0)
    /// High stability = consistent hazard over time.
    pub stability: f32,

    /// NEW: multi‑layer fused precision contribution
    pub precision: f32,

    /// NEW: fractal complexity contribution (multi‑scale hazard texture)
    pub fractal: f32,
}

/// A 2D hazard map that accumulates risk over time.
/// Inspired by your SyntheticMind temporal + spatial memory,
/// but fully standalone.
#[derive(Debug, Clone)]
pub struct HazardMap {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<HazardCell>,

    /// Temporal decay factor (0.0–1.0)
    decay: f32,

    /// Reinforcement factor for new hazards
    reinforce: f32,

    /// NEW: multi‑layer hazard weighting (outer ring emphasis)
    pub outer_weight: f32,

    /// NEW: stability smoothing factor
    pub stability_blend: f32,
}

impl HazardMap {
    pub fn new(width: usize, height: usize, decay: f32, reinforce: f32) -> Self {
        let mut cells = Vec::new();
        cells.resize(
            width * height,
            HazardCell {
                intensity: 0.0,
                label: None,
                last_action: None,
                stability: 1.0,
                precision: 0.0,
                fractal: 0.0,
            },
        );

        Self {
            width,
            height,
            cells,
            decay: decay.clamp(0.0, 1.0),
            reinforce: reinforce.clamp(0.0, 2.0),
            outer_weight: 0.35,       // NEW: outer ring hazard emphasis
            stability_blend: 0.85,    // NEW: stability smoothing
        }
    }

    /// Apply temporal decay to all hazard cells.
    pub fn decay(&mut self) {
        for cell in &mut self.cells {
            cell.intensity *= self.decay;

            // NEW: stability decays slower than intensity
            cell.stability = (cell.stability * self.stability_blend).clamp(0.0, 1.0);

            if cell.intensity < 0.001 {
                cell.intensity = 0.0;
                cell.label = None;
                cell.last_action = None;
                cell.precision = 0.0;
                cell.fractal = 0.0;
            }
        }
    }

    /// Reinforce hazards using fused heatmap + semantic classification + reflex action.
    /// Now includes:
    /// - stability update
    /// - precision contribution
    /// - fractal complexity contribution
    /// - outer ring weighting
    pub fn reinforce(
        &mut self,
        fused: &HeatLayer,
        semantic: Option<&SemanticResult>,
        action: Option<&ReflexAction>,
        fused_precision: f32,
        fractal_precision: f32,
    ) {
        let w = self.width;
        let h = self.height;

        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let max_r = (cx * cx + cy * cy).sqrt().max(1.0);

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let fused_val = fused.get(x, y);

                if fused_val > 0.0 {
                    let cell = &mut self.cells[idx];

                    // --- Reinforce intensity ---
                    let mut reinforcement = fused_val * self.reinforce;

                    // NEW: outer ring weighting
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let r = (dx * dx + dy * dy).sqrt() / max_r;
                    if r > 0.66 {
                        reinforcement *= 1.0 + self.outer_weight;
                    }

                    cell.intensity = (cell.intensity + reinforcement).min(1.0);

                    // --- Semantic label ---
                    if let Some(sem) = semantic {
                        cell.label = Some(sem.label.clone());
                    }

                    // --- Reflex action ---
                    if let Some(act) = action {
                        cell.last_action = Some(act.clone());
                    }

                    // --- NEW: stability update ---
                    let new_stability = 1.0 - fused_val * 0.5;
                    cell.stability = (cell.stability * self.stability_blend
                        + new_stability * (1.0 - self.stability_blend))
                        .clamp(0.0, 1.0);

                    // --- NEW: precision contribution ---
                    cell.precision = (cell.precision + fused_precision * fused_val)
                        .clamp(0.0, 1.0);

                    // --- NEW: fractal complexity contribution ---
                    cell.fractal = (cell.fractal + fractal_precision * fused_val)
                        .clamp(0.0, 1.0);
                }
            }
        }
    }

    /// Get hazard intensity at a cell.
    pub fn get_intensity(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x].intensity
        } else {
            0.0
        }
    }

    /// Get semantic label at a cell.
    pub fn get_label(&self, x: usize, y: usize) -> Option<SemanticLabel> {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x].label.clone()
        } else {
            None
        }
    }

    /// Get last reflex action at a cell.
    pub fn get_last_action(&self, x: usize, y: usize) -> Option<ReflexAction> {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x].last_action.clone()
        } else {
            None
        }
    }

    /// NEW: Get stability score at a cell.
    pub fn get_stability(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x].stability
        } else {
            0.0
        }
    }

    /// NEW: Get fused precision contribution at a cell.
    pub fn get_precision(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x].precision
        } else {
            0.0
        }
    }

    /// NEW: Get fractal complexity contribution at a cell.
    pub fn get_fractal(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x].fractal
        } else {
            0.0
        }
    }

    /// Clear the entire hazard map.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.intensity = 0.0;
            cell.label = None;
            cell.last_action = None;
            cell.stability = 1.0;
            cell.precision = 0.0;
            cell.fractal = 0.0;
        }
    }
}
