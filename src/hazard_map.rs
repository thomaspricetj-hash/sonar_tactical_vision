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
            },
        );

        Self {
            width,
            height,
            cells,
            decay: decay.clamp(0.0, 1.0),
            reinforce: reinforce.clamp(0.0, 2.0),
        }
    }

    /// Apply temporal decay to all hazard cells.
    pub fn decay(&mut self) {
        for cell in &mut self.cells {
            cell.intensity *= self.decay;
            if cell.intensity < 0.001 {
                cell.intensity = 0.0;
                cell.label = None;
                cell.last_action = None;
            }
        }
    }

    /// Reinforce hazards using fused heatmap + semantic classification + reflex action.
    pub fn reinforce(
        &mut self,
        fused: &HeatLayer,
        semantic: Option<&SemanticResult>,
        action: Option<&ReflexAction>,
    ) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let fused_val = fused.get(x, y);

                if fused_val > 0.0 {
                    let cell = &mut self.cells[idx];

                    // Reinforce intensity
                    cell.intensity = (cell.intensity + fused_val * self.reinforce).min(1.0);

                    // Apply semantic label if present
                    if let Some(sem) = semantic {
                        cell.label = Some(sem.label.clone());
                    }

                    // Apply reflex action if present
                    if let Some(act) = action {
                        cell.last_action = Some(act.clone());
                    }
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

    /// Clear the entire hazard map.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.intensity = 0.0;
            cell.label = None;
            cell.last_action = None;
        }
    }
}
