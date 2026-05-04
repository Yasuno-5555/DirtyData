use dirtydata_plugin_sdk::{DspPlugin, declare_plugin};

#[derive(Default)]
pub struct BitcrushPlugin {
    bits: f32,
    rate_divider: f32,
    counter: f32,
    last_val: [f32; 2],
}

impl DspPlugin for BitcrushPlugin {
    fn init(&mut self, _sample_rate: f32) {
        self.bits = 16.0;
        self.rate_divider = 1.0;
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        match id {
            0 => self.bits = value * 16.0,
            1 => self.rate_divider = 1.0 + value * 10.0,
            _ => {}
        }
    }

    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2] {
        self.counter += 1.0;
        if self.counter >= self.rate_divider {
            self.counter = 0.0;

            let quantize = |x: f32| {
                let levels = 2.0f32.powf(self.bits);
                (x * levels).round() / levels
            };

            self.last_val = [quantize(in_l), quantize(in_r)];
        }
        self.last_val
    }
}

declare_plugin!(BitcrushPlugin);
