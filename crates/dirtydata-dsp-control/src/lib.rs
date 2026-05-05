#![allow(clippy::all)]
#![allow(clippy::all)]

//! Control and Modulation Civilization

#[derive(Clone)]
pub struct FunctionGenerator {
    pub state: f32,
    pub stage: u8, // 0: idle, 1: rise, 2: fall
}

impl FunctionGenerator {
    pub fn new() -> Self {
        Self {
            state: 0.0,
            stage: 0,
        }
    }
    pub fn process(
        &mut self,
        trigger: f32,
        rise: f32,
        fall: f32,
        cycle: bool,
        sample_rate: f32,
    ) -> f32 {
        if trigger > 0.5 && self.stage == 0 {
            self.stage = 1;
        }

        match self.stage {
            1 => {
                // Rise
                let step = 1.0 / (rise.max(0.001) * sample_rate);
                self.state += step;
                if self.state >= 1.0 {
                    self.state = 1.0;
                    self.stage = 2;
                }
            }
            2 => {
                // Fall
                let step = 1.0 / (fall.max(0.001) * sample_rate);
                self.state -= step;
                if self.state <= 0.0 {
                    self.state = 0.0;
                    if cycle {
                        self.stage = 1;
                    } else {
                        self.stage = 0;
                    }
                }
            }
            _ => {}
        }
        self.state
    }
}

#[derive(Clone)]
pub struct RandomSource {
    prev_clock: f32,
    val: f32,
}

impl RandomSource {
    pub fn new() -> Self {
        Self {
            prev_clock: 0.0,
            val: 0.0,
        }
    }
    pub fn process(&mut self, clock: f32, mode: u8) -> f32 {
        if clock > 0.5 && self.prev_clock <= 0.5 {
            let r = rand::random::<f32>();
            match mode {
                0 => self.val = r,                                            // S&H
                1 => self.val = (self.val + (r - 0.5) * 0.1).clamp(0.0, 1.0), // Drunk
                _ => self.val = r,
            }
        }
        self.prev_clock = clock;
        self.val
    }
}
