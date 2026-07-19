use serde::{Serialize, Deserialize};

/// TacticalField
/// A near‑range spatial risk map generated from sonar readings.
/// This acts like a synthetic tactile‑vision layer around the robot.
///
/// Each cell represents a normalized "risk" value:
/// 0.0 = no object detected
/// 1.0 = collision imminent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalField {
    /// Risk values for each cell in the near‑field map.
    pub cells: Vec<f32>,

    /// Number of cells in the map (angular resolution).
    pub resolution: usize,

    /// Maximum radius (meters) this field represents.
    pub radius: f32,
}

impl TacticalField {
    /// Create a new empty tactical field.
    pub fn new(resolution: usize, radius: f32) -> Self {
        Self {
            cells: vec![0.0; resolution],
            resolution,
            radius,
        }
    }

    /// Reset all risk values to zero.
    pub fn clear(&mut self) {
        for c in &mut self.cells {
            *c = 0.0;
        }
    }

    /// Update a specific angular cell with a sonar distance.
    /// Converts distance → risk using a simple inverse‑distance model.
    pub fn update_cell(&mut self, index: usize, distance: f32) {
        if index >= self.resolution {
            return;
        }

        // Normalize distance into a risk value.
        // Closer = higher risk.
        let risk = if distance <= 0.0 {
            1.0
        } else {
            let normalized = 1.0 - (distance / self.radius);
            normalized.clamp(0.0, 1.0)
        };

        self.cells[index] = risk;
    }

    /// Returns the highest risk value in the field.
    pub fn max_risk(&self) -> f32 {
        self.cells
            .iter()
            .copied()
            .fold(0.0, |a, b| if b > a { b } else { a })
    }

    /// Returns true if any cell indicates imminent collision.
    pub fn collision_imminent(&self, threshold: f32) -> bool {
        self.cells.iter().any(|&r| r >= threshold)
    }
}
