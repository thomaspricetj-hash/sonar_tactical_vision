use crate::sonar_runtime::SonarRuntime;
use crate::sonar_router::SonarRouterOutput;
use crate::hazard_visualizer::HazardVisualizer;
use crate::device::SonarDevice;

/// A lightweight debugging console for the sonar system.
/// Upgraded for MAX‑tier debugging:
/// - cleaner formatting
/// - multi‑layer hazard metadata
/// - cross‑section summary
/// - precision + roundabout routing visibility
/// - stable output ordering
pub struct SonarDebugConsole {
    visualizer: HazardVisualizer,
    print_events: bool,
    print_signature: bool,
    print_semantic: bool,
    print_reflex: bool,
    print_hazard_map: bool,
    print_cross_sections: bool,     // NEW
}

impl SonarDebugConsole {
    pub fn new() -> Self {
        Self {
            visualizer: HazardVisualizer::new(),
            print_events: true,
            print_signature: true,
            print_semantic: true,
            print_reflex: true,
            print_hazard_map: true,
            print_cross_sections: true,
        }
    }

    pub fn disable_events(&mut self) {
        self.print_events = false;
    }
    pub fn disable_signature(&mut self) {
        self.print_signature = false;
    }
    pub fn disable_semantic(&mut self) {
        self.print_semantic = false;
    }
    pub fn disable_reflex(&mut self) {
        self.print_reflex = false;
    }
    pub fn disable_hazard_map(&mut self) {
        self.print_hazard_map = false;
    }
    pub fn disable_cross_sections(&mut self) {
        self.print_cross_sections = false;
    }

    /// Print the latest sonar output from the runtime.
    pub fn print_latest<D: SonarDevice>(&self, runtime: &SonarRuntime<D>) {
        if let Some(output) = runtime.latest() {
            self.print_output(output, runtime);
        } else {
            println!("No sonar output yet.");
        }
    }

    /// Print a full debug frame.
    fn print_output<D: SonarDevice>(
        &self,
        output: &SonarRouterOutput,
        runtime: &SonarRuntime<D>,
    ) {
        println!("================ SONAR DEBUG FRAME ================");
        println!("Frame: {}", runtime.frame());
        println!("Time:  {:.3} sec", runtime.time());
        println!("---------------------------------------------------");

        // --- Events ---
        if self.print_events {
            println!("Events:");
            if output.events.is_empty() {
                println!("  (none)");
            } else {
                for evt in &output.events {
                    println!("  {:?}", evt);
                }
            }
            println!("---------------------------------------------------");
        }

        // --- Signature ---
        if self.print_signature {
            println!("Signature:");
            println!("  Tag:        {}", output.signature.tag);
            println!("  Confidence: {:.3}", output.signature.confidence);
            println!(
                "  Label:      {}",
                output.signature.label.clone().unwrap_or_else(|| "None".into())
            );
            println!("---------------------------------------------------");
        }

        // --- Semantic ---
        if self.print_semantic {
            println!("Semantic:");
            println!("  Label:      {:?}", output.semantic.label);
            println!("  Confidence: {:.3}", output.semantic.confidence);
            println!("---------------------------------------------------");
        }

        // --- Reflex ---
        if self.print_reflex {
            println!("Reflex:");
            println!("  {:?}", output.reflex);
            println!("---------------------------------------------------");
        }

        // --- Cross‑Sections (NEW) ---
        if self.print_cross_sections {
            let cs = &output.cross_sections;
            println!("Cross‑Sections:");
            println!("  Entropy:            {:.3}", cs.entropy);
            println!("  Volatility:         {:.3}", cs.volatility);
            println!("  Drift:              {:.3}", cs.drift);
            println!("  Hazard:             {:.3}", cs.hazard);
            println!("  Semantic Weight:    {:.3}", cs.semantic_weight);
            println!("  Fused Precision:    {:.3}", cs.fused_precision);
            println!("  Motion Drift:       dx={:.3}, dy={:.3}", cs.motion_dx, cs.motion_dy);
            println!("  Temporal Stability: {:.3}", cs.temporal_stability);
            println!("  Front/Back:         {:.3} / {:.3}", cs.front_intensity, cs.back_intensity);
            println!("  Left/Right:         {:.3} / {:.3}", cs.left_intensity, cs.right_intensity);
            println!("  Roundabout Score:   {:.3}", cs.roundabout_score);
            println!("  Exit Bias (deg):    {:.1}", cs.exit_bias_deg);
            println!("---------------------------------------------------");
        }

        // --- Hazard Map ---
        if self.print_hazard_map {
            println!("Hazard Map:");
            let map = runtime.router().hazard_map();
            let ascii = self.visualizer.render_ascii(map);
            print!("{}", ascii);
            println!("---------------------------------------------------");
        }

        println!("===================================================\n");
    }
}
