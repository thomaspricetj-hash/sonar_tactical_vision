//! sonar_tactical_vision
//! Synthetic sonar‑tactile vision engine for close‑range perception,
//! collision avoidance, and tactile‑style sensing.

pub mod device;
pub mod array;
pub mod field;
pub mod events;
pub mod engine;
pub mod heatmap;
pub mod sonar_deep_store;
pub mod heatmap_compression;
pub mod novelty;
pub mod semantic_layer;
pub mod reflex_pipeline;
pub mod hazard_map;
pub mod sonar_router;
pub mod sonar_runtime;
pub mod hazard_visualizer;
pub mod sonar_debug_console;
pub mod sonar_cli;
pub mod sonar_fusion;

// OPTIONAL MEMORY‑INSPIRED MODULES (standalone)
pub mod sonar_signature;

pub use device::SonarDevice;
pub use array::SonarArray;
pub use field::TacticalField;
pub use events::TacticalEvent;
pub use engine::TacticalVisionEngine;
pub use heatmap::{HeatLayer, MultiLayerHeatmap};

// Export optional signature system
pub use sonar_signature::{SonarSignature, SonarSignatureExtractor};
pub use sonar_deep_store::{SonarDeepStore, SonarSnapshot};
pub use heatmap_compression::{HeatmapCompressor, BaselineHeatmapCompressor};
pub use novelty::{NoveltyDetector, BloomFilter};
pub use semantic_layer::{SemanticLayer, SemanticLabel, SemanticResult};
pub use reflex_pipeline::{ReflexPipeline, ReflexAction};
pub use hazard_map::{HazardMap, HazardCell};
pub use sonar_router::{SonarRouter, SonarRouterOutput};
pub use sonar_runtime::{SonarRuntime, SonarRuntimeState};
pub use hazard_visualizer::HazardVisualizer;
pub use sonar_debug_console::SonarDebugConsole;
pub use sonar_cli::SonarCli;
pub use sonar_fusion::{SonarFusion, FusionSensors, FusionState};