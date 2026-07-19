use crate::hazard_map::HazardMap;
use crate::semantic_layer::SemanticLabel;
use crate::reflex_pipeline::ReflexAction;

/// A simple ASCII hazard visualizer.
/// This is intentionally lightweight and standalone.
/// Later, you can replace this with a GPU or GUI visualizer.
pub struct HazardVisualizer {
    /// Character set for intensity shading
    shades: Vec<char>,
}

impl HazardVisualizer {
    pub fn new() -> Self {
        // From low → high intensity
        let shades = vec![' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
        Self { shades }
    }

    /// Convert intensity (0.0–1.0) into a shaded ASCII character.
    fn shade(&self, intensity: f32) -> char {
        let idx = (intensity.clamp(0.0, 1.0) * (self.shades.len() as f32 - 1.0)) as usize;
        self.shades[idx]
    }

    /// Convert semantic label into a single overlay character.
    fn semantic_char(&self, label: &Option<SemanticLabel>) -> Option<char> {
        match label {
            Some(SemanticLabel::TransparentObject) => Some('T'),
            Some(SemanticLabel::SoftContact) => Some('S'),
            Some(SemanticLabel::HighRiskZone) => Some('H'),
            Some(SemanticLabel::MotionFlowHazard) => Some('M'),
            Some(SemanticLabel::DirectionalHazard) => Some('D'),
            Some(SemanticLabel::PersistentObstacle) => Some('P'),
            Some(SemanticLabel::NovelPattern) => Some('N'),
            Some(SemanticLabel::Unknown) => Some('?'),
            None => None,
        }
    }

    /// Convert reflex action into a single overlay character.
    fn reflex_char(&self, action: &Option<ReflexAction>) -> Option<char> {
        match action {
            Some(ReflexAction::EmergencyStop) => Some('!'),
            Some(ReflexAction::SlowDown) => Some('v'),
            Some(ReflexAction::SteerAway { .. }) => Some('<'),
            Some(ReflexAction::MarkHazard) => Some('X'),
            Some(ReflexAction::None) => None,
            None => None,
        }
    }

    /// Render the hazard map as ASCII.
    /// Priority:
    /// 1. Reflex overlay
    /// 2. Semantic overlay
    /// 3. Intensity shading
    pub fn render_ascii(&self, map: &HazardMap) -> String {
        let mut out = String::new();

        for y in 0..map.height {
            for x in 0..map.width {
                let cell = &map.cells[y * map.width + x];

                // Reflex overlay takes highest priority
                if let Some(rc) = self.reflex_char(&cell.last_action) {
                    out.push(rc);
                    continue;
                }

                // Semantic overlay next
                if let Some(sc) = self.semantic_char(&cell.label) {
                    out.push(sc);
                    continue;
                }

                // Otherwise shade by intensity
                out.push(self.shade(cell.intensity));
            }
            out.push('\n');
        }

        out
    }

    /// Print directly to stdout.
    pub fn print(&self, map: &HazardMap) {
        let ascii = self.render_ascii(map);
        println!("{}", ascii);
    }
}
