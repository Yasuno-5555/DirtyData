//! CV and Logic Civilization

#[derive(Clone)]
pub struct Slew {
    state: f32,
}

impl Slew {
    pub fn new() -> Self { Self { state: 0.0 } }
    pub fn process(&mut self, input: f32, rise: f32, fall: f32, sample_rate: f32) -> f32 {
        let diff = input - self.state;
        let rate = if diff > 0.0 { rise } else { fall };
        // rate is time in seconds to reach target
        let step = 1.0 / (rate.max(0.001) * sample_rate);
        if diff.abs() < step {
            self.state = input;
        } else {
            self.state += diff.signum() * step;
        }
        self.state
    }
}

#[derive(Clone)]
pub struct Comparator {
    state: bool,
}

impl Comparator {
    pub fn new() -> Self { Self { state: false } }
    pub fn process(&mut self, input: f32, threshold: f32, hysteresis: f32) -> f32 {
        if input > threshold + hysteresis {
            self.state = true;
        } else if input < threshold - hysteresis {
            self.state = false;
        }
        if self.state { 1.0 } else { 0.0 }
    }
}

#[derive(Clone)]
pub struct ClockDivider {
    count: u32,
    prev_clock: f32,
}

impl ClockDivider {
    pub fn new() -> Self { Self { count: 0, prev_clock: 0.0 } }
    pub fn process(&mut self, clock: f32, divisor: u32) -> f32 {
        if clock > 0.5 && self.prev_clock <= 0.5 {
            self.count += 1;
        }
        self.prev_clock = clock;
        if (self.count % divisor) == 0 && clock > 0.5 { 1.0 } else { 0.0 }
    }
}

#[derive(Clone)]
pub struct EuclideanSequencer {
    pub steps: u32,
    pub hits: u32,
    pub offset: u32,
    idx: u32,
    prev_clock: f32,
}

impl EuclideanSequencer {
    pub fn new() -> Self { Self { steps: 16, hits: 4, offset: 0, idx: 0, prev_clock: 0.0 } }
    pub fn process(&mut self, clock: f32) -> f32 {
        let mut trigger = 0.0;
        if clock > 0.5 && self.prev_clock <= 0.5 {
            let i = (self.idx + self.offset) % self.steps.max(1);
            if ((i * self.hits) % self.steps.max(1)) < self.hits {
                trigger = 1.0;
            }
            self.idx = (self.idx + 1) % self.steps.max(1);
        }
        self.prev_clock = clock;
        trigger
    }
}
