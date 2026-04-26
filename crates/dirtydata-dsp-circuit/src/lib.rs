//! Circuit Sandbox: High-Performance MNA (Modified Nodal Analysis) Solver
//! "分散が音楽。現実はいつも雑。"

use nalgebra::{DMatrix, DVector};
use serde::{Serialize, Deserialize};
use rand::Rng;
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Material {
    CarbonComposition, // Noise & Drift
    MetalFilm,         // Precision
    Ceramic,           // Nonlinear capacitance
    Electrolytic,      // High ESR
    Silicon,
    Germanium,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CircuitElement {
    Resistor { 
        a: NodeId, b: NodeId, 
        value: f64, 
        tolerance: f64,
        material: Material 
    },
    Capacitor { 
        a: NodeId, b: NodeId, 
        value: f64, 
        tolerance: f64,
        state_v: f64,
        material: Material 
    },
    Diode { 
        a: NodeId, k: NodeId, 
        material: Material,
        is: f64,
    },
    VoltageSource { pos: NodeId, neg: NodeId, voltage: f64 },
}

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
pub struct CircuitContext {
    pub temperature_c: f64,
    pub global_drift: f64,
    pub vcc: f64,
    pub vee: f64,
}

pub struct MnaSolver {
    pub elements: Vec<CircuitElement>,
    pub num_nodes: usize,
    pub num_v_sources: usize,
    pub dt: f64,
    pub context: CircuitContext,
    prev_solution: DVector<f64>,
    static_jacobian: DMatrix<f64>,
    static_f: DVector<f64>,
    is_static_dirty: bool,
    has_nonlinear: bool,
    static_lu: Option<nalgebra::LU<f64, nalgebra::Dyn, nalgebra::Dyn>>,
    /// Tracks instability of individual circuit elements for "Explain Mode".
    pub instability_scores: std::collections::HashMap<usize, f32>,
}

impl MnaSolver {
    pub fn new(dt: f64) -> Self {
        let dim = 0;
        Self {
            elements: Vec::new(),
            num_nodes: 0,
            num_v_sources: 0,
            dt,
            context: CircuitContext { temperature_c: 25.0, global_drift: 1.0, vcc: 15.0, vee: -15.0 },
            prev_solution: DVector::from_element(dim, 0.0),
            static_jacobian: DMatrix::from_element(dim, dim, 0.0),
            static_f: DVector::from_element(dim, 0.0),
            is_static_dirty: true,
            has_nonlinear: false,
            static_lu: None,
            instability_scores: std::collections::HashMap::new(),
        }
    }

    pub fn num_elements(&self) -> usize { self.elements.len() }
    pub fn add_element(&mut self, el: CircuitElement) {
        if let CircuitElement::VoltageSource { .. } = &el { self.num_v_sources += 1; }
        self.elements.push(el);
        self.is_static_dirty = true;
    }
    pub fn add_element_dummy_handle(&mut self, idx: usize) -> Option<&mut CircuitElement> { 
        self.is_static_dirty = true;
        self.elements.get_mut(idx) 
    }
    pub fn set_num_nodes(&mut self, n: usize) {
        self.num_nodes = n;
        let dim = n + self.num_v_sources;
        self.prev_solution = DVector::from_element(dim, 0.0);
        self.static_jacobian = DMatrix::from_element(dim, dim, 0.0);
        self.static_f = DVector::from_element(dim, 0.0);
        self.is_static_dirty = true;
    }

    pub fn apply_tolerance(&mut self, seed: u64) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for el in &mut self.elements {
            match el {
                CircuitElement::Resistor { value, tolerance, .. } => {
                    *value *= rng.gen_range((1.0 - *tolerance)..(1.0 + *tolerance));
                }
                CircuitElement::Capacitor { value, tolerance, .. } => {
                    *value *= rng.gen_range((1.0 - *tolerance)..(1.0 + *tolerance));
                }
                _ => {}
            }
        }
    }

    /// Identifies independent islands of nodes in the circuit.
    /// Node 0 is treated as a reference and does not link islands together.
    pub fn find_islands(&self) -> Vec<HashSet<NodeId>> {
        let mut islands = Vec::new();
        let mut visited = HashSet::new();
        
        // Skip Node 0 as a starting point, it's the global reference
        for i in 1..self.num_nodes {
            let start_node = NodeId(i);
            if visited.contains(&start_node) { continue; }
            
            let mut island = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(start_node);
            
            while let Some(curr) = queue.pop_front() {
                if island.insert(curr) {
                    visited.insert(curr);
                    // Find nodes connected to curr
                    for el in &self.elements {
                        match el {
                            CircuitElement::Resistor { a, b, .. } |
                            CircuitElement::Capacitor { a, b, .. } |
                            CircuitElement::Diode { a, k: b, .. } |
                            CircuitElement::VoltageSource { pos: a, neg: b, .. } => {
                                if a.0 == curr.0 && b.0 != 0 { queue.push_back(*b); }
                                if b.0 == curr.0 && a.0 != 0 { queue.push_back(*a); }
                            }
                        }
                    }
                }
            }
            if !island.is_empty() {
                islands.push(island);
            }
        }
        islands
    }

    pub fn rebuild_static_cache(&mut self) {
        let n = self.num_nodes;
        let m = self.num_v_sources;
        let dim = n + m;
        self.static_jacobian = DMatrix::from_element(dim, dim, 0.0);
        self.static_f = DVector::from_element(dim, 0.0);
        self.has_nonlinear = false;
        
        let mut v_idx = 0;
        for el in &self.elements {
            match el {
                CircuitElement::Resistor { a, b, value, .. } => {
                    let g = 1.0 / value.max(1e-9);
                    Self::stamp_jacobian(n, &mut self.static_jacobian, a.0, b.0, g);
                }
                CircuitElement::Capacitor { a, b, value, .. } => {
                    let g = value / self.dt;
                    Self::stamp_jacobian(n, &mut self.static_jacobian, a.0, b.0, g);
                }
                CircuitElement::VoltageSource { pos, neg, .. } => {
                    let idx = n + v_idx;
                    if pos.0 < n { 
                        self.static_jacobian[(pos.0, idx)] += 1.0; 
                        self.static_jacobian[(idx, pos.0)] += 1.0; 
                    }
                    if neg.0 < n { 
                        self.static_jacobian[(neg.0, idx)] -= 1.0; 
                        self.static_jacobian[(idx, neg.0)] -= 1.0; 
                    }
                    v_idx += 1;
                }
                CircuitElement::Diode { .. } => {
                    self.has_nonlinear = true;
                }
            }
        }
        for i in 0..n { self.static_jacobian[(i, i)] += 1e-12; }
        
        if !self.has_nonlinear {
            self.static_lu = Some(self.static_jacobian.clone().lu());
        } else {
            self.static_lu = None;
        }
        
        self.is_static_dirty = false;
    }

    pub fn solve(&mut self) -> CircuitState {
        if self.is_static_dirty { self.rebuild_static_cache(); }
        
        let n = self.num_nodes;
        let m = self.num_v_sources;
        let dim = n + m;
        if self.prev_solution.len() != dim { self.prev_solution = DVector::from_element(dim, 0.0); }
        let mut x = self.prev_solution.clone();
        
        let mut converged = false;
        let mut final_iters = 0;

        let iter_limit = if self.has_nonlinear { 40 } else { 1 };

        self.instability_scores.clear();

        for iter in 0..iter_limit {
            final_iters = iter + 1;
            let mut f_x = DVector::from_element(dim, 0.0);
            let mut v_idx = 0;
            
            // Re-evaluate elements for F vector
            for (idx, el) in self.elements.iter().enumerate() {
                let start_f = f_x.clone();
                match el {
                    CircuitElement::Resistor { a, b, value, .. } => {
                        let g = 1.0 / value.max(1e-9);
                        Self::stamp_f(n, &mut f_x, a.0, b.0, g, &x);
                    }
                    CircuitElement::Capacitor { a, b, value, state_v, .. } => {
                        let g = value / self.dt;
                        let i_eq = g * (x_val(&x, a.0, n) - x_val(&x, b.0, n) - *state_v);
                        Self::stamp_current(n, &mut f_x, a.0, b.0, i_eq);
                    }
                    CircuitElement::VoltageSource { pos, neg, voltage } => {
                        let idx = n + v_idx;
                        let v_pos = x_val(&x, pos.0, n);
                        let v_neg = x_val(&x, neg.0, n);
                        if pos.0 < n { f_x[pos.0] += x[idx]; }
                        if neg.0 < n { f_x[neg.0] -= x[idx]; }
                        f_x[idx] = v_pos - v_neg - *voltage;
                        v_idx += 1;
                    }
                    CircuitElement::Diode { a, k, is, .. } => {
                        let vt = 0.026 * (self.context.temperature_c + 273.15) / 298.15;
                        let v_d = x_val(&x, a.0, n) - x_val(&x, k.0, n);
                        let exp_v = (v_d / vt).clamp(-40.0, 40.0).exp();
                        let i_d = is * (exp_v - 1.0);
                        let g_d = (is / vt) * exp_v;
                        Self::stamp_current(n, &mut f_x, a.0, k.0, i_d - g_d * v_d);
                    }
                }

                if iter > 20 {
                    let diff = (&f_x - &start_f).norm();
                    if diff > 1e-3 {
                        *self.instability_scores.entry(idx).or_insert(0.0) += diff as f32;
                    }
                }
            }

            let step = if !self.has_nonlinear {
                self.static_lu.as_ref().and_then(|lu| lu.solve(&f_x))
            } else {
                let mut jacobian = self.static_jacobian.clone();
                for el in &self.elements {
                    if let CircuitElement::Diode { a, k, is, .. } = el {
                        let vt = 0.026 * (self.context.temperature_c + 273.15) / 298.15;
                        let v_d = x_val(&x, a.0, n) - x_val(&x, k.0, n);
                        let exp_v = (v_d / vt).clamp(-40.0, 40.0).exp();
                        let g_d = (*is / vt) * exp_v;
                        Self::stamp_jacobian(n, &mut jacobian, a.0, k.0, g_d);
                    }
                }
                jacobian.lu().solve(&f_x)
            };

            if let Some(s) = step {
                x -= s.clone();
                if s.norm() < 1e-7 { converged = true; break; }
            } else { break; }
        }

        for el in &mut self.elements {
            if let CircuitElement::Capacitor { a, b, state_v, .. } = el {
                *state_v = x_val(&x, a.0, n) - x_val(&x, b.0, n);
            }
        }

        let culprit = if !converged {
            let mut max_v = 0.0;
            let mut max_idx = 0;
            for (i, &v) in x.as_slice().iter().take(n).enumerate() {
                if v.is_nan() {
                    max_idx = i;
                    break;
                }
                if v.abs() > max_v {
                    max_v = v.abs();
                    max_idx = i;
                }
            }
            Some(format!("Node {} unstable (V={:.2e})", max_idx, x[max_idx]))
        } else {
            None
        };

        self.prev_solution = x.clone();
        CircuitState {
            voltages: x.as_slice().iter().take(n).copied().collect(),
            currents: vec![0.0; self.elements.len()],
            iterations: final_iters,
            converged,
            failure_culprit: culprit,
            instability_scores: self.instability_scores.clone(),
        }
    }

    fn stamp_jacobian(n: usize, g: &mut DMatrix<f64>, a: usize, b: usize, cond: f64) {
        if a < n { g[(a, a)] += cond; }
        if b < n { g[(b, b)] += cond; }
        if a < n && b < n { g[(a, b)] -= cond; g[(b, a)] -= cond; }
    }
    fn stamp_f(n: usize, f: &mut DVector<f64>, a: usize, b: usize, cond: f64, x: &DVector<f64>) {
        let v_diff = x_val(x, a, n) - x_val(x, b, n);
        let cur = cond * v_diff;
        if a < n { f[a] += cur; }
        if b < n { f[b] -= cur; }
    }
    fn stamp_current(n: usize, f: &mut DVector<f64>, pos: usize, neg: usize, current: f64) {
        if pos < n { f[pos] += current; }
        if neg < n { f[neg] -= current; }
    }
}

#[inline]
fn x_val(x: &DVector<f64>, node: usize, num_nodes: usize) -> f64 {
    if node < num_nodes { x[node] } else { 0.0 }
}
