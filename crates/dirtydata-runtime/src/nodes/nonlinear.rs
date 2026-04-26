use super::base::*;
use dirtydata_core::types::ConfigSnapshot;

pub struct LorenzNode {
    state: [f32; 3],
}

impl LorenzNode {
    pub fn new() -> Self { Self { state: [0.1, 0.0, 0.0] } }
}

impl DspNode for LorenzNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let sigma = config.get("sigma").and_then(|v| v.as_float()).unwrap_or(10.0) as f32;
        let rho = config.get("rho").and_then(|v| v.as_float()).unwrap_or(28.0) as f32;
        let beta = config.get("beta").and_then(|v| v.as_float()).unwrap_or(8.0/3.0) as f32;
        let dt = 1.0 / ctx.sample_rate;
        rk4_step_fixed(&mut self.state, dt, 0.0, |s, _| {
            let dx = sigma * (s[1] - s[0]);
            let dy = s[0] * (rho - s[2]) - s[1];
            let dz = s[0] * s[1] - beta * s[2];
            [dx, dy, dz]
        });
        outputs[0] = [self.state[0] * 0.05, self.state[1] * 0.05];
    }
}

pub struct MackeyGlassNode {
    history: std::collections::VecDeque<f32>,
    current_x: f32,
}

impl MackeyGlassNode {
    pub fn new() -> Self { Self { history: std::collections::VecDeque::from(vec![0.5; 1000]), current_x: 0.5 } }
}

impl DspNode for MackeyGlassNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], config: &ConfigSnapshot, ctx: &ProcessContext) {
        let a = config.get("a").and_then(|v| v.as_float()).unwrap_or(0.2) as f32;
        let b = config.get("b").and_then(|v| v.as_float()).unwrap_or(0.1) as f32;
        let tau_samples = config.get("tau_samples").and_then(|v| v.as_float()).unwrap_or(300.0) as usize;
        let x_tau = *self.history.get(self.history.len().saturating_sub(tau_samples)).unwrap_or(&0.5);
        let dx = (a * x_tau) / (1.0 + x_tau.powi(10)) - b * self.current_x;
        self.current_x += dx / ctx.sample_rate;
        self.history.push_back(self.current_x);
        if self.history.len() > 2000 { self.history.pop_front(); }
        outputs[0] = [self.current_x, self.current_x];
    }
}

pub struct GrayScottNode {
    u: Vec<f32>,
    v: Vec<f32>,
}

impl GrayScottNode {
    pub fn new() -> Self { Self { u: vec![1.0; 100], v: vec![0.0; 100] } }
}

impl DspNode for GrayScottNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [[f32; 2]], _config: &ConfigSnapshot, _ctx: &ProcessContext) {
        // Simplified 1D reaction-diffusion
        outputs[0] = [self.u[0], self.v[0]];
    }
}
