//! Chaotic Circuit Models (Chua's Circuit, Rössler)
//!
//! Refinements:
//! - RK4 integration instead of Euler (massive stability improvement)
//! - 2x oversampling for high `rate` values
//! - Proper Chua diode piecewise-linear function
//! - Output scaling with DC blocking

#[derive(Clone)]
pub struct ChuaCircuit {
    x: f32,
    y: f32,
    z: f32,
    sample_rate: f32,
    // DC blocker state
    dc_prev_in: f32,
    dc_prev_out: f32,
}

impl ChuaCircuit {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            x: 0.1,
            y: 0.0,
            z: 0.0,
            sample_rate,
            dc_prev_in: 0.0,
            dc_prev_out: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Chua's diode characteristic (piecewise-linear)
    #[inline]
    fn chua_diode(x: f32, m0: f32, m1: f32) -> f32 {
        m1 * x + 0.5 * (m0 - m1) * ((x + 1.0).abs() - (x - 1.0).abs())
    }

    /// Compute derivatives for Chua's circuit
    #[inline]
    fn derivatives(state: &[f32; 3], alpha: f32, beta: f32, m0: f32, m1: f32) -> [f32; 3] {
        let x = state[0];
        let y = state[1];
        let z = state[2];
        let f_x = Self::chua_diode(x, m0, m1);
        [alpha * (y - x - f_x), x - y + z, -beta * y]
    }

    /// RK4 integration step (stack-allocated, zero heap allocation)
    fn rk4_step(state: &mut [f32; 3], dt: f32, alpha: f32, beta: f32, m0: f32, m1: f32) {
        let k1 = Self::derivatives(state, alpha, beta, m0, m1);

        let s2 = [
            state[0] + k1[0] * dt * 0.5,
            state[1] + k1[1] * dt * 0.5,
            state[2] + k1[2] * dt * 0.5,
        ];
        let k2 = Self::derivatives(&s2, alpha, beta, m0, m1);

        let s3 = [
            state[0] + k2[0] * dt * 0.5,
            state[1] + k2[1] * dt * 0.5,
            state[2] + k2[2] * dt * 0.5,
        ];
        let k3 = Self::derivatives(&s3, alpha, beta, m0, m1);

        let s4 = [
            state[0] + k3[0] * dt,
            state[1] + k3[1] * dt,
            state[2] + k3[2] * dt,
        ];
        let k4 = Self::derivatives(&s4, alpha, beta, m0, m1);

        for i in 0..3 {
            state[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
    }

    /// Process the chaotic circuit.
    /// `alpha`: typically around 15.6
    /// `beta`: typically around 28.0
    /// `rate`: scales the internal dt, acting as pitch control
    pub fn process(&mut self, alpha: f32, beta: f32, rate: f32) -> f32 {
        let base_dt = 1.0 / self.sample_rate;
        let rate_clamped = rate.clamp(0.01, 100.0);

        // Chua diode parameters
        let m0 = -1.143_f32;
        let m1 = -0.714_f32;

        // Adaptive oversampling: use 2x for high rates, 4x for very high rates
        let oversample = if rate_clamped > 10.0 {
            4
        } else if rate_clamped > 2.0 {
            2
        } else {
            1
        };
        let dt = (base_dt * rate_clamped) / oversample as f32;

        let mut state = [self.x, self.y, self.z];

        for _ in 0..oversample {
            Self::rk4_step(&mut state, dt, alpha, beta, m0, m1);
        }

        // Soft clamp (prevents blow-up while preserving dynamics)
        for s in &mut state {
            *s = s.clamp(-50.0, 50.0);
            if !s.is_finite() {
                *s = 0.0;
            }
        }

        self.x = state[0];
        self.y = state[1];
        self.z = state[2];

        // Scale to audio range with DC blocking
        let raw = self.x * 0.05;

        // DC blocker (1-pole highpass at ~5Hz)
        let dc_coeff = 1.0 - (2.0 * std::f32::consts::PI * 5.0 / self.sample_rate);
        let dc_out = raw - self.dc_prev_in + dc_coeff * self.dc_prev_out;
        self.dc_prev_in = raw;
        self.dc_prev_out = dc_out;

        dc_out
    }
}

#[derive(Clone)]
pub struct Lorenz {
    state: [f32; 3],
}

impl Lorenz {
    pub fn new() -> Self {
        Self {
            state: [0.1, 0.0, 0.0],
        }
    }
}

impl Default for Lorenz {
    fn default() -> Self {
        Self::new()
    }
}

impl Lorenz {
    pub fn process(&mut self, sigma: f32, rho: f32, beta: f32, dt: f32) -> [f32; 3] {
        let dx = sigma * (self.state[1] - self.state[0]);
        let dy = self.state[0] * (rho - self.state[2]) - self.state[1];
        let dz = self.state[0] * self.state[1] - beta * self.state[2];
        self.state[0] += dx * dt;
        self.state[1] += dy * dt;
        self.state[2] += dz * dt;
        self.state
    }
}

#[derive(Clone)]
pub struct MackeyGlass {
    history: std::collections::VecDeque<f32>,
    current_x: f32,
}

impl MackeyGlass {
    pub fn new() -> Self {
        Self {
            history: std::collections::VecDeque::from(vec![0.5; 1000]),
            current_x: 0.5,
        }
    }
}

impl Default for MackeyGlass {
    fn default() -> Self {
        Self::new()
    }
}

impl MackeyGlass {
    pub fn process(&mut self, a: f32, b: f32, tau: usize, dt: f32) -> f32 {
        let x_tau = *self
            .history
            .get(self.history.len().saturating_sub(tau))
            .unwrap_or(&0.5);
        let dx = (a * x_tau) / (1.0 + x_tau.powi(10)) - b * self.current_x;
        self.current_x += dx * dt;
        self.history.push_back(self.current_x);
        if self.history.len() > 2000 {
            self.history.pop_front();
        }
        self.current_x
    }
}
