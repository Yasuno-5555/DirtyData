#![allow(clippy::all)]

use dirtydata_plugin_sdk::{declare_plugin, DspPlugin};

#[derive(Default)]
pub struct LorenzAttractor {
    x: f32,
    y: f32,
    z: f32,
    sigma: f32,
    rho: f32,
    beta: f32,
    dt: f32,
    sample_rate: f32,
}

impl DspPlugin for LorenzAttractor {
    fn init(&mut self, sample_rate: f32) {
        self.x = 0.1;
        self.y = 0.0;
        self.z = 0.0;

        // Classic Lorenz parameters
        self.sigma = 10.0;
        self.rho = 28.0;
        self.beta = 8.0 / 3.0;

        self.sample_rate = sample_rate;
        self.dt = 0.001; // Base time step
    }

    fn set_parameter(&mut self, id: u32, value: f32) {
        match id {
            // Speed control (Time step scaling)
            0 => self.dt = 0.0001 + value * 0.005,
            // Rho control (Chaotic threshold)
            1 => self.rho = 10.0 + value * 40.0,
            _ => {}
        }
    }

    fn process(&mut self, _in_l: f32, _in_r: f32) -> [f32; 2] {
        // Lorenz equations
        let dx = self.sigma * (self.y - self.x);
        let dy = self.x * (self.rho - self.z) - self.y;
        let dz = self.x * self.y - self.beta * self.z;

        // Euler integration step
        self.x += dx * self.dt;
        self.y += dy * self.dt;
        self.z += dz * self.dt;

        // Normalize output roughly to -1.0 .. 1.0 (X and Z axes are good for modulation)
        let out_l = self.x / 20.0;
        let out_r = (self.z - 25.0) / 20.0;

        [out_l, out_r]
    }
}

declare_plugin!(LorenzAttractor);
