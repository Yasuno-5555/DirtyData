//! Destruction and Distortion Civilization

#[derive(Clone)]
pub struct BitCrush {
    acc: f32,
    count: f32,
}

impl BitCrush {
    pub fn new() -> Self {
        Self {
            acc: 0.0,
            count: 0.0,
        }
    }
    pub fn process(&mut self, input: f32, bits: f32, sr_div: f32) -> f32 {
        // Bit reduction
        let levels = 2.0f32.powf(bits.clamp(1.0, 24.0));
        let out = (input * levels).round() / levels;

        // Sample rate reduction
        self.count += 1.0;
        if self.count >= sr_div.max(1.0) {
            self.acc = out;
            self.count = 0.0;
        }
        self.acc
    }
}

#[derive(Clone)]
pub struct WaveShaper {}

impl WaveShaper {
    pub fn process(&self, input: f32, amount: f32) -> f32 {
        // Hard clipping with tanh flavoring
        (input * (1.0 + amount)).tanh()
    }
}

#[derive(Clone)]
pub struct Pll {
    phase: f32,
    vco_freq: f32,
}

impl Pll {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            vco_freq: 100.0,
        }
    }
    pub fn process(&mut self, input: f32, lock_speed: f32, sample_rate: f32) -> f32 {
        let vco_out = self.phase.sin();
        let error = input * vco_out; // Phase detector

        // Update frequency based on error
        self.vco_freq += error * lock_speed;
        self.vco_freq = self.vco_freq.clamp(20.0, 5000.0);

        self.phase += 2.0 * std::f32::consts::PI * self.vco_freq / sample_rate;
        if self.phase > std::f32::consts::PI * 2.0 {
            self.phase -= std::f32::consts::PI * 2.0;
        }

        vco_out
    }
}
