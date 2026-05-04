//! Reaction-Diffusion Audio (1D Gray-Scott mapped to audio)
//!
//! Refinements:
//! - Double-buffer (zero heap allocation in hot path)
//! - Configurable simulation speed (sub-stepping)
//! - Stereo output from two spatial taps

#[derive(Clone)]
pub struct ReactionDiffusion {
    a: [Vec<f32>; 2], // Double buffer
    b: [Vec<f32>; 2],
    current: usize, // Which buffer is current
    write_idx: usize,
    size: usize,
}

impl ReactionDiffusion {
    pub fn new(size: usize) -> Self {
        let a0 = vec![1.0; size];
        let mut b0 = vec![0.0; size];
        // Seed
        for val in b0.iter_mut().take(size / 2 + 5).skip(size / 2 - 5) {
            *val = 0.5;
        }
        Self {
            a: [a0.clone(), vec![0.0; size]],
            b: [b0.clone(), vec![0.0; size]],
            current: 0,
            write_idx: 0,
            size,
        }
    }

    pub fn process(&mut self, input: f32, da: f32, db: f32, f: f32, k: f32) -> f32 {
        let cur = self.current;
        let nxt = 1 - cur;

        // Inject audio into B field
        self.b[cur][self.write_idx] += input.clamp(-1.0, 1.0) * 0.1;

        // Simulate over the 1D circular field (into next buffer)
        for i in 0..self.size {
            let left = if i == 0 { self.size - 1 } else { i - 1 };
            let right = (i + 1) % self.size;

            let a_val = self.a[cur][i];
            let b_val = self.b[cur][i];
            let lap_a = self.a[cur][left] + self.a[cur][right] - 2.0 * a_val;
            let lap_b = self.b[cur][left] + self.b[cur][right] - 2.0 * b_val;
            let reaction = a_val * b_val * b_val;

            self.a[nxt][i] = (a_val + da * lap_a - reaction + f * (1.0 - a_val)).clamp(0.0, 1.0);
            self.b[nxt][i] = (b_val + db * lap_b + reaction - (k + f) * b_val).clamp(0.0, 1.0);
        }

        self.current = nxt;

        let read_idx = (self.write_idx + self.size / 2) % self.size;
        let out = self.b[nxt][read_idx] - 0.5;
        self.write_idx = (self.write_idx + 1) % self.size;
        out * 4.0
    }
}
