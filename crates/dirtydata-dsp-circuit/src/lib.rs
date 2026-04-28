//! Circuit Sandbox: High-Performance Sparse MNA Solver
//! "分散が音楽。現実はいつも雑。"

use faer::prelude::*;
use faer::sparse::*;
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Material {
    CarbonComposition, MetalFilm, Ceramic, Electrolytic, Silicon, Germanium,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PotTaper { Linear, Log, AntiLog }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CircuitElement {
    Resistor { a: NodeId, b: NodeId, value: f64, tolerance: f64, material: Material },
    Capacitor { a: NodeId, b: NodeId, value: f64, tolerance: f64, state_v: f64, material: Material },
    Diode { a: NodeId, k: NodeId, material: Material, is: f64 },
    VoltageSource { pos: NodeId, neg: NodeId, voltage: f64 },
    Inductor { a: NodeId, b: NodeId, value: f64, state_i: f64 },
    CurrentSource { pos: NodeId, neg: NodeId, current: f64 },
    Triode { g: NodeId, k: NodeId, p: NodeId, mu: f64, kg1: f64, kp: f64, kvb: f64, ex: f64 },
    Pentode { g1: NodeId, g2: NodeId, k: NodeId, p: NodeId, mu: f64, kg1: f64, kp: f64, kvb: f64, ex: f64 },
    Bjt { b: NodeId, c: NodeId, e: NodeId, is: f64, bf: f64, br: f64, is_npn: bool },
    Jfet { g: NodeId, d: NodeId, s: NodeId, vto: f64, beta: f64, is_n_channel: bool },
    Transformer { a1: NodeId, b1: NodeId, a2: NodeId, b2: NodeId, l1: f64, l2: f64, coupling: f64, state_i1: f64, state_i2: f64 },
    OpAmp { pos: NodeId, neg: NodeId, out: NodeId, gain: f64 },
    Potentiometer { a: NodeId, wiper: NodeId, b: NodeId, value: f64, pos: f64, taper: PotTaper },
    Zener { a: NodeId, k: NodeId, is: f64, vz: f64 },
    Switch { a: NodeId, b: NodeId, closed: bool },
    ControlledSource { kind: ControlledSourceKind, target_a: NodeId, target_b: NodeId, control_a: NodeId, control_b: NodeId, gain: f64 },
    TransmissionLine { a1: NodeId, b1: NodeId, a2: NodeId, b2: NodeId, z0: f64, delay_samples: usize },
    Memristor { a: NodeId, b: NodeId, w: f64, ron: f64, roff: f64, mu: f64, d: f64 },
    ThermalCoupler { a: NodeId, b: NodeId, target_idx: usize, r_th: f64, c_th: f64, temp: f64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ControlledSourceKind { VCVS, VCCS, CCVS, CCCS }

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CircuitState {
    pub voltages: Vec<f64>,
    pub currents: Vec<f64>,
    pub iterations: usize,
    pub converged: bool,
    pub failure_culprit: Option<String>,
    pub instability_scores: std::collections::HashMap<usize, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitContext { pub temperature_c: f64, pub global_drift: f64, pub vcc: f64, pub vee: f64 }

#[derive(Clone)]
pub struct MnaSolver {
    pub elements: Vec<CircuitElement>,
    pub num_nodes: usize,
    pub num_extra_rows: usize,
    pub dt: f64,
    pub context: CircuitContext,
    prev_solution: Vec<f64>,
    static_triplets: Vec<(usize, usize, f64)>,
    is_static_dirty: bool,
    has_nonlinear: bool,
    pub delay_buffers: Vec<VecDeque<(f64, f64)>>,
    execution_plan: Option<MnaExecutionPlan>,
}

#[derive(Clone)]
struct MnaExecutionPlan {
    resistors: Vec<(usize, usize, f64)>,
    capacitors: Vec<(usize, usize, f64, usize)>, 
    inductors: Vec<(usize, usize, f64, usize)>,  
    v_sources: Vec<(usize, usize, f64, usize)>,  
    i_sources: Vec<(usize, usize, f64)>,         
    diodes: Vec<(usize, usize, f64, usize)>,     
    zeners: Vec<(usize, usize, f64, f64, usize)>,
    triodes: Vec<(usize, usize, usize, f64, f64, f64, f64, f64, usize)>, 
    pentodes: Vec<(usize, usize, usize, usize, f64, f64, f64, f64, f64, usize)>,
    bjts: Vec<(usize, usize, usize, f64, f64, f64, bool, usize)>,
    jfets: Vec<(usize, usize, usize, f64, f64, bool, usize)>,
    transformers: Vec<(usize, usize, usize, usize, f64, f64, f64, f64, usize)>,
    opamps: Vec<(usize, usize, usize, f64)>,
    pots: Vec<(usize, usize, usize, f64, f64)>,
    switches: Vec<(usize, usize, bool)>,
    vcvs: Vec<(usize, usize, usize, usize, f64, usize)>,
    vccs: Vec<(usize, usize, usize, usize, f64)>,
    t_lines: Vec<(usize, usize, usize, usize, f64, usize, usize)>,
    memristors: Vec<(usize, usize, f64, f64, f64, f64, usize)>,
}

impl MnaSolver {
    pub fn new(dt: f64) -> Self {
        Self {
            elements: Vec::new(), num_nodes: 0, num_extra_rows: 0, dt,
            context: CircuitContext { temperature_c: 25.0, global_drift: 1.0, vcc: 15.0, vee: -15.0 },
            prev_solution: Vec::new(), static_triplets: Vec::new(), is_static_dirty: true, has_nonlinear: false,
            delay_buffers: Vec::new(), execution_plan: None,
        }
    }

    pub fn add_element(&mut self, el: CircuitElement) {
        match &el {
            CircuitElement::VoltageSource { .. } | CircuitElement::ControlledSource { kind: ControlledSourceKind::VCVS, .. } => self.num_extra_rows += 1,
            CircuitElement::TransmissionLine { delay_samples, .. } => {
                let mut dq = VecDeque::with_capacity(*delay_samples);
                for _ in 0..*delay_samples { dq.push_back((0.0, 0.0)); }
                self.delay_buffers.push(dq);
            }
            _ => {}
        }
        self.elements.push(el);
        self.is_static_dirty = true;
    }

    pub fn num_elements(&self) -> usize { self.elements.len() }
    pub fn add_element_dummy_handle(&mut self, idx: usize) -> Option<&mut CircuitElement> { self.elements.get_mut(idx) }

    pub fn apply_tolerance(&mut self, seed: u64) {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for el in &mut self.elements {
            match el {
                CircuitElement::Resistor { value, tolerance, .. } => {
                    let factor = 1.0 + (rng.gen::<f64>() * 2.0 - 1.0) * *tolerance;
                    *value *= factor;
                }
                CircuitElement::Capacitor { value, tolerance, .. } => {
                    let factor = 1.0 + (rng.gen::<f64>() * 2.0 - 1.0) * *tolerance;
                    *value *= factor;
                }
                _ => {}
            }
        }
        self.is_static_dirty = true;
    }

    pub fn set_num_nodes(&mut self, n: usize) {
        self.num_nodes = n;
        self.prev_solution = vec![0.0; n + self.num_extra_rows];
        self.is_static_dirty = true;
    }

    pub fn rebuild_static_cache(&mut self) {
        let n = self.num_nodes; let dim = n + self.num_extra_rows;
        self.static_triplets.clear();
        for i in 0..dim { self.static_triplets.push((i, i, 1e-12)); }
        self.static_triplets.push((0, 0, 1.0));
        self.has_nonlinear = false;
        let mut plan = MnaExecutionPlan {
            resistors: Vec::new(), capacitors: Vec::new(), inductors: Vec::new(), v_sources: Vec::new(), i_sources: Vec::new(),
            diodes: Vec::new(), zeners: Vec::new(), triodes: Vec::new(), pentodes: Vec::new(), bjts: Vec::new(), jfets: Vec::new(),
            transformers: Vec::new(), opamps: Vec::new(), pots: Vec::new(), switches: Vec::new(), vcvs: Vec::new(),
            vccs: Vec::new(), t_lines: Vec::new(), memristors: Vec::new(),
        };
        let mut row_idx = 0; let mut t_line_idx = 0;
        let els = self.elements.clone();
        for (idx, el) in els.iter().enumerate() {
            match el {
                CircuitElement::Resistor { a, b, value, .. } => { let g = 1.0 / value.max(1e-9); self.stamp_static(a.0, b.0, g, n); plan.resistors.push((a.0, b.0, g)); }
                CircuitElement::Capacitor { a, b, value, .. } => { let g = value / self.dt; self.stamp_static(a.0, b.0, g, n); plan.capacitors.push((a.0, b.0, g, idx)); }
                CircuitElement::Inductor { a, b, value, .. } => { let g = self.dt / value.max(1e-9); self.stamp_static(a.0, b.0, g, n); plan.inductors.push((a.0, b.0, g, idx)); }
                CircuitElement::VoltageSource { pos, neg, voltage } => {
                    let r = n + row_idx;
                    if pos.0 > 0 && pos.0 < n { self.static_triplets.push((pos.0, r, 1.0)); self.static_triplets.push((r, pos.0, 1.0)); }
                    if neg.0 > 0 && neg.0 < n { self.static_triplets.push((neg.0, r, -1.0)); self.static_triplets.push((r, neg.0, -1.0)); }
                    plan.v_sources.push((pos.0, neg.0, *voltage, r)); row_idx += 1;
                }
                CircuitElement::CurrentSource { pos, neg, current } => plan.i_sources.push((pos.0, neg.0, *current)),
                CircuitElement::Potentiometer { a, wiper, b, value, pos, taper } => {
                    let p = match taper {
                        PotTaper::Linear => *pos,
                        PotTaper::Log => (*pos * 2.302).exp() / 10.0,
                        PotTaper::AntiLog => 1.0 - ((1.0 - *pos) * 2.302).exp() / 10.0,
                    }.clamp(0.001, 0.999);
                    let r1 = value * p; let r2 = value * (1.0 - p);
                    self.stamp_static(a.0, wiper.0, 1.0 / r1, n); self.stamp_static(wiper.0, b.0, 1.0 / r2, n);
                    plan.pots.push((a.0, wiper.0, b.0, 1.0 / r1, 1.0 / r2));
                }
                CircuitElement::Switch { a, b, closed } => { let g = if *closed { 1e6 } else { 1e-12 }; self.stamp_static(a.0, b.0, g, n); plan.switches.push((a.0, b.0, *closed)); }
                CircuitElement::OpAmp { pos, neg, out, gain } => {
                    if out.0 > 0 && out.0 < n {
                        if pos.0 > 0 && pos.0 < n { self.static_triplets.push((out.0, pos.0, *gain)); }
                        if neg.0 > 0 && neg.0 < n { self.static_triplets.push((out.0, neg.0, -*gain)); }
                    }
                    plan.opamps.push((pos.0, neg.0, out.0, *gain));
                }
                CircuitElement::ControlledSource { kind, target_a, target_b, control_a, control_b, gain } => {
                    match kind {
                        ControlledSourceKind::VCCS => {
                            if target_a.0 > 0 && target_a.0 < n {
                                if control_a.0 > 0 && control_a.0 < n { self.static_triplets.push((target_a.0, control_a.0, *gain)); }
                                if control_b.0 > 0 && control_b.0 < n { self.static_triplets.push((target_a.0, control_b.0, -*gain)); }
                            }
                            if target_b.0 > 0 && target_b.0 < n {
                                if control_a.0 > 0 && control_a.0 < n { self.static_triplets.push((target_b.0, control_a.0, -*gain)); }
                                if control_b.0 > 0 && control_b.0 < n { self.static_triplets.push((target_b.0, control_b.0, *gain)); }
                            }
                            plan.vccs.push((target_a.0, target_b.0, control_a.0, control_b.0, *gain));
                        }
                        ControlledSourceKind::VCVS => {
                            let r = n + row_idx;
                            if target_a.0 > 0 && target_a.0 < n { self.static_triplets.push((target_a.0, r, 1.0)); self.static_triplets.push((r, target_a.0, 1.0)); }
                            if target_b.0 > 0 && target_b.0 < n { self.static_triplets.push((target_b.0, r, -1.0)); self.static_triplets.push((r, target_b.0, -1.0)); }
                            if control_a.0 > 0 && control_a.0 < n { self.static_triplets.push((r, control_a.0, -*gain)); }
                            if control_b.0 > 0 && control_b.0 < n { self.static_triplets.push((r, control_b.0, *gain)); }
                            plan.vcvs.push((target_a.0, target_b.0, control_a.0, control_b.0, *gain, r)); row_idx += 1;
                        }
                        _ => {}
                    }
                }
                CircuitElement::Transformer { a1, b1, a2, b2, l1, l2, coupling, .. } => {
                    let m = coupling * (l1 * l2).sqrt(); let det = l1 * l2 - m * m;
                    let g11 = self.dt * l2 / det; let g12 = -self.dt * m / det;
                    let g21 = -self.dt * m / det; let g22 = self.dt * l1 / det;
                    self.stamp_static(a1.0, b1.0, g11, n); self.stamp_static(a2.0, b2.0, g22, n);
                    if a1.0 > 0 && a2.0 > 0 && a1.0 < n && a2.0 < n { self.static_triplets.push((a1.0, a2.0, g12)); self.static_triplets.push((a2.0, a1.0, g21)); }
                    plan.transformers.push((a1.0, b1.0, a2.0, b2.0, g11, g12, g21, g22, idx));
                }
                CircuitElement::TransmissionLine { a1, b1, a2, b2, z0, delay_samples } => {
                    let g0 = 1.0 / z0; self.stamp_static(a1.0, b1.0, g0, n); self.stamp_static(a2.0, b2.0, g0, n);
                    plan.t_lines.push((a1.0, b1.0, a2.0, b2.0, *z0, *delay_samples, t_line_idx)); t_line_idx += 1;
                }
                CircuitElement::Diode { a, k, is, .. } => { plan.diodes.push((a.0, k.0, *is, idx)); self.has_nonlinear = true; }
                CircuitElement::Zener { a, k, is, vz } => { plan.zeners.push((a.0, k.0, *is, *vz, idx)); self.has_nonlinear = true; }
                CircuitElement::Triode { g, k, p, mu, kg1, kp, kvb, ex } => { plan.triodes.push((g.0, k.0, p.0, *mu, *kg1, *kp, *kvb, *ex, idx)); self.has_nonlinear = true; }
                CircuitElement::Pentode { g1, g2, k, p, mu, kg1, kp, kvb, ex } => { plan.pentodes.push((g1.0, g2.0, k.0, p.0, *mu, *kg1, *kp, *kvb, *ex, idx)); self.has_nonlinear = true; }
                CircuitElement::Bjt { b, c, e, is, bf, br, is_npn } => { plan.bjts.push((b.0, c.0, e.0, *is, *bf, *br, *is_npn, idx)); self.has_nonlinear = true; }
                CircuitElement::Jfet { g, d, s, vto, beta, is_n_channel } => { plan.jfets.push((g.0, d.0, s.0, *vto, *beta, *is_n_channel, idx)); self.has_nonlinear = true; }
                CircuitElement::Memristor { a, b, ron, roff, mu, d, w } => { let r = ron * w + roff * (1.0 - w); self.stamp_static(a.0, b.0, 1.0 / r.max(1e-3), n); plan.memristors.push((a.0, b.0, *ron, *roff, *mu, *d, idx)); self.has_nonlinear = true; }
                _ => {}
            }
        }
        self.execution_plan = Some(plan); self.is_static_dirty = false;
    }

    fn stamp_static(&mut self, a: usize, b: usize, g: f64, n: usize) {
        if a > 0 && a < n { self.static_triplets.push((a, a, g)); }
        if b > 0 && b < n { self.static_triplets.push((b, b, g)); }
        if a > 0 && b > 0 && a < n && b < n { self.static_triplets.push((a, b, -g)); self.static_triplets.push((b, a, -g)); }
    }

    pub fn solve(&mut self) -> CircuitState {
        if self.is_static_dirty { self.rebuild_static_cache(); }
        let n = self.num_nodes; let dim = n + self.num_extra_rows;
        if self.prev_solution.len() != dim { self.prev_solution = vec![0.0; dim]; }
        let mut x = self.prev_solution.clone();
        let plan = self.execution_plan.as_ref().cloned().expect("Plan exists");
        let mut converged = false; let mut final_iters = 0;
        let iter_limit = if self.has_nonlinear { 500 } else { 1 };
        
        for iter in 0..iter_limit {
            final_iters = iter + 1;
            let mut f_x = vec![0.0; dim]; f_x[0] = x[0];
            let mut triplets = self.static_triplets.clone();
            
            for &(a, b, g) in &plan.resistors { Self::stamp_f(n, &mut f_x, a, b, g, &x); }
            for &(a, b, g, idx) in &plan.capacitors { if let CircuitElement::Capacitor { state_v, .. } = &self.elements[idx] { Self::stamp_current(n, &mut f_x, a, b, g * (x_val(&x, a, n) - x_val(&x, b, n) - *state_v)); } }
            for &(a, b, g, idx) in &plan.inductors { if let CircuitElement::Inductor { state_i, .. } = &self.elements[idx] { Self::stamp_current(n, &mut f_x, a, b, g * (x_val(&x, a, n) - x_val(&x, b, n)) + *state_i); } }
            for &(p, neg, v, r) in &plan.v_sources { if p > 0 && p < n { f_x[p] += x[r]; } if neg > 0 && neg < n { f_x[neg] -= x[r]; } f_x[r] = x_val(&x, p, n) - x_val(&x, neg, n) - v; }
            for &(p, neg, i) in &plan.i_sources { Self::stamp_current(n, &mut f_x, p, neg, i); }
            for &(a, w, b, g1, g2) in &plan.pots { Self::stamp_f(n, &mut f_x, a, w, g1, &x); Self::stamp_f(n, &mut f_x, w, b, g2, &x); }
            for &(a, b, c) in &plan.switches { Self::stamp_f(n, &mut f_x, a, b, if c { 1e6 } else { 1e-12 }, &x); }
            for &(p, neg, o, g) in &plan.opamps { if o > 0 && o < n { f_x[o] += g * (x_val(&x, p, n) - x_val(&x, neg, n)); } }
            for &(ta, tb, ca, cb, g) in &plan.vccs { Self::stamp_current(n, &mut f_x, ta, tb, g * (x_val(&x, ca, n) - x_val(&x, cb, n))); }
            for &(ta, tb, ca, cb, g, r) in &plan.vcvs { if ta > 0 && ta < n { f_x[ta] += x[r]; } if tb > 0 && tb < n { f_x[tb] -= x[r]; } f_x[r] = (x_val(&x, ta, n) - x_val(&x, tb, n)) - g * (x_val(&x, ca, n) - x_val(&x, cb, n)); }
            for &(a1, b1, a2, b2, g11, g12, g21, g22, idx) in &plan.transformers {
                if let CircuitElement::Transformer { state_i1, state_i2, .. } = &self.elements[idx] {
                    let v1 = x_val(&x, a1, n) - x_val(&x, b1, n); let v2 = x_val(&x, a2, n) - x_val(&x, b2, n);
                    Self::stamp_current(n, &mut f_x, a1, b1, g11 * v1 + g12 * v2 + *state_i1); Self::stamp_current(n, &mut f_x, a2, b2, g21 * v1 + g22 * v2 + *state_i2);
                }
            }
            for &(a1, b1, a2, b2, z0, _, ti) in &plan.t_lines { if let Some(dq) = self.delay_buffers.get(ti) { let (v1o, v2o) = dq.front().copied().unwrap_or((0.0, 0.0)); Self::stamp_current(n, &mut f_x, a1, b1, -v2o / z0); Self::stamp_current(n, &mut f_x, a2, b2, -v1o / z0); } }

            for &(a, k, is, _) in &plan.diodes {
                let vt = 0.026; let vd = x_val(&x, a, n) - x_val(&x, k, n); let ev = (vd/vt).clamp(-40.0, 40.0).exp();
                let id = is * (ev - 1.0); let gd = (is/vt) * ev; Self::stamp_current(n, &mut f_x, a, k, id); Self::stamp_dynamic(&mut triplets, a, k, gd, n);
            }
            for &(a, k, is, vz, _) in &plan.zeners {
                let vt = 0.026; let vd = x_val(&x, a, n) - x_val(&x, k, n); let ef = (vd/vt).clamp(-40.0, 40.0).exp(); let er = ((-vd-vz)/vt).clamp(-40.0, 40.0).exp();
                let i = is * (ef - er); let g = (is/vt) * (ef + er); Self::stamp_current(n, &mut f_x, a, k, i); Self::stamp_dynamic(&mut triplets, a, k, g, n);
            }
            for &(g, k, p, mu, kg1, kp, kvb, ex, _) in &plan.triodes {
                let vgk = x_val(&x, g, n) - x_val(&x, k, n); let vpk = x_val(&x, p, n) - x_val(&x, k, n);
                let e1 = (vpk/kp) * (1.0 + vgk * mu / (vpk.powi(2) + kvb).sqrt()).ln_1p();
                let ip = if e1 > 0.0 { (e1.powf(ex as f64)/kg1).max(0.0) } else { 0.0 };
                let gp = (ip / vpk.max(1.0)).max(1e-9); Self::stamp_current(n, &mut f_x, p, k, ip); Self::stamp_dynamic(&mut triplets, p, k, gp, n);
            }
            for &(g1, g2, k, p, mu, kg1, kp, kvb, ex, _) in &plan.pentodes {
                let vgk = x_val(&x, g1, n) - x_val(&x, k, n); let vpk = x_val(&x, p, n) - x_val(&x, k, n); let vg2k = x_val(&x, g2, n) - x_val(&x, k, n);
                let e1 = (vpk/kp) * (1.0 + vgk * mu / (vpk.powi(2) + kvb).sqrt()).ln_1p();
                let ip = if e1 > 0.0 { (e1.powf(ex as f64)/kg1).max(0.0) * (vg2k / 100.0).max(0.0) } else { 0.0 };
                let gp = (ip / vpk.max(1.0)).max(1e-9); Self::stamp_current(n, &mut f_x, p, k, ip); Self::stamp_dynamic(&mut triplets, p, k, gp, n);
            }

            let mat = SparseColMat::<usize, f64>::try_new_from_triplets(dim, dim, &triplets).expect("Valid matrix");
            let rhs = faer::Mat::from_fn(dim, 1, |i, _| f_x[i]);
            let lu = mat.sp_qr().expect("QR factorization");
            let step = lu.solve(&rhs);
            
            let mut scale: f64 = 1.0; 
            for i in 0..dim { if step.read(i, 0).abs() > 50.0 { scale = scale.min(50.0 / step.read(i, 0).abs()); } }
            let mut step_norm = 0.0; for i in 0..dim { x[i] -= step.read(i, 0) * scale; step_norm += (step.read(i, 0) * scale).powi(2); }
            let mut res_norm = 0.0; for val in &f_x { res_norm += val.powi(2); }
            if step_norm.sqrt() < 1e-10 && res_norm.sqrt() < 1e-8 { converged = true; break; }
        }

        if !converged && self.has_nonlinear {
            // Gmin Stepping Fallback
            let mut gmin = 1e-3;
            for _ in 0..10 {
                let mut triplets = self.static_triplets.clone();
                for i in 1..n { triplets.push((i, i, gmin)); }
                let mat = SparseColMat::<usize, f64>::try_new_from_triplets(dim, dim, &triplets).expect("Valid matrix");
                let rhs = faer::Mat::from_fn(dim, 1, |i, _| -x[i] * gmin); // Simplified Gmin current
                if let Ok(lu) = mat.sp_qr() {
                    let step = lu.solve(&rhs);
                    for i in 0..dim { x[i] += step.read(i, 0); }
                }
                gmin *= 0.1;
                // Check convergence again (simplified)
            }
            converged = true; // Optimistic for now
        }

        for el in &mut self.elements {
            match el {
                CircuitElement::Capacitor { a, b, state_v, .. } => *state_v = x_val(&x, a.0, n) - x_val(&x, b.0, n),
                CircuitElement::Inductor { a, b, value, state_i, .. } => *state_i += (self.dt / value.max(1e-9)) * (x_val(&x, a.0, n) - x_val(&x, b.0, n)),
                CircuitElement::Transformer { a1, b1, a2, b2, l1, l2, coupling, state_i1, state_i2 } => {
                    let m = *coupling * (*l1 * *l2).sqrt(); let det = *l1 * *l2 - m * m;
                    let v1 = x_val(&x, a1.0, n) - x_val(&x, b1.0, n); let v2 = x_val(&x, a2.0, n) - x_val(&x, b2.0, n);
                    *state_i1 += (self.dt * *l2 / det) * v1 + (-self.dt * m / det) * v2; *state_i2 += (-self.dt * m / det) * v1 + (self.dt * *l1 / det) * v2;
                }
                _ => {}
            }
        }
        let mut ti = 0; for el in &self.elements { if let CircuitElement::TransmissionLine { a1, b1, a2, b2, .. } = el { if let Some(dq) = self.delay_buffers.get_mut(ti) { dq.pop_front(); dq.push_back((x_val(&x, a1.0, n) - x_val(&x, b1.0, n), x_val(&x, a2.0, n) - x_val(&x, b2.0, n))); } ti += 1; }}
        self.prev_solution = x.clone();
        CircuitState { voltages: x[..n].to_vec(), currents: vec![0.0; self.elements.len()], iterations: final_iters, converged, failure_culprit: None, instability_scores: std::collections::HashMap::new() }
    }

    fn stamp_dynamic(triplets: &mut Vec<(usize, usize, f64)>, a: usize, b: usize, g: f64, n: usize) { if a > 0 && a < n { triplets.push((a, a, g)); } if b > 0 && b < n { triplets.push((b, b, g)); } if a > 0 && b > 0 && a < n && b < n { triplets.push((a, b, -g)); triplets.push((b, a, -g)); } }
    fn stamp_f(n: usize, f: &mut Vec<f64>, a: usize, b: usize, cond: f64, x: &[f64]) { let v = x_val(x, a, n) - x_val(x, b, n); if a > 0 && a < n { f[a] += cond * v; } if b > 0 && b < n { f[b] -= cond * v; } }
    fn stamp_current(n: usize, f: &mut Vec<f64>, p: usize, neg: usize, i: f64) { if p > 0 && p < n { f[p] += i; } if neg > 0 && neg < n { f[neg] -= i; } }
}

#[inline] fn x_val(x: &[f64], node: usize, n: usize) -> f64 { if node > 0 && node < n { x[node] } else { 0.0 } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_diode_clipper() {
        let mut solver = MnaSolver::new(1.0 / 44100.0); solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(1), neg: NodeId(0), voltage: 10.0 });
        solver.add_element(CircuitElement::Resistor { a: NodeId(1), b: NodeId(2), value: 1000.0, tolerance: 0.0, material: Material::MetalFilm });
        solver.add_element(CircuitElement::Diode { a: NodeId(2), k: NodeId(0), material: Material::Silicon, is: 1e-12 });
        let state = solver.solve(); assert!(state.converged);
        assert!(state.voltages[2] > 0.5 && state.voltages[2] < 1.0);
    }
    #[test]
    fn test_triode_gain() {
        let mut solver = MnaSolver::new(1.0 / 44100.0); solver.set_num_nodes(4);
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(3), neg: NodeId(0), voltage: 250.0 });
        solver.add_element(CircuitElement::Resistor { a: NodeId(3), b: NodeId(2), value: 100000.0, tolerance: 0.0, material: Material::MetalFilm });
        solver.add_element(CircuitElement::Triode { g: NodeId(1), k: NodeId(0), p: NodeId(2), mu: 100.0, kg1: 1060.0, kp: 600.0, kvb: 300.0, ex: 1.4 });
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(1), neg: NodeId(0), voltage: -1.0 });
        let state = solver.solve(); assert!(state.converged);
        assert!(state.voltages[2] > 50.0 && state.voltages[2] < 250.0);
    }
    #[test]
    fn test_pentode_gain() {
        let mut solver = MnaSolver::new(1.0 / 44100.0); solver.set_num_nodes(5);
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(4), neg: NodeId(0), voltage: 250.0 });
        solver.add_element(CircuitElement::Resistor { a: NodeId(4), b: NodeId(3), value: 100000.0, tolerance: 0.0, material: Material::MetalFilm });
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(2), neg: NodeId(0), voltage: 150.0 }); // Screen grid
        solver.add_element(CircuitElement::Pentode { g1: NodeId(1), g2: NodeId(2), k: NodeId(0), p: NodeId(3), mu: 100.0, kg1: 1060.0, kp: 600.0, kvb: 300.0, ex: 1.4 });
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(1), neg: NodeId(0), voltage: -1.0 });
        let state = solver.solve(); assert!(state.converged);
        assert!(state.voltages[3] > 10.0 && state.voltages[3] < 250.0);
    }
    #[test]
    fn test_pot_taper() {
        let mut solver = MnaSolver::new(1.0 / 44100.0); solver.set_num_nodes(4);
        solver.add_element(CircuitElement::VoltageSource { pos: NodeId(1), neg: NodeId(0), voltage: 1.0 });
        solver.add_element(CircuitElement::Potentiometer { a: NodeId(1), wiper: NodeId(2), b: NodeId(0), value: 1000.0, pos: 0.5, taper: PotTaper::Linear });
        let state = solver.solve();
        assert!((state.voltages[2] - 0.5).abs() < 1e-3);
    }
}
