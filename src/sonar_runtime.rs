use crate::sonar_router::{SonarRouter, SonarRouterOutput};
use crate::sonar_fusion::{SonarFusion, FusionSensors, FusionState};
use crate::device::SonarDevice;

/// Runtime state for sonar processing.
#[derive(Debug, Clone)]
pub struct SonarRuntimeState {
    pub last_output: Option<SonarRouterOutput>,
    pub last_fusion: Option<FusionState>,
    pub frame: u64,
    pub time: f64,
}

/// A standalone runtime loop for the sonar system.
/// Now upgraded with multi‑sensor fusion.
pub struct SonarRuntime<D: SonarDevice> {
    router: SonarRouter<D>,
    fusion: SonarFusion,
    external_sensors: FusionSensors,
    state: SonarRuntimeState,
    tick_rate: f64, // frames per second
}

impl<D: SonarDevice> SonarRuntime<D> {
    /// Create a new sonar runtime.
    pub fn new(router: SonarRouter<D>, tick_rate: f64) -> Self {
        Self {
            router,
            fusion: SonarFusion::new(),
            external_sensors: FusionSensors {
                vision_obstacle_confidence: 0.0,
                lidar_obstacle_confidence: 0.0,
                radar_obstacle_confidence: 0.0,
                nearest_distance_m: None,
            },
            state: SonarRuntimeState {
                last_output: None,
                last_fusion: None,
                frame: 0,
                time: 0.0,
            },
            tick_rate: tick_rate.max(1.0),
        }
    }

    /// Update external sensor inputs (vision, LiDAR, radar).
    pub fn update_external_sensors(&mut self, sensors: FusionSensors) {
        self.external_sensors = sensors;
    }

    /// Run one frame of the sonar system.
    pub fn tick(&mut self) {
        let dt = 1.0 / self.tick_rate;
        self.state.time += dt;
        self.state.frame += 1;

        // --- 1. Sonar routing ---
        let output = self.router.route(self.state.time);
        self.state.last_output = Some(output.clone());

        // --- 2. Multi‑sensor fusion ---
        let fused = self.fusion.fuse(
            self.router.hazard_map(),
            &self.external_sensors,
        );
        self.state.last_fusion = Some(fused);
    }

    /// Run multiple frames.
    pub fn run_frames(&mut self, frames: u64) {
        for _ in 0..frames {
            self.tick();
        }
    }

    /// Latest sonar output.
    pub fn latest(&self) -> Option<&SonarRouterOutput> {
        self.state.last_output.as_ref()
    }

    /// Latest fused multi‑sensor state.
    pub fn latest_fusion(&self) -> Option<&FusionState> {
        self.state.last_fusion.as_ref()
    }

    /// Current frame number.
    pub fn frame(&self) -> u64 {
        self.state.frame
    }

    /// Current runtime time.
    pub fn time(&self) -> f64 {
        self.state.time
    }

    /// Access the router (for hazard map, etc.)
    pub fn router(&self) -> &SonarRouter<D> {
        &self.router
    }

    /// Mutable access to router (advanced use)
    pub fn router_mut(&mut self) -> &mut SonarRouter<D> {
        &mut self.router
    }
}
