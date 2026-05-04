use super::base::*;
use dirtydata_core::types::ConfigSnapshot;

#[derive(Clone)]
pub struct WavefolderNode {
    _stages: usize,
}

impl WavefolderNode {
    pub fn new() -> Self {
        Self { _stages: 4 }
    }
}

impl DspNode for WavefolderNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let gain = config.get("gain").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let stages = config
            .get("stages")
            .and_then(|v| match v {
                dirtydata_core::types::ConfigValue::Int(i) => Some(*i as usize),
                _ => None,
            })
            .unwrap_or(4);

        for i in 0..outputs.len() {
            let mut l = inputs.get(i * 2).cloned().unwrap_or(0.0) * gain;
            let mut r = inputs.get(i * 2 + 1).cloned().unwrap_or(0.0) * gain;

            for _ in 0..stages {
                l = (l * std::f32::consts::PI * 0.5).sin();
                r = (r * std::f32::consts::PI * 0.5).sin();
            }
            outputs[i] = [l, r];
        }
    }
}

#[derive(Clone)]
pub struct LorenzNode {
    state: [f32; 3],
    sigma: f32,
    rho: f32,
    beta: f32,
}

impl LorenzNode {
    pub fn new() -> Self {
        Self {
            state: [0.1, 0.0, 0.0],
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        }
    }
}

impl DspNode for LorenzNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let speed = config
            .get("speed")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;
        let dt = speed / ctx.sample_rate;

        let sigma = self.sigma;
        let rho = self.rho;
        let beta = self.beta;

        rk4_step_fixed(&mut self.state, dt, 0.0, |state, _t| {
            [
                sigma * (state[1] - state[0]),
                state[0] * (rho - state[2]) - state[1],
                state[0] * state[1] - beta * state[2],
            ]
        });

        for s in &mut self.state {
            *s = s.clamp(-100.0, 100.0);
            if !s.is_finite() {
                *s = 0.1;
            }
        }

        outputs[0] = [self.state[0] * 0.05, self.state[1] * 0.05];
        if outputs.len() > 1 {
            outputs[1] = [self.state[2] * 0.05, 0.0];
        }
    }
}

use std::collections::VecDeque;

#[derive(Clone)]
pub struct MackeyGlassNode {
    history: VecDeque<f32>,
    _tau_samples: usize,
    beta: f32,
    gamma: f32,
    n: f32,
    current_x: f32,
}

impl MackeyGlassNode {
    pub fn new(tau_ms: f32, sample_rate: f32) -> Self {
        let tau_samples = (tau_ms * 0.001 * sample_rate) as usize;
        let mut history = VecDeque::with_capacity(tau_samples + 1);
        for _ in 0..=tau_samples {
            history.push_back(0.5);
        }
        Self {
            history,
            _tau_samples: tau_samples,
            beta: 2.0,
            gamma: 1.0,
            n: 10.0,
            current_x: 0.5,
        }
    }
}

impl DspNode for MackeyGlassNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let speed = config
            .get("speed")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;
        let dt = speed / ctx.sample_rate;

        let x_tau = *self.history.front().unwrap();
        let f = |x: f32, xt: f32| self.beta * xt / (1.0 + xt.powf(self.n)) - self.gamma * x;

        let k1 = f(self.current_x, x_tau);
        let k2 = f(self.current_x + k1 * dt * 0.5, x_tau);
        let k3 = f(self.current_x + k2 * dt * 0.5, x_tau);
        let k4 = f(self.current_x + k3 * dt, x_tau);

        self.current_x += (dt / 6.0) * (k1 + 2.0 * k2 + 2.0 * k3 + k4);
        self.history.push_back(self.current_x);
        self.history.pop_front();

        outputs[0] = [self.current_x, self.current_x];
    }
}

#[derive(Clone)]
pub struct GrayScottNode {
    u: [Vec<f32>; 2],
    v: [Vec<f32>; 2],
    current: usize,
    size: usize,
    f: f32,
    k: f32,
    du: f32,
    dv: f32,
}

impl GrayScottNode {
    pub fn new(size: usize) -> Self {
        let u0 = vec![1.0; size];
        let mut v0 = vec![0.0; size];
        for i in (size / 2 - 5)..(size / 2 + 5) {
            v0[i] = 0.5;
        }
        Self {
            u: [u0.clone(), vec![0.0; size]],
            v: [v0.clone(), vec![0.0; size]],
            current: 0,
            size,
            f: 0.0545,
            k: 0.062,
            du: 0.1,
            dv: 0.05,
        }
    }
}

impl DspNode for GrayScottNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let cur = self.current;
        let nxt = 1 - cur;

        for i in 0..self.size {
            let prev = if i == 0 { self.size - 1 } else { i - 1 };
            let next = if i == self.size - 1 { 0 } else { i + 1 };

            let u_val = self.u[cur][i];
            let v_val = self.v[cur][i];
            let lap_u = self.u[cur][prev] + self.u[cur][next] - 2.0 * u_val;
            let lap_v = self.v[cur][prev] + self.v[cur][next] - 2.0 * v_val;
            let uv2 = u_val * v_val * v_val;

            self.u[nxt][i] =
                (u_val + self.du * lap_u - uv2 + self.f * (1.0 - u_val)).clamp(0.0, 1.5);
            self.v[nxt][i] =
                (v_val + self.dv * lap_v + uv2 - (self.f + self.k) * v_val).clamp(0.0, 1.5);
        }

        self.current = nxt;
        outputs[0] = [
            self.u[nxt][self.size / 2] * 2.0 - 1.0,
            self.v[nxt][self.size / 2] * 2.0 - 1.0,
        ];
    }
}

#[derive(Clone)]
pub struct ChuaCircuitNode {
    inner: dirtydata_dsp_chaos::ChuaCircuit,
}
impl ChuaCircuitNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_chaos::ChuaCircuit::new(sample_rate),
        }
    }
}
impl DspNode for ChuaCircuitNode {
    fn process(
        &mut self,
        _inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let alpha = config
            .get("alpha")
            .and_then(|v| v.as_float())
            .unwrap_or(15.6) as f32;
        let beta = config
            .get("beta")
            .and_then(|v| v.as_float())
            .unwrap_or(28.0) as f32;
        let rate = config.get("rate").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;

        let out = self.inner.process(alpha, beta, rate);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct ReactionDiffusionNode {
    inner: dirtydata_dsp_reaction::ReactionDiffusion,
}
impl ReactionDiffusionNode {
    pub fn new() -> Self {
        Self {
            inner: dirtydata_dsp_reaction::ReactionDiffusion::new(256),
        }
    }
}
impl DspNode for ReactionDiffusionNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let da = config.get("da").and_then(|v| v.as_float()).unwrap_or(1.0) as f32;
        let db = config.get("db").and_then(|v| v.as_float()).unwrap_or(0.5) as f32;
        let f = config.get("f").and_then(|v| v.as_float()).unwrap_or(0.055) as f32;
        let k = config.get("k").and_then(|v| v.as_float()).unwrap_or(0.062) as f32;

        let out = self.inner.process(input, da, db, f, k);
        for o in outputs {
            *o = [out, out];
        }
    }
}
