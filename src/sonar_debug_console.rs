use crate::sonar_runtime::SonarRuntime;
use crate::sonar_router::SonarRouterOutput;
use crate::hazard_visualizer::HazardVisualizer;
use crate::device::SonarDevice;

/// A lightweight debugging console for the sonar system.
/// Prints events, semantic meaning, reflex actions, and hazard maps.
pub struct SonarDebugConsole {
    visualizer: HazardVisualizer,
    print_events: bool,
    print_signature: bool,
    print_semantic: bool,
    print_reflex: bool,
    print_hazard_map: bool,
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
