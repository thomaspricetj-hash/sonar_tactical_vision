use crate::SonarDevice;

/// Roundabout‑ready echo packet for each sensor.
#[derive(Debug, Clone)]
pub struct EchoPacket {
    pub angle_rad: f32,
    pub raw_distance: f32,
    pub clamped_distance: f32,
    pub stability_score: f32,
    pub angular_sector: usize, // 0=front,1=right,2=back,3=left
    pub escape_bias: f32,      // lower = safer escape lane
    pub forward_pressure: f32, // higher = more hazard forward
}

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

    /// Ping all sensors and return raw distances.
    pub fn ping_all(&mut self) -> Vec<f32> {
        let mut readings = Vec::with_capacity(self.sensors.len());

        for sensor in &mut self.sensors {
            let d = sensor.ping();
            let d = d.clamp(sensor.min_range(), sensor.max_range());
            readings.push(d);
        }

        readings
    }

    /// NEW: Ping all sensors and return roundabout‑ready echo packets.
    pub fn ping_roundabout(&mut self) -> Vec<EchoPacket> {
        let mut packets = Vec::with_capacity(self.sensors.len());

        for (i, sensor) in self.sensors.iter_mut().enumerate() {
            let raw = sensor.ping();
            let clamped = raw.clamp(sensor.min_range(), sensor.max_range());
            let angle = self.angles[i];

            // Angular sector mapping (front/right/back/left)
            let sector = Self::sector_from_angle(angle);

            // Stability score: closer readings = more stable
            let stability = 1.0 / (1.0 + clamped);

            // Escape bias: left/right sectors preferred for roundabout routing
            let escape_bias = match sector {
                1 | 3 => (clamped * 0.5).clamp(0.0, 1.0), // right/left safer
                _ => (clamped * 1.0).clamp(0.0, 1.0),     // front/back less safe
            };

            // Forward pressure: front sector increases hazard pressure
            let forward_pressure = if sector == 0 {
                (1.0 - stability).clamp(0.0, 1.0)
            } else {
                0.0
            };

            packets.push(EchoPacket {
                angle_rad: angle,
                raw_distance: raw,
                clamped_distance: clamped,
                stability_score: stability,
                angular_sector: sector,
                escape_bias,
                forward_pressure,
            });
        }

        packets
    }

    /// NEW: Convert angle to roundabout sector.
    /// 0 = front, 1 = right, 2 = back, 3 = left
    fn sector_from_angle(angle: f32) -> usize {
        let a = angle % (std::f32::consts::TAU);
        if a < std::f32::consts::FRAC_PI_4 || a > 7.0 * std::f32::consts::FRAC_PI_4 {
            0 // front
        } else if a < 3.0 * std::f32::consts::FRAC_PI_4 {
            1 // right
        } else if a < 5.0 * std::f32::consts::FRAC_PI_4 {
            2 // back
        } else {
            3 // left
        }
    }

    /// Get the angle of a specific sensor.
    pub fn angle_of(&self, index: usize) -> Option<f32> {
        self.angles.get(index).copied()
    }
}
