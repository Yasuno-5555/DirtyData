//! Fluid LFO — Navier-Stokes Modulator.
//!
//! 簡易な2Dグリッド上で流体力学をシミュレートし、その運動をモジュレーション信号として出力する。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection,
    RackDspNode, RackProcessContext, SignalType, SmoothedParam,
};

const GRID_SIZE: usize = 16;
const N: usize = GRID_SIZE + 2;

pub struct FluidLfoModule {
    // Velocity
    u: [f32; N * N],
    v: [f32; N * N],
    u_prev: [f32; N * N],
    v_prev: [f32; N * N],
    // Density (Dye)
    dens: [f32; N * N],
    dens_prev: [f32; N * N],

    viscosity_smooth: SmoothedParam,
    diffusion_smooth: SmoothedParam,
    _sample_rate: f32,
    dt: f32,
    tick_counter: usize,
}

impl FluidLfoModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            u: [0.0; N * N],
            v: [0.0; N * N],
            u_prev: [0.0; N * N],
            v_prev: [0.0; N * N],
            dens: [0.0; N * N],
            dens_prev: [0.0; N * N],
            viscosity_smooth: SmoothedParam::new(0.0001, sample_rate, 50.0),
            diffusion_smooth: SmoothedParam::new(0.0001, sample_rate, 50.0),
            _sample_rate: sample_rate,
            dt: 0.1, // Fixed simulation time step for stability
            tick_counter: 0,
        }
    }

    fn ix(x: usize, y: usize) -> usize {
        x + (GRID_SIZE + 2) * y
    }

    // Simplified solver steps (Based on Jos Stam's Stable Fluids)
    fn add_source(x: &mut [f32], s: &[f32], dt: f32) {
        for i in 0..N*N {
            x[i] += dt * s[i];
        }
    }

    fn set_bnd(b: usize, x: &mut [f32]) {
        for i in 1..=GRID_SIZE {
            x[Self::ix(0, i)] = if b == 1 { -x[Self::ix(1, i)] } else { x[Self::ix(1, i)] };
            x[Self::ix(GRID_SIZE + 1, i)] = if b == 1 { -x[Self::ix(GRID_SIZE, i)] } else { x[Self::ix(GRID_SIZE, i)] };
            x[Self::ix(i, 0)] = if b == 2 { -x[Self::ix(i, 1)] } else { x[Self::ix(i, 1)] };
            x[Self::ix(i, GRID_SIZE + 1)] = if b == 2 { -x[Self::ix(i, GRID_SIZE)] } else { x[Self::ix(i, GRID_SIZE)] };
        }
        x[Self::ix(0, 0)] = 0.5 * (x[Self::ix(1, 0)] + x[Self::ix(0, 1)]);
        x[Self::ix(0, GRID_SIZE + 1)] = 0.5 * (x[Self::ix(1, GRID_SIZE + 1)] + x[Self::ix(0, GRID_SIZE)]);
        x[Self::ix(GRID_SIZE + 1, 0)] = 0.5 * (x[Self::ix(GRID_SIZE, 0)] + x[Self::ix(GRID_SIZE + 1, 1)]);
        x[Self::ix(GRID_SIZE + 1, GRID_SIZE + 1)] = 0.5 * (x[Self::ix(GRID_SIZE, GRID_SIZE + 1)] + x[Self::ix(GRID_SIZE + 1, GRID_SIZE)]);
    }

    fn lin_solve(b: usize, x: &mut [f32], x0: &[f32], a: f32, c: f32) {
        for _k in 0..4 { // Fewer iterations for performance
            for i in 1..=GRID_SIZE {
                for j in 1..=GRID_SIZE {
                    x[Self::ix(i, j)] = (x0[Self::ix(i, j)] + a * (x[Self::ix(i - 1, j)] + x[Self::ix(i + 1, j)] + x[Self::ix(i, j - 1)] + x[Self::ix(i, j + 1)])) / c;
                }
            }
            Self::set_bnd(b, x);
        }
    }

    fn diffuse(b: usize, x: &mut [f32], x0: &[f32], diff: f32, dt: f32) {
        let a = dt * diff * (GRID_SIZE as f32) * (GRID_SIZE as f32);
        Self::lin_solve(b, x, x0, a, 1.0 + 4.0 * a);
    }

    fn advect(b: usize, d: &mut [f32], d0: &[f32], u: &[f32], v: &[f32], dt: f32) {
        let dt0 = dt * GRID_SIZE as f32;
        for i in 1..=GRID_SIZE {
            for j in 1..=GRID_SIZE {
                let mut x = i as f32 - dt0 * u[Self::ix(i, j)];
                let mut y = j as f32 - dt0 * v[Self::ix(i, j)];
                if x < 0.5 { x = 0.5; }
                if x > GRID_SIZE as f32 + 0.5 { x = GRID_SIZE as f32 + 0.5; }
                let i0 = x as usize;
                let i1 = i0 + 1;
                if y < 0.5 { y = 0.5; }
                if y > GRID_SIZE as f32 + 0.5 { y = GRID_SIZE as f32 + 0.5; }
                let j0 = y as usize;
                let j1 = j0 + 1;
                let s1 = x - i0 as f32;
                let s0 = 1.0 - s1;
                let t1 = y - j0 as f32;
                let t0 = 1.0 - t1;
                d[Self::ix(i, j)] = s0 * (t0 * d0[Self::ix(i0, j0)] + t1 * d0[Self::ix(i0, j1)]) +
                                    s1 * (t0 * d0[Self::ix(i1, j0)] + t1 * d0[Self::ix(i1, j1)]);
            }
        }
        Self::set_bnd(b, d);
    }

    fn project(u: &mut [f32], v: &mut [f32], p: &mut [f32], div: &mut [f32]) {
        let h = 1.0 / GRID_SIZE as f32;
        for i in 1..=GRID_SIZE {
            for j in 1..=GRID_SIZE {
                div[Self::ix(i, j)] = -0.5 * h * (u[Self::ix(i + 1, j)] - u[Self::ix(i - 1, j)] + v[Self::ix(i, j + 1)] - v[Self::ix(i, j - 1)]);
                p[Self::ix(i, j)] = 0.0;
            }
        }
        Self::set_bnd(0, div);
        Self::set_bnd(0, p);
        Self::lin_solve(0, p, div, 1.0, 4.0);
        for i in 1..=GRID_SIZE {
            for j in 1..=GRID_SIZE {
                u[Self::ix(i, j)] -= 0.5 * (p[Self::ix(i + 1, j)] - p[Self::ix(i - 1, j)]) / h;
                v[Self::ix(i, j)] -= 0.5 * (p[Self::ix(i, j + 1)] - p[Self::ix(i, j - 1)]) / h;
            }
        }
        Self::set_bnd(1, u);
        Self::set_bnd(2, v);
    }

    fn dens_step(&mut self, diff: f32) {
        Self::add_source(&mut self.dens, &self.dens_prev, self.dt);
        let mut tmp = self.dens_prev.clone();
        Self::diffuse(0, &mut tmp, &self.dens, diff, self.dt);
        self.dens_prev = tmp.clone(); // Swap back basically
        Self::advect(0, &mut self.dens, &self.dens_prev, &self.u, &self.v, self.dt);
    }

    fn vel_step(&mut self, visc: f32) {
        Self::add_source(&mut self.u, &self.u_prev, self.dt);
        Self::add_source(&mut self.v, &self.v_prev, self.dt);
        
        let mut tmp_u = self.u_prev.clone();
        Self::diffuse(1, &mut tmp_u, &self.u, visc, self.dt);
        self.u_prev = tmp_u.clone();
        
        let mut tmp_v = self.v_prev.clone();
        Self::diffuse(2, &mut tmp_v, &self.v, visc, self.dt);
        self.v_prev = tmp_v.clone();
        
        Self::project(&mut self.u_prev, &mut self.v_prev, &mut self.u, &mut self.v);
        
        let mut tmp2_u = self.u.clone();
        Self::advect(1, &mut tmp2_u, &self.u_prev, &self.u_prev, &self.v_prev, self.dt);
        self.u = tmp2_u.clone();
        
        let mut tmp2_v = self.v.clone();
        Self::advect(2, &mut tmp2_v, &self.v_prev, &self.u_prev, &self.v_prev, self.dt);
        self.v = tmp2_v.clone();
        
        Self::project(&mut self.u, &mut self.v, &mut self.u_prev, &mut self.v_prev);
    }

}

