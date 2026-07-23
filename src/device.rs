/// SonarDevice
/// Hardware abstraction for any sonar sensor.
/// This allows the engine to work with real sensors or simulated ones.
pub trait SonarDevice {
    fn ping(&mut self) -> f32;
    fn min_range(&self) -> f32;
    fn max_range(&self) -> f32;

    fn stability(&self) -> f32 {
        1.0
    }

    fn confidence(&self) -> f32 {
        1.0
    }

    fn dropped(&self) -> bool {
        false
    }
}

/// Example simulated sonar device.
/// Useful for testing without hardware.
pub struct SimulatedSonar {
    pub min: f32,
    pub max: f32,
    pub simulated_distance: f32,

    pub noise: f32,
    pub dropout_prob: f32,

    pub last_stability: f32,
    pub last_confidence: f32,
    pub last_dropped: bool,

    /// NEW: deterministic pseudo‑random seed (no rand crate needed)
    seed: u32,
}

impl SimulatedSonar {
    pub fn new(min: f32, max: f32) -> Self {
        Self {
            min,
            max,
            simulated_distance: max,

            noise: 0.0,
            dropout_prob: 0.0,

            last_stability: 1.0,
            last_confidence: 1.0,
            last_dropped: false,

            seed: 0xA5A5_1234, // deterministic seed
        }
    }

    pub fn set_distance(&mut self, d: f32) {
        self.simulated_distance = d.clamp(self.min, self.max);
    }

    pub fn set_noise(&mut self, noise: f32) {
        self.noise = noise.clamp(0.0, 5.0);
    }

    pub fn set_dropout(&mut self, prob: f32) {
        self.dropout_prob = prob.clamp(0.0, 1.0);
    }

    /// Deterministic pseudo‑random generator (no rand crate)
    fn next_rand(&mut self) -> f32 {
        // xorshift32 — tiny, fast, deterministic
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;

        // convert to 0.0–1.0
        (self.seed as f32 / u32::MAX as f32).clamp(0.0, 1.0)
    }

    fn apply_noise(&mut self, v: f32) -> f32 {
        if self.noise <= 0.0 {
            return v;
        }

        let r = self.next_rand();          // 0.0–1.0
        let jitter = (r * 2.0 - 1.0) * self.noise; // -noise → +noise

        (v + jitter).clamp(self.min, self.max)
    }

    fn simulate_dropout(&mut self) -> bool {
        let r = self.next_rand();
        r < self.dropout_prob
    }

    fn compute_stability(&self, v: f32) -> f32 {
        let dist_norm = (v - self.min) / (self.max - self.min);
        (1.0 - dist_norm).clamp(0.0, 1.0)
    }

    fn compute_confidence(&self, stability: f32, dropped: bool) -> f32 {
        if dropped {
            return 0.0;
        }
        let noise_penalty = (self.noise / (self.max - self.min)).clamp(0.0, 1.0);
        (stability * (1.0 - noise_penalty)).clamp(0.0, 1.0)
    }
}

impl SonarDevice for SimulatedSonar {
    fn ping(&mut self) -> f32 {
        let dropped = self.simulate_dropout();
        self.last_dropped = dropped;

        if dropped {
            self.last_stability = 0.0;
            self.last_confidence = 0.0;
            return self.max;
        }

        let noisy = self.apply_noise(self.simulated_distance);
        let clamped = noisy.clamp(self.min, self.max);

        let stability = self.compute_stability(clamped);
        let confidence = self.compute_confidence(stability, dropped);

        self.last_stability = stability;
        self.last_confidence = confidence;

        clamped
    }

    fn min_range(&self) -> f32 {
        self.min
    }

    fn max_range(&self) -> f32 {
        self.max
    }

    fn stability(&self) -> f32 {
        self.last_stability
    }

    fn confidence(&self) -> f32 {
        self.last_confidence
    }

    fn dropped(&self) -> bool {
        self.last_dropped
    }
}

