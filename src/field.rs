use serde::{Serialize, Deserialize};

/// TacticalField
/// Near‑range angular risk map used as the first‑layer substrate for:
/// - roundabout sonar packets
/// - multi‑layer heatmaps (raw, field, edge, temporal, predictive, motion)
/// - cross‑section slices (front/back/left/right/quadrants/rings)
/// - fused precision + fractal complexity
///
/// Each cell stores a normalized risk value:
/// 0.0 = no object detected
/// 1.0 = collision imminent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalField {
    /// Risk values for each angular cell.
    pub cells: Vec<f32>,

    /// Number of angular cells (resolution).
    pub resolution: usize,

    /// Maximum measurable radius (meters).
    pub radius: f32,

    /// NEW: angular width per cell (radians).
    /// Useful for roundabout sector mapping and multi‑layer indexing.
    pub cell_angle: f32,

    /// NEW: per‑cell stability score (0.0–1.0)
    /// Derived from temporal consistency and multi‑layer fused precision.
    pub stability: Vec<f32>,

    /// NEW: per‑cell hazard weight (0.0–1.0)
    /// Used to bias fused heatmap and directional hazard logic.
    pub hazard_weight: Vec<f32>,
}

impl TacticalField {
    /// Create a new empty tactical field.
    pub fn new(resolution: usize, radius: f32) -> Self {
        Self {
            cells: vec![0.0; resolution],
            resolution,
            radius,
            cell_angle: std::f32::consts::TAU / resolution as f32,
            stability: vec![1.0; resolution],
            hazard_weight: vec![0.0; resolution],
        }
    }

    /// Reset all risk values to zero.
    pub fn clear(&mut self) {
        for c in &mut self.cells {
            *c = 0.0;
        }
        for s in &mut self.stability {
            *s = 1.0;
        }
        for h in &mut self.hazard_weight {
            *h = 0.0;
        }
    }

    /// Update a specific angular cell with a sonar distance.
    /// Converts distance → risk using inverse‑distance model.
    /// Adds roundabout stability + hazard weighting hooks.
    pub fn update_cell(&mut self, index: usize, distance: f32) {
        if index >= self.resolution {
            return;
        }

        // Base risk from distance
        let risk = if distance <= 0.0 {
            1.0
        } else {
            let normalized = 1.0 - (distance / self.radius);
            normalized.clamp(0.0, 1.0)
        };

        self.cells[index] = risk;

        // NEW: stability update (temporal smoothing)
        // Higher risk → lower stability (more volatile)
        let stability = 1.0 - (risk * 0.5);
        self.stability[index] = stability.clamp(0.0, 1.0);

        // NEW: hazard weight (outer ring + high risk)
        self.hazard_weight[index] = (risk * 0.8).clamp(0.0, 1.0);
    }

    /// Returns the highest risk value in the field.
    pub fn max_risk(&self) -> f32 {
        self.cells
            .iter()
            .copied()
            .fold(0.0_f32, |a, b| if b > a { b } else { a })
    }

    /// Returns true if any cell indicates imminent collision.
    pub fn collision_imminent(&self, threshold: f32) -> bool {
        self.cells.iter().any(|&r| r >= threshold)
    }

    /// NEW: Returns the angular index of the highest risk.
    pub fn peak_index(&self) -> usize {
        let mut max_i = 0;
        let mut max_v = 0.0_f32;
        for (i, v) in self.cells.iter().enumerate() {
            if *v > max_v {
                max_v = *v;
                max_i = i;
            }
        }
        max_i
    }

    /// NEW: Returns the angle (radians) of the highest risk.
    pub fn peak_angle(&self) -> f32 {
        self.peak_index() as f32 * self.cell_angle
    }

    /// NEW: Returns a weighted risk average using stability + hazard weight.
    pub fn weighted_risk(&self) -> f32 {
        let mut sum = 0.0_f32;
        let mut weight_sum = 0.0_f32;

        for i in 0..self.resolution {
            let w = (self.stability[i] * 0.5) + (self.hazard_weight[i] * 0.5);
            sum += self.cells[i] * w;
            weight_sum += w;
        }

        if weight_sum > 0.0 {
            (sum / weight_sum).clamp(0.0_f32, 1.0_f32)
        } else {
            0.0_f32
        }
    }

    /// NEW: Returns a simple multi‑layer index summary for this field.
    pub fn summary(&self) -> TacticalFieldSummary {
        TacticalFieldSummary {
            max_risk: self.max_risk(),
            weighted_risk: self.weighted_risk(),
            peak_index: self.peak_index(),
            peak_angle: self.peak_angle(),
            avg_stability: self.stability.iter().copied().sum::<f32>() / self.resolution as f32,
            avg_hazard_weight: self.hazard_weight.iter().copied().sum::<f32>() / self.resolution as f32,
        }
    }
}

/// NEW: Summary struct for multi‑layer field diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalFieldSummary {
    pub max_risk: f32,
    pub weighted_risk: f32,
    pub peak_index: usize,
    pub peak_angle: f32,
    pub avg_stability: f32,
    pub avg_hazard_weight: f32,
}
