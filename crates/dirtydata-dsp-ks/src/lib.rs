#![allow(clippy::all)]
#![allow(clippy::all)]

//! Karplus-Strong++ Physical Modeling String
//!
//! Refinements:
//! - Lagrange 3rd-order fractional delay interpolation
//! - Proper pick position comb filter
//! - Stiffness allpass cascade (inharmonicity)
//! - Frequency-dependent damping with DC blocker

#[derive(Clone)]
pub struct KarplusStrong {
    delay_line: Vec<f32>,
    write_idx: usize,
    sample_rate: f32,
    lp_z1: f32,
    lp_z2: f32,
    ap_state: [f32; 3],
    ap_prev_in: [f32; 3],
    dc_prev_in: f32,
    dc_prev_out: f32,
}

impl KarplusStrong {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            delay_line: vec![0.0; sample_rate as usize],
            write_idx: 0,
            sample_rate,
            lp_z1: 0.0,
            lp_z2: 0.0,
            ap_state: [0.0; 3],
            ap_prev_in: [0.0; 3],
            dc_prev_in: 0.0,
            dc_prev_out: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    #[inline]
    fn lagrange3(buf: &[f32], pos: f32, len: usize) -> f32 {
        let i = pos.floor() as isize;
        let d = pos - i as f32;
        let idx = |o: isize| -> usize {
            let mut x = (i + o) % len as isize;
            if x < 0 {
                x += len as isize;
            }
            x as usize
        };
        let (y_1, y0, y1, y2) = (buf[idx(-1)], buf[idx(0)], buf[idx(1)], buf[idx(2)]);
        let c0 = y0;
        let c1 = y1 - y_1 / 3.0 - y0 * 0.5 - y2 / 6.0;
        let c2 = (y1 + y_1) * 0.5 - y0;
        let c3 = (y1 - y_1) / 6.0 + (y0 - y1) * 0.5 + (y2 - y0) / 6.0;
        ((c3 * d + c2) * d + c1) * d + c0
    }

    #[inline]
    fn allpass1(input: f32, g: f32, prev_in: &mut f32, state: &mut f32) -> f32 {
        let out = g * input + *prev_in - g * *state;
        *prev_in = input;
        *state = out;
        out
    }

    pub fn process(
        &mut self,
        exciter: f32,
        freq_hz: f32,
        damping: f32,
        dispersion: f32,
        pick_pos: f32,
    ) -> f32 {
        let freq = freq_hz.clamp(20.0, self.sample_rate * 0.45);
        let period = self.sample_rate / freq;
        let len = self.delay_line.len();

        // Pick position comb
        let pd = (period * pick_pos.clamp(0.01, 0.99)).max(1.0);
        let mut pp = self.write_idx as f32 - pd;
        if pp < 0.0 {
            pp += len as f32;
        }
        let excited = exciter - Self::lagrange3(&self.delay_line, pp, len) * 0.5;

        // Main delay read
        let mut rp = self.write_idx as f32 - period;
        if rp < 0.0 {
            rp += len as f32;
        }
        let mut delayed = Self::lagrange3(&self.delay_line, rp, len);

        // Dispersion allpass cascade
        let dc = dispersion.clamp(0.0, 0.99) * 0.5;
        for i in 0..3 {
            delayed = Self::allpass1(
                delayed,
                dc * (1.0 - i as f32 * 0.2),
                &mut self.ap_prev_in[i],
                &mut self.ap_state[i],
            );
        }

        // Frequency-dependent damping
        let w =
            2.0 * std::f32::consts::PI * (20000.0 * (1.0 - damping.clamp(0.0, 0.99))).max(100.0)
                / self.sample_rate;
        let a = w / (1.0 + w);
        self.lp_z1 += a * (delayed - self.lp_z1);
        self.lp_z2 += a * (self.lp_z1 - self.lp_z2);
        let fb = self.lp_z2 * (0.999 - damping * 0.005);

        self.delay_line[self.write_idx] = excited + fb;
        self.write_idx = (self.write_idx + 1) % len;

        // DC blocker
        let out = fb - self.dc_prev_in + 0.995 * self.dc_prev_out;
        self.dc_prev_in = fb;
        self.dc_prev_out = out;
        out
    }
}
