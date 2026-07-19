/// SonarDevice
/// Hardware abstraction for any sonar sensor.
/// This allows the engine to work with real sensors or simulated ones.
pub trait SonarDevice {
    /// Perform a sonar ping and return the measured distance in meters.
    /// If the sensor fails or returns invalid data, implementations should
    /// clamp or sanitize the value rather than panic.
    fn ping(&mut self) -> f32;

    /// Minimum measurable distance for this sensor (meters).
    fn min_range(&self) -> f32;

    /// Maximum measurable distance for this sensor (meters).
    fn max_range(&self) -> f32;
}

/// Example simulated sonar device.
/// Useful for testing without hardware.
pub struct SimulatedSonar {
    pub min: f32,
    pub max: f32,
    pub simulated_distance: f32,
}

impl SimulatedSonar {
    pub fn new(min: f32, max: f32) -> Self {
        Self {
            min,
            max,
            simulated_distance: max,
        }
    }

    /// Set the next simulated reading.
    pub fn set_distance(&mut self, d: f32) {
        self.simulated_distance = d.clamp(self.min, self.max);
    }
}

impl SonarDevice for SimulatedSonar {
    fn ping(&mut self) -> f32 {
        self.simulated_distance.clamp(self.min, self.max)
    }

    fn min_range(&self) -> f32 {
        self.min
    }

    fn max_range(&self) -> f32 {
        self.max
    }
}
