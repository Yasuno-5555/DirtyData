use super::base::*;
use dirtydata_core::types::ConfigSnapshot;
use rustfft::{num_complex::Complex, FftPlanner};

#[derive(Clone)]
pub struct SpectralFreezeNode {
    size: usize,
    buffer: Vec<f32>,
    fft_result: Vec<Complex<f32>>,
    frozen: bool,
    write_pos: usize,
    read_pos: usize,
}

impl SpectralFreezeNode {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            buffer: vec![0.0; size],
            fft_result: vec![Complex::default(); size],
            frozen: false,
            write_pos: 0,
            read_pos: 0,
        }
    }
}

impl DspNode for SpectralFreezeNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let freeze = config
            .get("freeze")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let input = inputs.get(0).cloned().unwrap_or(0.0);

        if freeze && !self.frozen {
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(self.size);
            let mut complex_buf: Vec<Complex<f32>> =
                self.buffer.iter().map(|&x| Complex::new(x, 0.0)).collect();
            fft.process(&mut complex_buf);
            self.fft_result = complex_buf;
            self.frozen = true;

            let ifft = planner.plan_fft_inverse(self.size);
            let mut inv_buf = self.fft_result.clone();
            ifft.process(&mut inv_buf);
            for (i, c) in inv_buf.iter().enumerate() {
                self.buffer[i] = c.re / self.size as f32;
            }
        } else if !freeze {
            self.frozen = false;
        }

        if !self.frozen {
            self.buffer[self.write_pos] = input;
            self.write_pos = (self.write_pos + 1) % self.size;
        }

        let out_val = if self.frozen {
            let v = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.size;
            v
        } else {
            input
        };

        for out in outputs.iter_mut() {
            *out = [out_val, out_val];
        }
    }
}

#[derive(Clone)]
pub struct FFTConvolveNode {
    size: usize,
    input_buffer: Vec<f32>,
    impulse_buffer: Vec<f32>,
    result_buffer: Vec<f32>,
    pos: usize,
}

impl FFTConvolveNode {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            input_buffer: vec![0.0; size],
            impulse_buffer: vec![0.0; size],
            result_buffer: vec![0.0; size],
            pos: 0,
        }
    }
}

impl DspNode for FFTConvolveNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).cloned().unwrap_or(0.0);
        let impulse = inputs.get(1).cloned().unwrap_or(0.0);

        self.input_buffer[self.pos] = input;
        self.impulse_buffer[self.pos] = impulse;
        self.pos += 1;

        if self.pos >= self.size {
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(self.size);

            let mut in_complex: Vec<Complex<f32>> = self
                .input_buffer
                .iter()
                .map(|&x| Complex::new(x, 0.0))
                .collect();
            let mut imp_complex: Vec<Complex<f32>> = self
                .impulse_buffer
                .iter()
                .map(|&x| Complex::new(x, 0.0))
                .collect();

            fft.process(&mut in_complex);
            fft.process(&mut imp_complex);

            for i in 0..self.size {
                in_complex[i] *= imp_complex[i];
            }

            let ifft = planner.plan_fft_inverse(self.size);
            ifft.process(&mut in_complex);

            for (i, c) in in_complex.iter().enumerate() {
                self.result_buffer[i] = c.re / self.size as f32;
            }
            self.pos = 0;
        }

        let out_val = self.result_buffer[self.pos];

        for out in outputs.iter_mut() {
            *out = [out_val, out_val];
        }
    }
}

#[derive(Clone)]
pub struct Grain {
    pub pos: f32,
    pub duration_samples: f32,
    pub current_sample: f32,
    pub active: bool,
}

#[derive(Clone)]
pub struct GranularNode {
    buffer: Vec<[f32; 2]>,
    write_pos: usize,
    grains: Vec<Grain>,
    next_grain_samples: f32,
}

impl GranularNode {
    pub fn new(sample_rate: f32) -> Self {
        let buf_size = (sample_rate * 2.0) as usize; // 2 seconds buffer
        let mut grains = Vec::new();
        for _ in 0..16 {
            grains.push(Grain {
                pos: 0.0,
                duration_samples: 0.0,
                current_sample: 0.0,
                active: false,
            });
        }
        Self {
            buffer: vec![[0.0, 0.0]; buf_size],
            write_pos: 0,
            grains,
            next_grain_samples: 0.0,
        }
    }
}

impl DspNode for GranularNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let pos_norm = config
            .get("position")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let size_ms = config
            .get("size")
            .and_then(|v| v.as_float())
            .unwrap_or(50.0) as f32;
        let density = config
            .get("density")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;

        let size_samples = (size_ms * 0.001 * ctx.sample_rate) as f32;

        for i in 0..outputs.len() {
            let in_l = inputs.get(i * 2).cloned().unwrap_or(0.0);
            let in_r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0);
            self.buffer[self.write_pos] = [in_l, in_r];
            self.write_pos = (self.write_pos + 1) % self.buffer.len();

            self.next_grain_samples -= 1.0;
            if self.next_grain_samples <= 0.0 {
                if let Some(grain) = self.grains.iter_mut().find(|g| !g.active) {
                    grain.active = true;
                    grain.current_sample = 0.0;
                    grain.duration_samples = size_samples;
                    let jitter = (rand::random::<f32>() - 0.5) * 0.05;
                    grain.pos = (pos_norm + jitter).clamp(0.0, 1.0);
                }
                self.next_grain_samples = (1.0 - density) * size_samples * 0.5 + 100.0;
            }

            let mut mixed = [0.0, 0.0];
            for grain in self.grains.iter_mut().filter(|g| g.active) {
                let norm_idx = grain.current_sample / grain.duration_samples;
                let window = 1.0 - (2.0 * norm_idx - 1.0).abs();
                let read_base = (grain.pos * (self.buffer.len() as f32 - 1.0)) as usize;
                let read_idx = (read_base + grain.current_sample as usize) % self.buffer.len();
                let val = self.buffer[read_idx];

                mixed[0] += val[0] * window;
                mixed[1] += val[1] * window;

                grain.current_sample += 1.0;
                if grain.current_sample >= grain.duration_samples {
                    grain.active = false;
                }
            }

            outputs[i] = mixed;
        }
    }
}
