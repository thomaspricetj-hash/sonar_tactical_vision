use crate::SonarDevice;

/// SonarArray
/// A multi‑sensor sonar array with angular mapping.
/// Each sensor corresponds to a specific angle around the robot.
pub struct SonarArray<D: SonarDevice> {
    /// All sonar sensors in the array.
    pub sensors: Vec<D>,

    /// Angle (in radians) for each sensor.
    /// Example: [0.0, 0.5, 1.0, 1.5] for a 4‑sensor ring.
    pub angles: Vec<f32>,
}

impl<D: SonarDevice> SonarArray<D> {
    /// Create a new sonar array.
    pub fn new(sensors: Vec<D>, angles: Vec<f32>) -> Self {
        assert!(
            sensors.len() == angles.len(),
            "Sensors and angles must have the same length"
        );

        Self { sensors, angles }
    }

    /// Number of sensors in the array.
    pub fn len(&self) -> usize {
        self.sensors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sensors.is_empty()
    }

    /// Ping all sensors and return a vector of distances.
    pub fn ping_all(&mut self) -> Vec<f32> {
        let mut readings = Vec::with_capacity(self.sensors.len());

        for sensor in &mut self.sensors {
            let d = sensor.ping();
            let d = d.clamp(sensor.min_range(), sensor.max_range());
            readings.push(d);
        }

        readings
    }

    /// Get the angle of a specific sensor.
    pub fn angle_of(&self, index: usize) -> Option<f32> {
        self.angles.get(index).copied()
    }
}
