#![allow(clippy::all)]
#![allow(clippy::all)]

//! Wave Digital Filter (WDF) Component Library
//! Topology-preserving physical modeling of analog circuits.
//!
//! Refinements:
//! - Newton-Raphson solver for diode pair nonlinearity (C5 fix)
//! - Inductor component with proper state update
//! - Improved series/parallel adaptors

/// A generic WDF one-port component
pub trait WdfNode {
    fn wave_up(&mut self) -> f32;
    fn set_wave_down(&mut self, wave: f32);
    fn port_resistance(&self) -> f32;
}

/// WDF Capacitor
#[derive(Clone)]
pub struct Capacitor {
    rp: f32,
    state: f32,
}
impl Capacitor {
    pub fn new(capacitance: f32, sample_rate: f32) -> Self {
        Self {
            rp: 1.0 / (2.0 * capacitance * sample_rate),
            state: 0.0,
        }
    }
}
impl WdfNode for Capacitor {
    fn wave_up(&mut self) -> f32 {
        self.state
    }
    fn set_wave_down(&mut self, wave: f32) {
        self.state = wave;
    }
    fn port_resistance(&self) -> f32 {
        self.rp
    }
}

/// WDF Resistor
#[derive(Clone)]
pub struct Resistor {
    rp: f32,
}
impl Resistor {
    pub fn new(r: f32) -> Self {
        Self { rp: r.max(1e-6) }
    }
}
impl WdfNode for Resistor {
    fn wave_up(&mut self) -> f32 {
        0.0
    }
    fn set_wave_down(&mut self, _wave: f32) {}
    fn port_resistance(&self) -> f32 {
        self.rp
    }
}

/// WDF Inductor
#[derive(Clone)]
pub struct Inductor {
    rp: f32,
    state: f32,
}
impl Inductor {
    pub fn new(inductance: f32, sample_rate: f32) -> Self {
        Self {
            rp: 2.0 * inductance * sample_rate,
            state: 0.0,
        }
    }
}
impl WdfNode for Inductor {
    fn wave_up(&mut self) -> f32 {
        -self.state
    }
    fn set_wave_down(&mut self, wave: f32) {
        self.state = wave;
    }
    fn port_resistance(&self) -> f32 {
        self.rp
    }
}

/// Simple RC circuit topology
#[derive(Clone)]
pub struct WdfSimpleRc {
    capacitor: Capacitor,
    _resistor: Resistor,
    rho: f32,
}
impl WdfSimpleRc {
    pub fn new(r: f32, c: f32, sample_rate: f32) -> Self {
        let capacitor = Capacitor::new(c, sample_rate);
        let resistor = Resistor::new(r);
        let rho =
            resistor.port_resistance() / (resistor.port_resistance() + capacitor.port_resistance());
        Self {
            capacitor,
            _resistor: resistor,
            rho,
        }
    }

    pub fn process(&mut self, voltage_in: f32) -> f32 {
        let a1 = voltage_in;
        let a2 = self.capacitor.wave_up();
        let wave_diff = a1 - a2;
        let b1 = a1 - self.rho * wave_diff;
        let _b2 = a2 + (1.0 - self.rho) * wave_diff;
        self.capacitor.set_wave_down(b1);
        (a2 + b1) * 0.5
    }
}

// ──────────────────────────────────────────────
// Advanced WDF Components
// ──────────────────────────────────────────────

/// WDF 3-Port Series Adaptor
#[derive(Clone)]
pub struct SeriesAdaptor {
    r1: f32,
    r2: f32,
    r3: f32,
}
impl SeriesAdaptor {
    pub fn new(r1: f32, r2: f32) -> Self {
        Self {
            r1,
            r2,
            r3: r1 + r2,
        }
    }
    pub fn set_resistances(&mut self, r1: f32, r2: f32) {
        self.r1 = r1;
        self.r2 = r2;
        self.r3 = r1 + r2;
    }
    pub fn port3_resistance(&self) -> f32 {
        self.r3
    }
    pub fn process(&self, a1: f32, a2: f32, a3: f32) -> (f32, f32, f32) {
        let b3 = -(a1 + a2);
        let diff = a3 - b3;
        let b1 = a1 + (self.r1 / self.r3) * diff;
        let b2 = a2 + (self.r2 / self.r3) * diff;
        (b1, b2, b3)
    }
}