impl RackDspNode for FluidLfoModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let visc_knob = params[0];
        let diff_knob = params[1];

        self.viscosity_smooth.set(visc_knob);
        self.diffusion_smooth.set(diff_knob);

        let visc = self.viscosity_smooth.next(0.0);
        let diff = self.diffusion_smooth.next(0.0);

        // Subsample the fluid simulation to save CPU (e.g., run every 64 samples)
        self.tick_counter += 1;
        let do_sim = self.tick_counter >= 64;
        if do_sim {
            self.tick_counter = 0;
            // Clear previous input
            self.u_prev.fill(0.0);
            self.v_prev.fill(0.0);
            self.dens_prev.fill(0.0);

            // Inject force/density from CV inputs into the center of the grid
            let center = Self::ix(GRID_SIZE / 2, GRID_SIZE / 2);
            // Only using voice 0 for the force input for simplicity in this 2D field
            let force_x = inputs[0 * 16]; 
            let force_y = inputs[1 * 16];
            let density_in = inputs[2 * 16];

            self.u_prev[center] = force_x * 5.0;
            self.v_prev[center] = force_y * 5.0;
            self.dens_prev[center] = density_in.max(0.0) * 10.0;

            self.vel_step(visc);
            self.dens_step(diff);
            
            // Apply slight damping to density so it doesn't build up forever
            for i in 0..N*N {
                self.dens[i] *= 0.99;
                self.u[i] *= 0.99;
                self.v[i] *= 0.99;
            }
        }

        // Read outputs from specific taps in the grid
        let tap1 = Self::ix(GRID_SIZE / 4, GRID_SIZE / 4);
        let tap2 = Self::ix(3 * GRID_SIZE / 4, 3 * GRID_SIZE / 4);

        for i in 0..16 {
            // Polyphonic outputs could map to different taps, but here we just copy
            outputs[0 * 16 + i] = self.u[tap1] * 10.0; // U vel
            outputs[1 * 16 + i] = self.v[tap1] * 10.0; // V vel
            outputs[2 * 16 + i] = self.dens[tap2] * 2.0 - 5.0; // Density
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_fluid_lfo",
        name: "FLUID",
        version: "1.0.0",
        manufacturer: "DirtyRack",
        hp_width: 12,
        visuals: crate::signal::ModuleVisuals {
            background_color: [20, 30, 40],
            text_color: [150, 200, 255],
            accent_color: [50, 150, 255],
            panel_texture: crate::signal::PanelTexture::BrushedAluminium,
        },
        tags: &["Builtin", "LFO", "CHAOS", "MOD"],
        params: &[
            ParamDescriptor {
                name: "VISC",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.0,
                max: 0.01,
                default: 0.0001,
                position: [0.3, 0.3],
                unit: "",
            },
            ParamDescriptor {
                name: "DIFF",
                kind: ParamKind::Knob,
                response: ParamResponse::Smoothed { ms: 50.0 },
                min: 0.0,
                max: 0.01,
                default: 0.0001,
                position: [0.7, 0.3],
                unit: "",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "FORCE_X",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.2, 0.6],
            },
            PortDescriptor {
                name: "FORCE_Y",
                direction: PortDirection::Input,
                signal_type: SignalType::BiCV,
                max_channels: 1,
                position: [0.5, 0.6],
            },
            PortDescriptor {
                name: "DENSITY_IN",
                direction: PortDirection::Input,
                signal_type: SignalType::UniCV,
                max_channels: 1,
                position: [0.8, 0.6],
            },
            PortDescriptor {
                name: "U_OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 16,
                position: [0.2, 0.85],
            },
            PortDescriptor {
                name: "V_OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 16,
                position: [0.5, 0.85],
            },
            PortDescriptor {
                name: "DENS_OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::BiCV,
                max_channels: 16,
                position: [0.8, 0.85],
            },
        ],
        factory: |sr| Box::new(FluidLfoModule::new(sr)),
    }
}