/// WDF 3-Port Parallel Adaptor
#[derive(Clone)]
pub struct ParallelAdaptor {
    g1: f32,
    g2: f32,
    g3: f32,
}
impl ParallelAdaptor {
    pub fn new(r1: f32, r2: f32) -> Self {
        let g1 = 1.0 / r1.max(1e-9);
        let g2 = 1.0 / r2.max(1e-9);
        Self {
            g1,
            g2,
            g3: g1 + g2,
        }
    }
    pub fn set_resistances(&mut self, r1: f32, r2: f32) {
        self.g1 = 1.0 / r1.max(1e-9);
        self.g2 = 1.0 / r2.max(1e-9);
        self.g3 = self.g1 + self.g2;
    }
    pub fn port3_resistance(&self) -> f32 {
        1.0 / self.g3
    }
    pub fn process(&self, a1: f32, a2: f32, a3: f32) -> (f32, f32, f32) {
        let b3 = (self.g1 * a1 + self.g2 * a2) / self.g3;
        let diff = a3 - b3;
        (a1 + diff, a2 + diff, b3)
    }
}

/// Antiparallel Diode Pair with Newton-Raphson solver (C5 fix).
///
/// Uses the Shockley diode equation: I = Is * (exp(V/Vt) - 1)
/// For antiparallel pair, the implicit equation is solved iteratively.
#[derive(Clone)]
pub struct WdfDiodePair {
    rp: f32,
    is: f32,       // Reverse saturation current
    vt: f32,       // Thermal voltage (kT/q ≈ 26mV)
    nr_iters: u32, // Newton-Raphson iterations
}

impl WdfDiodePair {
    pub fn new(is: f32, vt: f32) -> Self {
        Self {
            rp: 1.0,
            is,
            vt: vt.max(0.001),
            nr_iters: 8,
        }
    }

    pub fn set_port_resistance(&mut self, rp: f32) {
        self.rp = rp.max(1e-6);
    }

    /// Newton-Raphson solver for diode pair wave reflection.
    ///
    /// For an antiparallel diode pair at a WDF port, we need to solve:
    ///   b = a - 2*Rp * Id(v)
    /// where v = (a + b) / 2 and Id(v) = Is*(exp(v/Vt) - exp(-v/Vt))
    ///
    /// This is an implicit equation solved via Newton's method.
    pub fn wave_up(&self, a: f32) -> f32 {
        let rp = self.rp;
        let is = self.is;
        let vt = self.vt;

        // Initial guess: voltage across diode ≈ clipped version of a/2
        let mut v = (a * 0.5).clamp(-2.0, 2.0);

        for _ in 0..self.nr_iters {
            // Diode current: Id = Is * (sinh(v/Vt)) for antiparallel pair
            // Using exp clamping to prevent overflow
            let v_norm = (v / vt).clamp(-30.0, 30.0);
            let exp_pos = v_norm.exp();
            let exp_neg = (-v_norm).exp();
            let sinh_val = 0.5 * (exp_pos - exp_neg);
            let cosh_val = 0.5 * (exp_pos + exp_neg);

            let i_d = 2.0 * is * sinh_val;
            let di_dv = 2.0 * is * cosh_val / vt;

            // f(v) = a - 2*v - 2*Rp*Id(v) = 0
            let f_val = a - 2.0 * v - 2.0 * rp * i_d;
            let f_prime = -2.0 - 2.0 * rp * di_dv;

            if f_prime.abs() < 1e-12 {
                break;
            }

            let delta = f_val / f_prime;
            v -= delta;

            // Convergence check
            if delta.abs() < 1e-6 {
                break;
            }
        }

        // Reflected wave: b = 2*v - a
        2.0 * v - a
    }
}

/// WDF Diode Clipper Circuit (with proper NR solver)
#[derive(Clone)]
pub struct WdfDiodeClipper {
    resistor: Resistor,
    capacitor: Capacitor,
    diodes: WdfDiodePair,
    p1: ParallelAdaptor,
}

impl WdfDiodeClipper {
    pub fn new(r: f32, c: f32, sample_rate: f32) -> Self {
        let resistor = Resistor::new(r);
        let capacitor = Capacitor::new(c, sample_rate);
        let mut diodes = WdfDiodePair::new(2.52e-9, 0.02585);
        let p1 = ParallelAdaptor::new(capacitor.port_resistance(), resistor.port_resistance());
        diodes.set_port_resistance(p1.port3_resistance());
        Self {
            resistor,
            capacitor,
            diodes,
            p1,
        }
    }

    pub fn process(&mut self, voltage_in: f32) -> f32 {
        let a1_p = self.capacitor.wave_up();
        let a2_p = voltage_in;

        // First pass: compute junction without diode
        let (_, _, b3_p) = self.p1.process(a1_p, a2_p, 0.0);

        // Diode pair reflects wave (NR-solved)
        let a3_p = self.diodes.wave_up(b3_p);

        // Second pass with diode contribution
        let (b1_p, b2_p, _) = self.p1.process(a1_p, a2_p, a3_p);

        self.capacitor.set_wave_down(b1_p);
        self.resistor.set_wave_down(b2_p);

        (a1_p + b1_p) * 0.5
    }
}
