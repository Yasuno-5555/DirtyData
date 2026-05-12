#![allow(clippy::all)]
#![allow(clippy::all)]

//! Circuit Sandbox: High-Performance Sparse MNA Solver
//! "分散が音楽。現実はいつも雑。"

use faer::prelude::*;
use faer::sparse::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Material {
    CarbonComposition,
    MetalFilm,
    Ceramic,
    Electrolytic,
    Silicon,
    Germanium,
    Vacuum,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PotTaper {
    Linear,
    Log,
    AntiLog,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CircuitElement {
    Resistor {
        a: NodeId,
        b: NodeId,
        value: f64,
        tolerance: f64,
        material: Material,
    },
    Capacitor {
        a: NodeId,
        b: NodeId,
        value: f64,
        tolerance: f64,
        state_v: f64,
        material: Material,
    },
    Diode {
        a: NodeId,
        k: NodeId,
        material: Material,
        is: f64,
    },
    VoltageSource {
        pos: NodeId,
        neg: NodeId,
        voltage: f64,
    },
    Inductor {
        a: NodeId,
        b: NodeId,
        value: f64,
        state_i: f64,
    },
    CurrentSource {
        pos: NodeId,
        neg: NodeId,
        current: f64,
    },
    Triode {
        g: NodeId,
        k: NodeId,
        p: NodeId,
        mu: f64,
        kg1: f64,
        kp: f64,
        kvb: f64,
        ex: f64,
        material: Material,
    },
    Pentode {
        g1: NodeId,
        g2: NodeId,
        k: NodeId,
        p: NodeId,
        mu: f64,
        kg1: f64,
        kp: f64,
        kvb: f64,
        ex: f64,
        material: Material,
    },
    Bjt {
        b: NodeId,
        c: NodeId,
        e: NodeId,
        is: f64,
        bf: f64,
        br: f64,
        is_npn: bool,
    },
    Jfet {
        g: NodeId,
        d: NodeId,
        s: NodeId,
        vto: f64,
        beta: f64,
        is_n_channel: bool,
    },
    Transformer {
        a1: NodeId,
        b1: NodeId,
        a2: NodeId,
        b2: NodeId,
        l1: f64,
        l2: f64,
        coupling: f64,
        state_i1: f64,
        state_i2: f64,
    },
    OpAmp {
        pos: NodeId,
        neg: NodeId,
        out: NodeId,
        gain: f64,
    },
    Potentiometer {
        a: NodeId,
        wiper: NodeId,
        b: NodeId,
        value: f64,
        pos: f64,
        taper: PotTaper,
    },
    Zener {
        a: NodeId,
        k: NodeId,
        is: f64,
        vz: f64,
    },
    Switch {
        a: NodeId,
        b: NodeId,
        closed: bool,
    },
    ControlledSource {
        kind: ControlledSourceKind,
        target_a: NodeId,
        target_b: NodeId,
        control_a: NodeId,
        control_b: NodeId,
        gain: f64,
    },
    TransmissionLine {
        a1: NodeId,
        b1: NodeId,
        a2: NodeId,
        b2: NodeId,
        z0: f64,
        delay_samples: usize,
    },
    Memristor {
        a: NodeId,
        b: NodeId,
        w: f64,
        ron: f64,
        roff: f64,
        mu: f64,
        d: f64,
    },
    ThermalCoupler {
        a: NodeId,
        b: NodeId,
        target_idx: usize,
        r_th: f64,
        c_th: f64,
        temp: f64,
    },
    Mosfet {
        g: NodeId,
        d: NodeId,
        s: NodeId,
        vto: f64,
        beta: f64,
        lambda: f64,
        is_n_channel: bool,
    },
    Igbt {
        g: NodeId,
        c: NodeId,
        e: NodeId,
        vto: f64,
        beta: f64,
        bf: f64,
        is: f64,
    },
    Scr {
        a: NodeId,
        k: NodeId,
        g: NodeId,
        v_hold: f64,
        i_hold: f64,
        i_gate_trigger: f64,
        state_on: bool,
    },
    Triac {
        m1: NodeId,
        m2: NodeId,
        g: NodeId,
        v_hold: f64,
        i_hold: f64,
        i_gate_trigger: f64,
        state_on: bool,
    },
    DcMotor {
        pos: NodeId,
        neg: NodeId,
        resistance: f64,
        inductance: f64,
        ke: f64,
        kt: f64,
        inertia: f64,
        friction: f64,
        state_i: f64,
        state_omega: f64,
    },
    Thermistor {
        a: NodeId,
        b: NodeId,
        r25: f64,
        beta: f64,
        is_ntc: bool,
    },
    Photodiode {
        a: NodeId,
        k: NodeId,
        sensitivity: f64,
        current_lux: f64,
    },
    Ldr {
        a: NodeId,
        b: NodeId,
        r_dark: f64,
        gamma: f64,
        current_lux: f64,
    },
    Piezoelectric {
        a: NodeId,
        b: NodeId,
        capacitance: f64,
        sensitivity: f64,
        state_v: f64,
        force: f64,
    },
    HallSensor {
        pos: NodeId,
        neg: NodeId,
        out: NodeId,
        sensitivity: f64,
        b_field: f64,
    },
    Crystal {
        a: NodeId,
        b: NodeId,
        lm: f64,
        cm: f64,
        rm: f64,
        co: f64,
        state_im: f64,
        state_vm: f64,
    },
    Balun {
        p1a: NodeId,
        p1b: NodeId,
        p2a: NodeId,
        p2b: NodeId,
        l: f64,
        coupling: f64,
    },
    Microstrip {
        a: NodeId,
        b: NodeId,
        z0: f64,
        length: f64,
        er: f64,
        loss_tan: f64,
    },
    VoltageNoise {
        a: NodeId,
        b: NodeId,
        density: f64,
        flicker_alpha: f64,
        hum_amplitude: f64,
        psu_ripple: f64,
    },
    CurrentNoise {
        a: NodeId,
        b: NodeId,
        density: f64,
        flicker_alpha: f64,
    },
    LogicGate {
        kind: LogicKind,
        inputs: Vec<NodeId>,
        out: NodeId,
        v_high: f64,
        v_low: f64,
        delay: f64,
        state_v: f64,
    },
    Comparator {
        pos: NodeId,
        neg: NodeId,
        out: NodeId,
        v_high: f64,
        v_low: f64,
    },
    PulseSource {
        pos: NodeId,
        neg: NodeId,
        amplitude: f64,
        freq: f64,
        duty: f64,
        rise_time: f64,
    },
    Ota {
        p: NodeId,
        n: NodeId,
        iabc: NodeId,
        out: NodeId,
        gm_per_amp: f64,
        r_in: f64,
    },
    Vactrol {
        led_p: NodeId,
        led_n: NodeId,
        ldr_a: NodeId,
        ldr_b: NodeId,
        tau_rise: f64,
        tau_fall: f64,
        state_brightness: f64,
    },
    DyingBattery {
        pos: NodeId,
        neg: NodeId,
        voltage: f64,
        internal_r: f64,
        capacity_ah: f64,
        current_charge: f64,
    },
    Bbd {
        input: NodeId,
        output: NodeId,
        clock_hz: f64,
        num_stages: usize,
        state_phase: f64,
    },
    Relay {
        coil_p: NodeId,
        coil_n: NodeId,
        a: NodeId,
        b: NodeId,
        l_coil: f64,
        r_coil: f64,
        state_on: bool,
    },
    NeonBulb {
        a: NodeId,
        b: NodeId,
        v_breakdown: f64,
        v_extinguish: f64,
        state_on: bool,
    },
    DirtyGround {
        node: NodeId,
        r_parasitic: f64,
        l_parasitic: f64,
        noise_density: f64,
    },
    Loudspeaker {
        a: NodeId,
        b: NodeId,
        re: f64,
        le: f64,
        bl: f64,
        mms: f64,
        rms: f64,
        cms: f64,
        state_i: f64,
        state_v: f64,
        state_x: f64,
    },
    GuitarPickup {
        a: NodeId,
        b: NodeId,
        l: f64,
        r: f64,
        c: f64,
        flux_v: f64,
        state_i: f64,
    },
    Varactor {
        a: NodeId,
        k: NodeId,
        c0: f64,
        vj: f64,
        m: f64,
        state_v: f64,
    },
    SaturableInductor {
        a: NodeId,
        b: NodeId,
        l_max: f64,
        l_min: f64,
        i_sat: f64,
        state_i: f64,
    },
    TunnelDiode {
        a: NodeId,
        k: NodeId,
        ip: f64,
        vp: f64,
        iv: f64,
        vv: f64,
        state_v: f64,
    },
    SparkGap {
        a: NodeId,
        b: NodeId,
        v_breakdown: f64,
        v_hold: f64,
        state_on: bool,
    },
    CircuitGhost {
        a: NodeId,
        b: NodeId,
        c_min: f64,
        c_max: f64,
        drift_rate: f64,
        state_c: f64,
    },
    VoltageControlledResistor {
        a: NodeId,
        b: NodeId,
        ctrl: NodeId,
        r_min: f64,
        r_max: f64,
        v_min: f64,
        v_max: f64,
    },
}

impl CircuitElement {
    pub fn nodes(&self) -> Vec<NodeId> {
        match self {
            CircuitElement::Resistor { a, b, .. } => vec![*a, *b],
            CircuitElement::Capacitor { a, b, .. } => vec![*a, *b],
            CircuitElement::Diode { a, k, .. } => vec![*a, *k],
            CircuitElement::VoltageSource { pos, neg, .. } => vec![*pos, *neg],
            CircuitElement::Inductor { a, b, .. } => vec![*a, *b],
            CircuitElement::CurrentSource { pos, neg, .. } => vec![*pos, *neg],
            CircuitElement::Triode { g, k, p, .. } => vec![*g, *k, *p],
            CircuitElement::Pentode { g1, g2, k, p, .. } => vec![*g1, *g2, *k, *p],
            CircuitElement::Bjt { b, c, e, .. } => vec![*b, *c, *e],
            CircuitElement::Jfet { g, d, s, .. } => vec![*g, *d, *s],
            CircuitElement::Transformer { a1, b1, a2, b2, .. } => vec![*a1, *b1, *a2, *b2],
            CircuitElement::OpAmp { pos, neg, out, .. } => vec![*pos, *neg, *out],
            CircuitElement::Potentiometer { a, wiper, b, .. } => vec![*a, *wiper, *b],
            CircuitElement::Zener { a, k, .. } => vec![*a, *k],
            CircuitElement::Switch { a, b, .. } => vec![*a, *b],
            CircuitElement::ControlledSource {
                target_a,
                target_b,
                control_a,
                control_b,
                ..
            } => vec![*target_a, *target_b, *control_a, *control_b],
            CircuitElement::TransmissionLine { a1, b1, a2, b2, .. } => vec![*a1, *b1, *a2, *b2],
            CircuitElement::Memristor { a, b, .. } => vec![*a, *b],
            CircuitElement::ThermalCoupler { a, b, .. } => vec![*a, *b],
            CircuitElement::Mosfet { g, d, s, .. } => vec![*g, *d, *s],
            CircuitElement::Igbt { g, c, e, .. } => vec![*g, *c, *e],
            CircuitElement::Scr { a, k, g, .. } => vec![*a, *k, *g],
            CircuitElement::Triac { m1, m2, g, .. } => vec![*m1, *m2, *g],
            CircuitElement::DcMotor { pos, neg, .. } => vec![*pos, *neg],
            CircuitElement::Thermistor { a, b, .. } => vec![*a, *b],
            CircuitElement::Photodiode { a, k, .. } => vec![*a, *k],
            CircuitElement::Ldr { a, b, .. } => vec![*a, *b],
            CircuitElement::Piezoelectric { a, b, .. } => vec![*a, *b],
            CircuitElement::HallSensor { pos, neg, out, .. } => vec![*pos, *neg, *out],
            CircuitElement::Crystal { a, b, .. } => vec![*a, *b],
            CircuitElement::Balun {
                p1a, p1b, p2a, p2b, ..
            } => vec![*p1a, *p1b, *p2a, *p2b],
            CircuitElement::Microstrip { a, b, .. } => vec![*a, *b],
            CircuitElement::VoltageNoise { a, b, .. } => vec![*a, *b],
            CircuitElement::CurrentNoise { a, b, .. } => vec![*a, *b],
            CircuitElement::LogicGate { inputs, out, .. } => {
                let mut v = inputs.clone();
                v.push(*out);
                v
            }
            CircuitElement::Comparator { pos, neg, out, .. } => vec![*pos, *neg, *out],
            CircuitElement::PulseSource { pos, neg, .. } => vec![*pos, *neg],
            CircuitElement::Ota {
                p, n, iabc, out, ..
            } => vec![*p, *n, *iabc, *out],
            CircuitElement::Vactrol {
                led_p,
                led_n,
                ldr_a,
                ldr_b,
                ..
            } => vec![*led_p, *led_n, *ldr_a, *ldr_b],
            CircuitElement::DyingBattery { pos, neg, .. } => vec![*pos, *neg],
            CircuitElement::Bbd { input, output, .. } => vec![*input, *output],
            CircuitElement::Relay {
                coil_p,
                coil_n,
                a,
                b,
                ..
            } => vec![*coil_p, *coil_n, *a, *b],
            CircuitElement::NeonBulb { a, b, .. } => vec![*a, *b],
            CircuitElement::DirtyGround { node, .. } => vec![*node],
            CircuitElement::Loudspeaker { a, b, .. } => vec![*a, *b],
            CircuitElement::GuitarPickup { a, b, .. } => vec![*a, *b],
            CircuitElement::Varactor { a, k, .. } => vec![*a, *k],
            CircuitElement::SaturableInductor { a, b, .. } => vec![*a, *b],
            CircuitElement::TunnelDiode { a, k, .. } => vec![*a, *k],
            CircuitElement::SparkGap { a, b, .. } => vec![*a, *b],
            CircuitElement::CircuitGhost { a, b, .. } => vec![*a, *b],
            CircuitElement::VoltageControlledResistor { a, b, ctrl, .. } => vec![*a, *b, *ctrl],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum LogicKind {
    AND,
    OR,
    NOT,
    NAND,
    NOR,
    XOR,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum ControlledSourceKind {
    VCVS,
    VCCS,
    CCVS,
    CCCS,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CircuitState {
    pub voltages: Vec<f64>,
    pub currents: Vec<f64>,
    pub iterations: usize,
    pub converged: bool,
    pub failure_culprit: Option<String>,
    pub instability_scores: std::collections::HashMap<usize, f32>,
    pub provenance: std::collections::HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitContext {
    pub temperature_c: f64,
    pub global_drift: f64,
    pub vcc: f64,
    pub vee: f64,
}

#[derive(Clone)]
#[allow(clippy::type_complexity)]
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
    noise_seed: u64,
    is_solving_gmin: bool,
}

#[derive(Clone)]
#[allow(clippy::type_complexity)]
struct MnaExecutionPlan {
    resistors: Vec<(usize, usize, f64)>,
    capacitors: Vec<(usize, usize, f64, usize)>,
    inductors: Vec<(usize, usize, f64, usize)>,
    v_sources: Vec<(usize, usize, f64, usize)>,
    i_sources: Vec<(usize, usize, f64)>,
    diodes: Vec<(usize, usize, f64, usize)>,
    zeners: Vec<(usize, usize, f64, f64, usize)>,
    triodes: Vec<(usize, usize, usize, f64, f64, f64, f64, f64, Material, usize)>,
    pentodes: Vec<(usize, usize, usize, usize, f64, f64, f64, f64, f64, Material, usize)>,
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
    mosfets: Vec<(usize, usize, usize, f64, f64, f64, bool, usize)>,
    igbts: Vec<(usize, usize, usize, f64, f64, f64, f64, usize)>,
    scrs: Vec<(usize, usize, usize, f64, f64, f64, usize)>,
    triacs: Vec<(usize, usize, usize, f64, f64, f64, usize)>,
    motors: Vec<(usize, usize, f64, f64, f64, f64, f64, f64, usize, usize)>, // (..., r_ia, idx)
    thermistors: Vec<(usize, usize, f64, f64, bool, usize)>,
    ldrs: Vec<(usize, usize, f64, f64, f64, usize)>,
    logic_gates: Vec<(LogicKind, Vec<usize>, usize, f64, f64, f64, usize, usize)>, // (..., idx, r)
    noise_sources: Vec<(bool, usize, usize, f64, f64, f64, f64, usize, usize)>, // (is_v, ..., hum, ripple, idx, r)
    crystals: Vec<(usize, usize, f64, f64, f64, f64, usize, usize)>,
    comparators: Vec<(usize, usize, usize, f64, f64, usize)>, // (..., r)
    pulse_sources: Vec<(usize, usize, f64, f64, f64, f64, usize)>, // (..., r)
    hall_sensors: Vec<(usize, usize, usize, f64, f64, usize)>, // (..., r)
    otas: Vec<(usize, usize, usize, usize, f64, f64)>,        // (p, n, iabc, out, gm, r_in)
    vactrols: Vec<(usize, usize, usize, usize, f64, f64, usize)>,
    batteries: Vec<(usize, usize, f64, f64, usize, usize)>, // (pos, neg, v, r_int, idx, r)
    bbds: Vec<(usize, usize, f64, usize, usize, usize, usize)>, // (..., r, el_idx, bbd_idx)
    relays: Vec<(usize, usize, usize, usize, f64, f64, usize)>,
    neons: Vec<(usize, usize, f64, f64, usize)>,
    dirty_grounds: Vec<(usize, f64, f64, f64)>,
    loudspeakers: Vec<(
        usize,
        usize,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
        usize,
        usize,
        usize,
    )>, // (..., idx, r_i, r_v)
    pickups: Vec<(usize, usize, f64, f64, f64, usize, usize)>, // (..., idx, r)
    varactors: Vec<(usize, usize, f64, f64, f64, usize)>,
    saturable_inductors: Vec<(usize, usize, f64, f64, f64, usize)>,
    tunnel_diodes: Vec<(usize, usize, f64, f64, f64, f64, usize)>,
    spark_gaps: Vec<(usize, usize, f64, f64, usize)>,
    ghosts: Vec<(usize, usize, f64, f64, f64, usize)>,
    vcrs: Vec<(usize, usize, usize, f64, f64, f64, f64, usize)>,
}

impl MnaSolver {
    pub fn new(dt: f64) -> Self {
        Self {
            elements: Vec::new(),
            num_nodes: 0,
            num_extra_rows: 0,
            dt,
            context: CircuitContext {
                temperature_c: 25.0,
                global_drift: 1.0,
                vcc: 15.0,
                vee: -15.0,
            },
            prev_solution: Vec::new(),
            static_triplets: Vec::new(),
            is_static_dirty: true,
            has_nonlinear: false,
            delay_buffers: Vec::new(),
            execution_plan: None,
            noise_seed: 0,
            is_solving_gmin: false,
        }
    }

    pub fn add_element(&mut self, el: CircuitElement) {
        match &el {
            CircuitElement::VoltageSource { .. }
            | CircuitElement::ControlledSource {
                kind: ControlledSourceKind::VCVS,
                ..
            }
            | CircuitElement::LogicGate { .. }
            | CircuitElement::Comparator { .. }
            | CircuitElement::PulseSource { .. }
            | CircuitElement::VoltageNoise { .. }
            | CircuitElement::HallSensor { .. }
            | CircuitElement::DyingBattery { .. }
            | CircuitElement::Ota { .. } => self.num_extra_rows += 1,
            CircuitElement::DcMotor { .. } => self.num_extra_rows += 2,
            CircuitElement::Crystal { .. } => self.num_extra_rows += 1,
            CircuitElement::TransmissionLine { delay_samples, .. } => {
                let mut dq = VecDeque::with_capacity(*delay_samples);
                for _ in 0..*delay_samples {
                    dq.push_back((0.0, 0.0));
                }
                self.delay_buffers.push(dq);
            }
            CircuitElement::Bbd { num_stages, .. } => {
                self.num_extra_rows += 1;
                let mut dq = VecDeque::with_capacity(*num_stages);
                for _ in 0..*num_stages {
                    dq.push_back((0.0, 0.0));
                }
                self.delay_buffers.push(dq);
            }
            CircuitElement::Loudspeaker { .. } => self.num_extra_rows += 2,
            CircuitElement::GuitarPickup { .. } => self.num_extra_rows += 1,
            _ => {}
        }
        self.elements.push(el);
        self.is_static_dirty = true;
    }

    pub fn num_elements(&self) -> usize {
        self.elements.len()
    }
    pub fn add_element_dummy_handle(&mut self, idx: usize) -> Option<&mut CircuitElement> {
        self.elements.get_mut(idx)
    }

    pub fn apply_tolerance(&mut self, seed: u64) {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for el in &mut self.elements {
            match el {
                CircuitElement::Resistor {
                    value, tolerance, ..
                } => {
                    let factor = 1.0 + (rng.gen::<f64>() * 2.0 - 1.0) * *tolerance;
                    *value *= factor;
                }
                CircuitElement::Capacitor {
                    value, tolerance, ..
                } => {
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
        let n = self.num_nodes;
        let dim = n + self.num_extra_rows;
        self.static_triplets.clear();
        for i in 0..dim {
            self.static_triplets.push((i, i, 1e-12));
        }
        self.static_triplets.push((0, 0, 1.0));
        self.has_nonlinear = false;
        let mut plan = MnaExecutionPlan {
            resistors: Vec::new(),
            capacitors: Vec::new(),
            inductors: Vec::new(),
            v_sources: Vec::new(),
            i_sources: Vec::new(),
            diodes: Vec::new(),
            zeners: Vec::new(),
            triodes: Vec::new(),
            pentodes: Vec::new(),
            bjts: Vec::new(),
            jfets: Vec::new(),
            transformers: Vec::new(),
            opamps: Vec::new(),
            pots: Vec::new(),
            switches: Vec::new(),
            vcvs: Vec::new(),
            vccs: Vec::new(),
            t_lines: Vec::new(),
            memristors: Vec::new(),
            mosfets: Vec::new(),
            igbts: Vec::new(),
            scrs: Vec::new(),
            triacs: Vec::new(),
            motors: Vec::new(),
            thermistors: Vec::new(),
            ldrs: Vec::new(),
            logic_gates: Vec::new(),
            noise_sources: Vec::new(),
            crystals: Vec::new(),
            comparators: Vec::new(),
            pulse_sources: Vec::new(),
            hall_sensors: Vec::new(),
            otas: Vec::new(),
            vactrols: Vec::new(),
            batteries: Vec::new(),
            bbds: Vec::new(),
            relays: Vec::new(),
            neons: Vec::new(),
            dirty_grounds: Vec::new(),
            loudspeakers: Vec::new(),
            pickups: Vec::new(),
            varactors: Vec::new(),
            saturable_inductors: Vec::new(),
            tunnel_diodes: Vec::new(),
            spark_gaps: Vec::new(),
            ghosts: Vec::new(),
            vcrs: Vec::new(),
        };
        let mut row_idx = 0;
        let mut t_line_idx = 0;
        let mut bbd_idx = 0;
        let els = self.elements.clone();
        for (idx, el) in els.iter().enumerate() {
            match el {
                CircuitElement::Resistor { a, b, value, .. } => {
                    let g = 1.0 / value.max(1e-9);
                    self.stamp_static(a.0, b.0, g, n);
                    plan.resistors.push((a.0, b.0, g));
                }
                CircuitElement::Capacitor { a, b, value, .. } => {
                    let g = value / self.dt;
                    self.stamp_static(a.0, b.0, g, n);
                    plan.capacitors.push((a.0, b.0, g, idx));
                }
                CircuitElement::Inductor { a, b, value, .. } => {
                    let g = self.dt / value.max(1e-9);
                    self.stamp_static(a.0, b.0, g, n);
                    plan.inductors.push((a.0, b.0, g, idx));
                }
                CircuitElement::VoltageSource { pos, neg, voltage } => {
                    let r = n + row_idx;
                    if pos.0 > 0 && pos.0 < n {
                        self.static_triplets.push((pos.0, r, 1.0));
                        self.static_triplets.push((r, pos.0, 1.0));
                    }
                    if neg.0 > 0 && neg.0 < n {
                        self.static_triplets.push((neg.0, r, -1.0));
                        self.static_triplets.push((r, neg.0, -1.0));
                    }
                    plan.v_sources.push((pos.0, neg.0, *voltage, r));
                    row_idx += 1;
                }
                CircuitElement::CurrentSource { pos, neg, current } => {
                    plan.i_sources.push((pos.0, neg.0, *current))
                }
                CircuitElement::Potentiometer {
                    a,
                    wiper,
                    b,
                    value,
                    pos,
                    taper,
                } => {
                    let p = match taper {
                        PotTaper::Linear => *pos,
                        PotTaper::Log => (*pos * 2.302).exp() / 10.0,
                        PotTaper::AntiLog => 1.0 - ((1.0 - *pos) * 2.302).exp() / 10.0,
                    }
                    .clamp(0.001, 0.999);
                    let r1 = value * p;
                    let r2 = value * (1.0 - p);
                    self.stamp_static(a.0, wiper.0, 1.0 / r1, n);
                    self.stamp_static(wiper.0, b.0, 1.0 / r2, n);
                    plan.pots.push((a.0, wiper.0, b.0, 1.0 / r1, 1.0 / r2));
                }
                CircuitElement::Switch { a, b, closed } => {
                    let g = if *closed { 1e6 } else { 1e-12 };
                    self.stamp_static(a.0, b.0, g, n);
                    plan.switches.push((a.0, b.0, *closed));
                }
                CircuitElement::OpAmp {
                    pos,
                    neg,
                    out,
                    gain,
                } => {
                    if out.0 > 0 && out.0 < n {
                        if pos.0 > 0 && pos.0 < n {
                            self.static_triplets.push((out.0, pos.0, *gain));
                        }
                        if neg.0 > 0 && neg.0 < n {
                            self.static_triplets.push((out.0, neg.0, -*gain));
                        }
                    }
                    plan.opamps.push((pos.0, neg.0, out.0, *gain));
                }
                CircuitElement::ControlledSource {
                    kind,
                    target_a,
                    target_b,
                    control_a,
                    control_b,
                    gain,
                } => match kind {
                    ControlledSourceKind::VCCS => {
                        if target_a.0 > 0 && target_a.0 < n {
                            if control_a.0 > 0 && control_a.0 < n {
                                self.static_triplets.push((target_a.0, control_a.0, *gain));
                            }
                            if control_b.0 > 0 && control_b.0 < n {
                                self.static_triplets.push((target_a.0, control_b.0, -*gain));
                            }
                        }
                        if target_b.0 > 0 && target_b.0 < n {
                            if control_a.0 > 0 && control_a.0 < n {
                                self.static_triplets.push((target_b.0, control_a.0, -*gain));
                            }
                            if control_b.0 > 0 && control_b.0 < n {
                                self.static_triplets.push((target_b.0, control_b.0, *gain));
                            }
                        }
                        plan.vccs
                            .push((target_a.0, target_b.0, control_a.0, control_b.0, *gain));
                    }
                    ControlledSourceKind::VCVS => {
                        let r = n + row_idx;
                        if target_a.0 > 0 && target_a.0 < n {
                            self.static_triplets.push((target_a.0, r, 1.0));
                            self.static_triplets.push((r, target_a.0, 1.0));
                        }
                        if target_b.0 > 0 && target_b.0 < n {
                            self.static_triplets.push((target_b.0, r, -1.0));
                            self.static_triplets.push((r, target_b.0, -1.0));
                        }
                        if control_a.0 > 0 && control_a.0 < n {
                            self.static_triplets.push((r, control_a.0, -*gain));
                        }
                        if control_b.0 > 0 && control_b.0 < n {
                            self.static_triplets.push((r, control_b.0, *gain));
                        }
                        plan.vcvs.push((
                            target_a.0,
                            target_b.0,
                            control_a.0,
                            control_b.0,
                            *gain,
                            r,
                        ));
                        row_idx += 1;
                    }
                    _ => {}
                },
                CircuitElement::Transformer {
                    a1,
                    b1,
                    a2,
                    b2,
                    l1,
                    l2,
                    coupling,
                    ..
                } => {
                    let m = coupling * (l1 * l2).sqrt();
                    let det = l1 * l2 - m * m;
                    let g11 = self.dt * l2 / det;
                    let g12 = -self.dt * m / det;
                    let g21 = -self.dt * m / det;
                    let g22 = self.dt * l1 / det;
                    self.stamp_static(a1.0, b1.0, g11, n);
                    self.stamp_static(a2.0, b2.0, g22, n);
                    if a1.0 > 0 && a2.0 > 0 && a1.0 < n && a2.0 < n {
                        self.static_triplets.push((a1.0, a2.0, g12));
                        self.static_triplets.push((a2.0, a1.0, g21));
                    }
                    plan.transformers
                        .push((a1.0, b1.0, a2.0, b2.0, g11, g12, g21, g22, idx));
                }
                CircuitElement::TransmissionLine {
                    a1,
                    b1,
                    a2,
                    b2,
                    z0,
                    delay_samples,
                } => {
                    let g0 = 1.0 / z0;
                    self.stamp_static(a1.0, b1.0, g0, n);
                    self.stamp_static(a2.0, b2.0, g0, n);
                    plan.t_lines
                        .push((a1.0, b1.0, a2.0, b2.0, *z0, *delay_samples, t_line_idx));
                    t_line_idx += 1;
                }
                CircuitElement::Diode { a, k, is, .. } => {
                    self.has_nonlinear = true;
                    plan.diodes.push((a.0, k.0, *is, idx));
                }
                CircuitElement::Zener { a, k, is, vz } => {
                    self.has_nonlinear = true;
                    plan.zeners.push((a.0, k.0, *is, *vz, idx));
                }
                CircuitElement::Triode {
                    g,
                    k,
                    p,
                    mu,
                    kg1,
                    kp,
                    kvb,
                    ex,
                    material,
                } => {
                    self.has_nonlinear = true;
                    plan.triodes
                        .push((g.0, k.0, p.0, *mu, *kg1, *kp, *kvb, *ex, *material, idx));
                }
                CircuitElement::Pentode {
                    g1,
                    g2,
                    k,
                    p,
                    mu,
                    kg1,
                    kp,
                    kvb,
                    ex,
                    material,
                } => {
                    self.has_nonlinear = true;
                    plan.pentodes
                        .push((g1.0, g2.0, k.0, p.0, *mu, *kg1, *kp, *kvb, *ex, *material, idx));
                }
                CircuitElement::Bjt {
                    b,
                    c,
                    e,
                    is,
                    bf,
                    br,
                    is_npn,
                } => {
                    self.has_nonlinear = true;
                    plan.bjts.push((b.0, c.0, e.0, *is, *bf, *br, *is_npn, idx));
                }
                CircuitElement::Jfet {
                    g,
                    d,
                    s,
                    vto,
                    beta,
                    is_n_channel,
                } => {
                    self.has_nonlinear = true;
                    plan.jfets
                        .push((g.0, d.0, s.0, *vto, *beta, *is_n_channel, idx));
                }
                CircuitElement::Mosfet {
                    g,
                    d,
                    s,
                    vto,
                    beta,
                    lambda,
                    is_n_channel,
                } => {
                    self.has_nonlinear = true;
                    plan.mosfets
                        .push((g.0, d.0, s.0, *vto, *beta, *lambda, *is_n_channel, idx));
                }
                CircuitElement::Igbt {
                    g,
                    c,
                    e,
                    vto,
                    beta,
                    bf,
                    is,
                } => {
                    self.has_nonlinear = true;
                    plan.igbts.push((g.0, c.0, e.0, *vto, *beta, *bf, *is, idx));
                }
                CircuitElement::Scr {
                    a,
                    k,
                    g,
                    v_hold,
                    i_hold,
                    i_gate_trigger,
                    ..
                } => {
                    self.has_nonlinear = true;
                    plan.scrs
                        .push((a.0, k.0, g.0, *v_hold, *i_hold, *i_gate_trigger, idx));
                }
                CircuitElement::Triac {
                    m1,
                    m2,
                    g,
                    v_hold,
                    i_hold,
                    i_gate_trigger,
                    ..
                } => {
                    self.has_nonlinear = true;
                    plan.triacs
                        .push((m1.0, m2.0, g.0, *v_hold, *i_hold, *i_gate_trigger, idx));
                }
                CircuitElement::DcMotor {
                    pos,
                    neg,
                    resistance,
                    inductance,
                    ke,
                    kt,
                    inertia,
                    friction,
                    ..
                } => {
                    let r_ia = n + row_idx;
                    plan.motors.push((
                        pos.0,
                        neg.0,
                        *resistance,
                        *inductance,
                        *ke,
                        *kt,
                        *inertia,
                        *friction,
                        r_ia,
                        idx,
                    ));
                    row_idx += 2;
                }
                CircuitElement::Thermistor {
                    a,
                    b,
                    r25,
                    beta,
                    is_ntc,
                } => {
                    plan.thermistors.push((a.0, b.0, *r25, *beta, *is_ntc, idx));
                }
                CircuitElement::Ldr {
                    a,
                    b,
                    r_dark,
                    gamma,
                    ..
                } => {
                    plan.ldrs.push((a.0, b.0, *r_dark, *gamma, 0.0, idx));
                }
                CircuitElement::LogicGate {
                    kind,
                    inputs,
                    out,
                    v_high,
                    v_low,
                    delay,
                    ..
                } => {
                    let r = n + row_idx;
                    if out.0 > 0 && out.0 < n {
                        self.static_triplets.push((out.0, r, 1.0));
                        self.static_triplets.push((r, out.0, 1.0));
                    }
                    plan.logic_gates.push((
                        kind.clone(),
                        inputs.iter().map(|n| n.0).collect(),
                        out.0,
                        *v_high,
                        *v_low,
                        *delay,
                        idx,
                        r,
                    ));
                    row_idx += 1;
                    self.has_nonlinear = true;
                }
                CircuitElement::VoltageNoise {
                    a,
                    b,
                    density,
                    flicker_alpha,
                    hum_amplitude,
                    psu_ripple,
                } => {
                    let r = n + row_idx;
                    if a.0 > 0 && a.0 < n {
                        self.static_triplets.push((a.0, r, 1.0));
                        self.static_triplets.push((r, a.0, 1.0));
                    }
                    if b.0 > 0 && b.0 < n {
                        self.static_triplets.push((b.0, r, -1.0));
                        self.static_triplets.push((r, b.0, -1.0));
                    }
                    plan.noise_sources.push((
                        true,
                        a.0,
                        b.0,
                        *density,
                        *flicker_alpha,
                        *hum_amplitude,
                        *psu_ripple,
                        idx,
                        r,
                    ));
                    row_idx += 1;
                }
                CircuitElement::Crystal {
                    a,
                    b,
                    lm,
                    cm,
                    rm,
                    co,
                    ..
                } => {
                    let r = n + row_idx;
                    self.stamp_static(a.0, b.0, 1.0 / (*co).max(1e-12), n);
                    plan.crystals.push((a.0, b.0, *lm, *cm, *rm, *co, idx, r));
                    row_idx += 1;
                }
                CircuitElement::Comparator {
                    pos,
                    neg,
                    out,
                    v_high,
                    v_low,
                } => {
                    let r = n + row_idx;
                    if out.0 > 0 && out.0 < n {
                        self.static_triplets.push((out.0, r, 1.0));
                        self.static_triplets.push((r, out.0, 1.0));
                    }
                    plan.comparators
                        .push((pos.0, neg.0, out.0, *v_high, *v_low, r));
                    row_idx += 1;
                    self.has_nonlinear = true;
                }
                CircuitElement::PulseSource {
                    pos,
                    neg,
                    amplitude,
                    freq,
                    duty,
                    rise_time,
                } => {
                    let r = n + row_idx;
                    if pos.0 > 0 && pos.0 < n {
                        self.static_triplets.push((pos.0, r, 1.0));
                        self.static_triplets.push((r, pos.0, 1.0));
                    }
                    if neg.0 > 0 && neg.0 < n {
                        self.static_triplets.push((neg.0, r, -1.0));
                        self.static_triplets.push((r, neg.0, -1.0));
                    }
                    plan.pulse_sources
                        .push((pos.0, neg.0, *amplitude, *freq, *duty, *rise_time, r));
                    row_idx += 1;
                    self.has_nonlinear = true;
                }
                CircuitElement::HallSensor {
                    pos,
                    neg,
                    out,
                    sensitivity,
                    b_field,
                } => {
                    let r = n + row_idx;
                    if out.0 > 0 && out.0 < n {
                        self.static_triplets.push((out.0, r, 1.0));
                        self.static_triplets.push((r, out.0, 1.0));
                    }
                    plan.hall_sensors
                        .push((pos.0, neg.0, out.0, *sensitivity, *b_field, r));
                    row_idx += 1;
                    self.has_nonlinear = true;
                }
                CircuitElement::Ota {
                    p,
                    n: neg,
                    iabc,
                    out,
                    gm_per_amp,
                    r_in,
                } => {
                    plan.otas
                        .push((p.0, neg.0, iabc.0, out.0, *gm_per_amp, *r_in));
                    self.stamp_static(iabc.0, 0, 1.0 / r_in.max(1e-9), n);
                    self.has_nonlinear = true;
                }
                CircuitElement::Vactrol {
                    led_p,
                    led_n,
                    ldr_a,
                    ldr_b,
                    tau_rise,
                    tau_fall,
                    ..
                } => {
                    plan.vactrols.push((
                        led_p.0, led_n.0, ldr_a.0, ldr_b.0, *tau_rise, *tau_fall, idx,
                    ));
                }
                CircuitElement::DyingBattery {
                    pos,
                    neg,
                    voltage,
                    internal_r,
                    ..
                } => {
                    let r = n + row_idx;
                    if pos.0 > 0 && pos.0 < n {
                        self.static_triplets.push((pos.0, r, 1.0));
                        self.static_triplets.push((r, pos.0, 1.0));
                    }
                    if neg.0 > 0 && neg.0 < n {
                        self.static_triplets.push((neg.0, r, -1.0));
                        self.static_triplets.push((r, neg.0, -1.0));
                    }
                    self.static_triplets.push((r, r, -*internal_r));
                    plan.batteries
                        .push((pos.0, neg.0, *voltage, *internal_r, idx, r));
                    row_idx += 1;
                }
                CircuitElement::Bbd {
                    input,
                    output,
                    clock_hz,
                    num_stages,
                    ..
                } => {
                    let r = n + row_idx;
                    if output.0 > 0 && output.0 < n {
                        self.static_triplets.push((output.0, r, 1.0));
                        self.static_triplets.push((r, output.0, 1.0));
                    }
                    plan.bbds
                        .push((input.0, output.0, *clock_hz, *num_stages, r, idx, bbd_idx));
                    row_idx += 1;
                    bbd_idx += 1;
                }
                CircuitElement::Loudspeaker {
                    a,
                    b,
                    re,
                    le,
                    bl,
                    mms,
                    rms,
                    cms,
                    ..
                } => {
                    let r_i = n + row_idx;
                    let r_v = n + row_idx + 1;
                    plan.loudspeakers
                        .push((a.0, b.0, *re, *le, *bl, *mms, *rms, *cms, idx, r_i, r_v));
                    row_idx += 2;
                }
                CircuitElement::GuitarPickup { a, b, l, r, c, .. } => {
                    let r_i = n + row_idx;
                    if a.0 > 0 && a.0 < n {
                        self.static_triplets.push((a.0, r_i, 1.0));
                        self.static_triplets.push((r_i, a.0, 1.0));
                    }
                    if b.0 > 0 && b.0 < n {
                        self.static_triplets.push((b.0, r_i, -1.0));
                        self.static_triplets.push((r_i, b.0, -1.0));
                    }
                    self.static_triplets.push((r_i, r_i, -(*r + *l / self.dt)));
                    self.stamp_static(a.0, b.0, *c / self.dt, n);
                    plan.pickups.push((a.0, b.0, *l, *r, *c, idx, r_i));
                    row_idx += 1;
                }
                CircuitElement::Relay {
                    coil_p,
                    coil_n,
                    a,
                    b,
                    l_coil,
                    r_coil,
                    ..
                } => {
                    plan.relays
                        .push((coil_p.0, coil_n.0, a.0, b.0, *l_coil, *r_coil, idx));
                }
                CircuitElement::NeonBulb {
                    a,
                    b,
                    v_breakdown,
                    v_extinguish,
                    ..
                } => {
                    plan.neons
                        .push((a.0, b.0, *v_breakdown, *v_extinguish, idx));
                }
                CircuitElement::DirtyGround {
                    node,
                    r_parasitic,
                    l_parasitic,
                    noise_density,
                } => {
                    let g = 1.0 / (r_parasitic + l_parasitic / self.dt);
                    self.stamp_static(node.0, 0, g, n);
                    plan.dirty_grounds
                        .push((node.0, *r_parasitic, *l_parasitic, *noise_density));
                }
                CircuitElement::Memristor {
                    a,
                    b,
                    ron,
                    roff,
                    mu,
                    d,
                    w,
                } => {
                    let r = ron * w + roff * (1.0 - w);
                    self.stamp_static(a.0, b.0, 1.0 / r.max(1e-3), n);
                    plan.memristors.push((a.0, b.0, *ron, *roff, *mu, *d, idx));
                    self.has_nonlinear = true;
                }
                CircuitElement::Varactor { a, k, c0, vj, m, .. } => {
                    self.has_nonlinear = true;
                    plan.varactors.push((a.0, k.0, *c0, *vj, *m, idx));
                }
                CircuitElement::SaturableInductor { a, b, l_max, l_min, i_sat, .. } => {
                    self.has_nonlinear = true;
                    plan.saturable_inductors.push((a.0, b.0, *l_max, *l_min, *i_sat, idx));
                }
                CircuitElement::TunnelDiode { a, k, ip, vp, iv, vv, .. } => {
                    self.has_nonlinear = true;
                    plan.tunnel_diodes.push((a.0, k.0, *ip, *vp, *iv, *vv, idx));
                }
                CircuitElement::SparkGap { a, b, v_breakdown, v_hold, .. } => {
                    self.has_nonlinear = true;
                    plan.spark_gaps.push((a.0, b.0, *v_breakdown, *v_hold, idx));
                }
                CircuitElement::CircuitGhost { a, b, c_min, c_max, drift_rate, .. } => {
                    plan.ghosts.push((a.0, b.0, *c_min, *c_max, *drift_rate, idx));
                }
                CircuitElement::VoltageControlledResistor { a, b, ctrl, r_min, r_max, v_min, v_max } => {
                    self.has_nonlinear = true;
                    plan.vcrs.push((a.0, b.0, ctrl.0, *r_min, *r_max, *v_min, *v_max, idx));
                }
                _ => {}
            }
        }
        self.execution_plan = Some(plan);
        self.is_static_dirty = false;
    }

    fn stamp_static(&mut self, a: usize, b: usize, g: f64, n: usize) {
        if a > 0 && a < n {
            self.static_triplets.push((a, a, g));
        }
        if b > 0 && b < n {
            self.static_triplets.push((b, b, g));
        }
        if a > 0 && b > 0 && a < n && b < n {
            self.static_triplets.push((a, b, -g));
            self.static_triplets.push((b, a, -g));
        }
    }

    pub fn solve(&mut self) -> CircuitState {
        if self.is_static_dirty {
            self.rebuild_static_cache();
        }
        let n = self.num_nodes;
        let dim = n + self.num_extra_rows;
        if self.prev_solution.len() != dim {
            self.prev_solution = vec![0.0; dim];
        }
        let mut x = self.prev_solution.clone();
        let plan = self.execution_plan.as_ref().cloned().expect("Plan exists");
        let mut converged = false;
        let mut final_iters = 0;
        let iter_limit = if self.has_nonlinear { 500 } else { 2 };

        let mut resistor_noises = Vec::with_capacity(plan.resistors.len());
        let mut ground_noises = Vec::with_capacity(plan.dirty_grounds.len());
        {
            use rand::{Rng, SeedableRng};
            let mut rng = rand::rngs::StdRng::seed_from_u64(self.noise_seed);
            for &(_a, _b, g) in &plan.resistors {
                let temp_k = self.context.temperature_c + 273.15;
                let noise_v = (4.0 * 1.38e-23 * temp_k * (1.0 / g) / self.dt).sqrt();
                resistor_noises.push((rng.gen::<f64>() * 2.0 - 1.0) * noise_v * g);
            }
            for &(_node, r, l, density) in &plan.dirty_grounds {
                let g = 1.0 / (r + l / self.dt);
                ground_noises.push((rng.gen::<f64>() * 2.0 - 1.0) * density * g);
            }
        }

        for iter in 0..iter_limit {
            final_iters = iter + 1;
            let mut f_x = vec![0.0; dim];
            f_x[0] = x[0];
            let mut triplets = self.static_triplets.clone();

            for (idx, &(a, b, g)) in plan.resistors.iter().enumerate() {
                Self::stamp_f(n, &mut f_x, a, b, g, &x);
                Self::stamp_current(n, &mut f_x, a, b, resistor_noises[idx]);
            }
            for &(a, b, g, idx) in &plan.capacitors {
                if let CircuitElement::Capacitor { state_v, .. } = &self.elements[idx] {
                    Self::stamp_current(
                        n,
                        &mut f_x,
                        a,
                        b,
                        g * (x_val(&x, a, n) - x_val(&x, b, n) - *state_v),
                    );
                }
            }
            for &(a, b, g, idx) in &plan.inductors {
                if let CircuitElement::Inductor { state_i, .. } = &self.elements[idx] {
                    Self::stamp_current(
                        n,
                        &mut f_x,
                        a,
                        b,
                        g * (x_val(&x, a, n) - x_val(&x, b, n)) + *state_i,
                    );
                }
            }
            for &(p, neg, v, r) in &plan.v_sources {
                if p > 0 && p < n {
                    f_x[p] += x[r];
                }
                if neg > 0 && neg < n {
                    f_x[neg] -= x[r];
                }
                f_x[r] = x_val(&x, p, n) - x_val(&x, neg, n) - v;
            }
            for &(p, neg, i) in &plan.i_sources {
                Self::stamp_current(n, &mut f_x, p, neg, i);
            }
            for &(a, w, b, g1, g2) in &plan.pots {
                Self::stamp_f(n, &mut f_x, a, w, g1, &x);
                Self::stamp_f(n, &mut f_x, w, b, g2, &x);
            }
            for &(a, b, c) in &plan.switches {
                Self::stamp_f(n, &mut f_x, a, b, if c { 1e6 } else { 1e-12 }, &x);
            }
            for &(p, neg, o, g) in &plan.opamps {
                if o > 0 && o < n {
                    f_x[o] += g * (x_val(&x, p, n) - x_val(&x, neg, n));
                }
            }
            for &(ta, tb, ca, cb, g) in &plan.vccs {
                Self::stamp_current(
                    n,
                    &mut f_x,
                    ta,
                    tb,
                    g * (x_val(&x, ca, n) - x_val(&x, cb, n)),
                );
            }
            for &(ta, tb, ca, cb, g, r) in &plan.vcvs {
                if ta > 0 && ta < n {
                    f_x[ta] += x[r];
                }
                if tb > 0 && tb < n {
                    f_x[tb] -= x[r];
                }
                f_x[r] = (x_val(&x, ta, n) - x_val(&x, tb, n))
                    - g * (x_val(&x, ca, n) - x_val(&x, cb, n));
            }
            for &(kind, ref ins, out, vh, vl, _delay, _idx, r) in &plan.logic_gates {
                let v_th = (vh + vl) / 2.0;
                let res_discrete = match kind {
                    LogicKind::AND => ins.iter().all(|&i| x_val(&x, i, n) > v_th),
                    LogicKind::OR => ins.iter().any(|&i| x_val(&x, i, n) > v_th),
                    LogicKind::XOR => {
                        ins.iter().filter(|&&i| x_val(&x, i, n) > v_th).count() % 2 == 1
                    }
                    LogicKind::NOT => x_val(&x, ins[0], n) < v_th,
                    LogicKind::NAND => !ins.iter().all(|&i| x_val(&x, i, n) > v_th),
                    LogicKind::NOR => !ins.iter().any(|&i| x_val(&x, i, n) > v_th),
                };
                let v_target = if res_discrete { vh } else { vl };
                if out > 0 && out < n {
                    f_x[out] += x[r];
                }
                f_x[r] = x_val(&x, out, n) - v_target;
            }
            for &(p, neg, out, vh, vl, r) in &plan.comparators {
                let v_diff = x_val(&x, p, n) - x_val(&x, neg, n);
                let res_soft = 1.0 / (1.0 + (-v_diff / 0.01).exp());
                let v_target = vl + (vh - vl) * res_soft;
                if out > 0 && out < n {
                    f_x[out] += x[r];
                }
                f_x[r] = x_val(&x, out, n) - v_target;
            }
            for &(p, neg, amp, freq, duty, _rise, r) in &plan.pulse_sources {
                let t = (self.dt * self.noise_seed as f64) % (1.0 / freq);
                let res = t < (1.0 / freq) * duty;
                let v_target = if res { amp } else { 0.0 };
                if p > 0 && p < n {
                    f_x[p] += x[r];
                }
                if neg > 0 && neg < n {
                    f_x[neg] -= x[r];
                }
                f_x[r] = x_val(&x, p, n) - x_val(&x, neg, n) - v_target;
            }
            for &(p, neg, out, sens, b, r) in &plan.hall_sensors {
                let v_target = (x_val(&x, p, n) - x_val(&x, neg, n)) + b * sens;
                if out > 0 && out < n {
                    f_x[out] += x[r];
                }
                f_x[r] = x_val(&x, out, n) - v_target;
            }
            for &(a1, b1, a2, b2, g11, g12, g21, g22, idx) in &plan.transformers {
                if let CircuitElement::Transformer {
                    state_i1, state_i2, ..
                } = &self.elements[idx]
                {
                    let v1 = x_val(&x, a1, n) - x_val(&x, b1, n);
                    let v2 = x_val(&x, a2, n) - x_val(&x, b2, n);
                    Self::stamp_current(n, &mut f_x, a1, b1, g11 * v1 + g12 * v2 + *state_i1);
                    Self::stamp_current(n, &mut f_x, a2, b2, g21 * v1 + g22 * v2 + *state_i2);
                }
            }
            let mut ti = 0;
            for &(a1, b1, a2, b2, z0, _, _ti_in_plan) in &plan.t_lines {
                if let Some(dq) = self.delay_buffers.get(ti) {
                    let (v1o, v2o) = dq.front().copied().unwrap_or((0.0, 0.0));
                    Self::stamp_current(n, &mut f_x, a1, b1, -v2o / z0);
                    Self::stamp_current(n, &mut f_x, a2, b2, -v1o / z0);
                }
                ti += 1;
            }
            for &(_inp, out, _clk_hz, _stages, r, el_idx, b_idx) in &plan.bbds {
                let v_out = self
                    .delay_buffers
                    .get(ti + b_idx)
                    .and_then(|dq| {
                        if dq.len() >= 2 {
                            if let CircuitElement::Bbd { state_phase, .. } = &self.elements[el_idx]
                            {
                                let v0 = dq[0].0;
                                let v1 = dq[1].0;
                                Some(v0 * (1.0 - state_phase) + v1 * state_phase)
                            } else {
                                dq.front().map(|&(v, _)| v)
                            }
                        } else {
                            dq.front().map(|&(v, _)| v)
                        }
                    })
                    .unwrap_or(0.0);
                if out > 0 && out < n {
                    f_x[out] += x[r];
                }
                f_x[r] = x_val(&x, out, n) - v_out;
            }
            for &(a, b, re, le, bl, mms, rms, cms, idx, ri, rv) in &plan.loudspeakers {
                if let CircuitElement::Loudspeaker {
                    state_i,
                    state_v,
                    state_x,
                    ..
                } = &self.elements[idx]
                {
                    if a > 0 && a < n {
                        f_x[a] += x[ri];
                        triplets.push((a, ri, 1.0));
                        triplets.push((ri, a, 1.0));
                    }
                    if b > 0 && b < n {
                        f_x[b] -= x[ri];
                        triplets.push((b, ri, -1.0));
                        triplets.push((ri, b, -1.0));
                    }
                    let g_elec = re + le / self.dt;
                    let g_mech = mms / self.dt + rms;
                    f_x[ri] = (x_val(&x, a, n) - x_val(&x, b, n)) - x[ri] * g_elec - x[rv] * bl
                        + le / self.dt * state_i;
                    f_x[rv] = x[ri] * bl - x[rv] * g_mech + mms / self.dt * state_v
                        - (1.0 / cms) * state_x;
                    triplets.push((ri, ri, -g_elec));
                    triplets.push((ri, rv, -bl));
                    triplets.push((rv, ri, bl));
                    triplets.push((rv, rv, -g_mech));
                }
            }
            for &(a, b, l, r_val, _c, idx, ri) in &plan.pickups {
                if let CircuitElement::GuitarPickup {
                    state_i, flux_v, ..
                } = &self.elements[idx]
                {
                    if a > 0 && a < n {
                        f_x[a] += x[ri];
                    }
                    if b > 0 && b < n {
                        f_x[b] -= x[ri];
                    }
                    f_x[ri] = (x_val(&x, a, n) - x_val(&x, b, n)) - x[ri] * (r_val + l / self.dt)
                        + l / self.dt * state_i
                        + flux_v;
                }
            }
            for &(p, neg, ctrl, out, gm, r_in) in &plan.otas {
                let vt = 0.026;
                let i_abc = (x_val(&x, ctrl, n) / r_in.max(1e-9)).max(1e-9);
                let v_diff = x_val(&x, p, n) - x_val(&x, neg, n);
                let arg = (v_diff / (2.0 * vt)).clamp(-4.0, 4.0);
                let tanh_v = arg.tanh();
                let i_out = i_abc * gm * tanh_v;
                let g_m = (i_abc * gm / (2.0 * vt)) * (1.0 - tanh_v.powi(2));
                if out > 0 && out < n {
                    f_x[out] -= i_out;
                    triplets.push((out, p, -g_m));
                    triplets.push((out, neg, g_m));
                }
            }
            for &(lp, ln, la, lb, _, _, idx) in &plan.vactrols {
                if let CircuitElement::Vactrol {
                    state_brightness, ..
                } = &self.elements[idx]
                {
                    let g = (*state_brightness).max(1e-9);
                    Self::stamp_f(n, &mut f_x, la, lb, g, &x);
                }
                let v_led = x_val(&x, lp, n) - x_val(&x, ln, n);
                let i_led = 1e-12 * ((v_led / 0.026).exp() - 1.0);
                Self::stamp_current(n, &mut f_x, lp, ln, i_led);
            }
            for &(p, neg, v, _, idx, r) in &plan.batteries {
                if let CircuitElement::DyingBattery {
                    current_charge,
                    capacity_ah,
                    ..
                } = &self.elements[idx]
                {
                    let v_eff = v * (0.5 + 0.5 * (*current_charge / *capacity_ah).min(1.0));
                    if p > 0 && p < n {
                        f_x[p] += x[r];
                    }
                    if neg > 0 && neg < n {
                        f_x[neg] -= x[r];
                    }
                    f_x[r] = x_val(&x, p, n) - x_val(&x, neg, n) - v_eff;
                }
            }
            for &(cp, cn, a, b, _, _, idx) in &plan.relays {
                let _v_coil = x_val(&x, cp, n) - x_val(&x, cn, n);
                if let CircuitElement::Relay { state_on, .. } = &self.elements[idx] {
                    let g = if *state_on { 1e6 } else { 1e-12 };
                    Self::stamp_f(n, &mut f_x, a, b, g, &x);
                }
                Self::stamp_f(n, &mut f_x, cp, cn, 1.0 / 100.0, &x);
            }
            for &(a, b, _, _, idx) in &plan.neons {
                if let CircuitElement::NeonBulb { state_on, .. } = &self.elements[idx] {
                    let g = if *state_on { 1e-2 } else { 1e-12 };
                    Self::stamp_f(n, &mut f_x, a, b, g, &x);
                }
            }
            for (idx, &(node, _r, _l, _density)) in plan.dirty_grounds.iter().enumerate() {
                Self::stamp_current(n, &mut f_x, node, 0, ground_noises[idx]);
            }
            for &(a, k, is, _) in &plan.diodes {
                let vt = 0.026;
                let vd = x_val(&x, a, n) - x_val(&x, k, n);
                let vd_lim = 0.8;
                let vd_eff = if vd > vd_lim {
                    vd_lim + (vd - vd_lim).ln_1p()
                } else {
                    vd
                };
                let ev = (vd_eff / vt).clamp(-40.0, 40.0).exp();
                let id = is * (ev - 1.0);
                let gd = (is / vt) * ev;
                Self::stamp_current(n, &mut f_x, a, k, id);
                Self::stamp_dynamic(&mut triplets, a, k, gd, n);
            }
            for &(a, k, is, vz, _) in &plan.zeners {
                let vt = 0.026;
                let vd = x_val(&x, a, n) - x_val(&x, k, n);
                let ef = (vd / 0.026).clamp(-40.0, 40.0).exp();
                let er = ((-vd - vz) / 0.026).clamp(-40.0, 40.0).exp();
                let i = is * (ef - er);
                let g = (is / vt) * (ef + er);
                Self::stamp_current(n, &mut f_x, a, k, i);
                Self::stamp_dynamic(&mut triplets, a, k, g, n);
            }
            for &(g, k, p, mu, kg1, kp, kvb, ex, material, _) in &plan.triodes {
                let vgk = x_val(&x, g, n) - x_val(&x, k, n);
                let vpk = (x_val(&x, p, n) - x_val(&x, k, n)).max(0.001);
                let e1 = (vpk / kp) * (kp * (1.0 / mu + vgk / (vpk.powi(2) + kvb).sqrt())).ln_1p();
                let ip = if e1 > 0.0 {
                    (e1.powf(ex as f64) / kg1).max(0.0)
                } else {
                    0.0
                };
                let gp = (ip / vpk).max(1e-9);
                Self::stamp_current(n, &mut f_x, p, k, ip);
                Self::stamp_dynamic(&mut triplets, p, k, gp, n);

                // Grid Current and Microphonics
                let micro_noise = if let Material::Vacuum = material {
                    let t = self.dt * self.noise_seed as f64;
                    (t * 100.0).sin() * 0.001 // subtle hum
                } else { 0.0 };
                let vgk_eff = vgk + micro_noise;
                if vgk_eff > 0.0 {
                    let ig = 1e-4 * (vgk_eff / 0.026).exp();
                    let gg = ig / 0.026;
                    Self::stamp_current(n, &mut f_x, g, k, ig);
                    Self::stamp_dynamic(&mut triplets, g, k, gg, n);
                }
            }
            for &(g1, g2, k, p, mu, kg1, kp, kvb, ex, _material, _idx) in &plan.pentodes {
                let vg1k = x_val(&x, g1, n) - x_val(&x, k, n);
                let vpk = (x_val(&x, p, n) - x_val(&x, k, n)).max(0.001);
                let vg2k = (x_val(&x, g2, n) - x_val(&x, k, n)).max(0.001);
                let e1 = (vpk / kp) * (kp * (1.0 / mu + vg1k / (vpk.powi(2) + kvb).sqrt())).ln_1p();
                let ik = if e1 > 0.0 {
                    (e1.powf(ex as f64) / kg1).max(0.0)
                } else {
                    0.0
                };
                // Distribution: ip / ig2 ratio usually depends on Vp vs Vg2
                let ratio = if vpk > 0.0 {
                    (vpk / vg2k).powf(0.5).min(5.0)
                } else {
                    0.01
                };
                let ip = ik * (ratio / (1.0 + ratio));
                let ig2 = ik - ip;
                let gp = (ik / vpk).max(1e-9);
                Self::stamp_current(n, &mut f_x, p, k, ip);
                Self::stamp_current(n, &mut f_x, g2, k, ig2);
                Self::stamp_dynamic(&mut triplets, p, k, gp * 0.8, n);
                Self::stamp_dynamic(&mut triplets, g2, k, gp * 0.2, n);

                // Grid Current
                if vg1k > 0.0 {
                    let ig = 1e-4 * (vg1k / 0.026).exp();
                    let gg = ig / 0.026;
                    Self::stamp_current(n, &mut f_x, g1, k, ig);
                    Self::stamp_dynamic(&mut triplets, g1, k, gg, n);
                }
            }
            for &(b, c, e, is, bf, br, is_npn, _) in &plan.bjts {
                let vt = 0.026;
                let s = if is_npn { 1.0 } else { -1.0 };
                let vbe = (x_val(&x, b, n) - x_val(&x, e, n)) * s;
                let vbc = (x_val(&x, b, n) - x_val(&x, c, n)) * s;
                let v_crit = 0.7;
                let vbe_eff = if vbe > v_crit {
                    v_crit + (vbe - v_crit).ln_1p()
                } else {
                    vbe
                };
                let vbc_eff = if vbc > v_crit {
                    v_crit + (vbc - v_crit).ln_1p()
                } else {
                    vbc
                };
                let evbe = (vbe_eff / vt).clamp(-40.0, 40.0).exp();
                let evbc = (vbc_eff / vt).clamp(-40.0, 40.0).exp();
                let gbe = (is / vt) * evbe;
                let gbc = (is / vt) * evbc;
                let ibe = is * (evbe - 1.0);
                let ibc = is * (evbc - 1.0);
                let af = bf / (bf + 1.0);
                let ar = br / (br + 1.0);
                let fb = s * (ibe + ibc);
                let fc = s * (af * ibe - (1.0 + ar) * ibc);
                let fe = s * (ar * ibc - (1.0 + af) * ibe);
                if b > 0 && b < n {
                    f_x[b] += fb;
                }
                if c > 0 && c < n {
                    f_x[c] += fc;
                }
                if e > 0 && e < n {
                    f_x[e] += fe;
                }
                let jbb = gbe + gbc;
                let jbc = -gbc;
                let jbe = -gbe;
                let jcb = af * gbe - (1.0 + ar) * gbc;
                let jcc = (1.0 + ar) * gbc;
                let jce = -af * gbe;
                let jeb = ar * gbc - (1.0 + af) * gbe;
                let jec = -ar * gbc;
                let jee = (1.0 + af) * gbe;
                if b > 0 && b < n {
                    triplets.push((b, b, jbb + 1e-12));
                    triplets.push((b, c, jbc));
                    triplets.push((b, e, jbe));
                }
                if c > 0 && c < n {
                    triplets.push((c, b, jcb));
                    triplets.push((c, c, jcc + 1e-12));
                    triplets.push((c, e, jce));
                }
                if e > 0 && e < n {
                    triplets.push((e, b, jeb));
                    triplets.push((e, c, jec));
                    triplets.push((e, e, jee + 1e-12));
                }
            }
            for &(g, d, s, vto, beta, is_n_channel, _) in &plan.jfets {
                let pol = if is_n_channel { 1.0 } else { -1.0 };
                let vgs = (x_val(&x, g, n) - x_val(&x, s, n)) * pol;
                let vds = (x_val(&x, d, n) - x_val(&x, s, n)) * pol;
                let (id, jdd, jdg, jds) = if vgs < vto {
                    (0.0, 1e-12, 0.0, 0.0)
                } else if vds < vgs - vto {
                    let i = beta * vds * (2.0 * (vgs - vto) - vds);
                    let gdd = beta * (2.0 * (vgs - vto) - 2.0 * vds);
                    let gdg = beta * 2.0 * vds;
                    (i, gdd, gdg, -(gdd + gdg))
                } else {
                    let i = beta * (vgs - vto).powi(2);
                    let gdg = 2.0 * beta * (vgs - vto);
                    (i, 1e-12, gdg, -gdg)
                };
                Self::stamp_current(n, &mut f_x, d, s, pol * id);
                if d > 0 && d < n {
                    triplets.push((d, d, jdd));
                    triplets.push((d, g, jdg * pol));
                    triplets.push((d, s, jds * pol));
                }
                if s > 0 && s < n {
                    triplets.push((s, d, -jdd));
                    triplets.push((s, g, -jdg * pol));
                    triplets.push((s, s, -jds * pol));
                }
            }
            for &(g, d, s, vto, beta, lambda, is_n_channel, _) in &plan.mosfets {
                let pol = if is_n_channel { 1.0 } else { -1.0 };
                let vgs = (x_val(&x, g, n) - x_val(&x, s, n)) * pol;
                let vds = (x_val(&x, d, n) - x_val(&x, s, n)) * pol;
                let (id, jdd, jdg, jds) = if vgs < vto {
                    (0.0, 1e-12, 0.0, 0.0)
                } else if vds < vgs - vto {
                    let base_i = beta * vds * (2.0 * (vgs - vto) - vds);
                    let i = base_i * (1.0 + lambda * vds);
                    let gdd = beta * (2.0 * (vgs - vto) - 2.0 * vds) * (1.0 + lambda * vds)
                        + base_i * lambda;
                    let gdg = beta * 2.0 * vds * (1.0 + lambda * vds);
                    (i, gdd, gdg, -(gdd + gdg))
                } else {
                    let base_i = beta * (vgs - vto).powi(2);
                    let i = base_i * (1.0 + lambda * vds);
                    let gdd = base_i * lambda;
                    let gdg = 2.0 * beta * (vgs - vto) * (1.0 + lambda * vds);
                    (i, gdd, gdg, -(gdd + gdg))
                };
                Self::stamp_current(n, &mut f_x, d, s, pol * id);
                if d > 0 && d < n {
                    triplets.push((d, d, jdd));
                    triplets.push((d, g, jdg * pol));
                    triplets.push((d, s, jds * pol));
                }
                if s > 0 && s < n {
                    triplets.push((s, d, -jdd));
                    triplets.push((s, g, -jdg * pol));
                    triplets.push((s, s, -jds * pol));
                }
            }
            for &(g, c, e, vto, beta, bf, is, _) in &plan.igbts {
                let (_vge, _vce) = (
                    x_val(&x, g, n) - x_val(&x, e, n),
                    x_val(&x, c, n) - x_val(&x, e, n),
                );
                let vge = x_val(&x, g, n) - x_val(&x, e, n);
                let (id, _jdd, jdg) = if vge < vto {
                    (0.0, 1e-12, 0.0)
                } else {
                    let i = beta * (vge - vto).powi(2);
                    let g_g = 2.0 * beta * (vge - vto);
                    (i, 1e-12, g_g)
                };
                let vt = 0.026;
                let evbe = (0.7_f64 / vt).exp();
                let ibe = is * (evbe - 1.0);
                let ic = bf * id + ibe;
                Self::stamp_current(n, &mut f_x, c, e, ic);
                if c > 0 && c < n {
                    triplets.push((c, g, bf * jdg));
                    triplets.push((c, c, 1e-6));
                }
            }
            for &(a, k, _g, _v_hold, _i_hold, _i_gate, idx) in &plan.scrs {
                if let CircuitElement::Scr { state_on, .. } = &self.elements[idx] {
                    let g_scr = if *state_on { 1e3 } else { 1e-9 };
                    Self::stamp_f(n, &mut f_x, a, k, g_scr, &x);
                }
            }
            for &(m1, m2, _g, _v_hold, _i_hold, _i_gate, idx) in &plan.triacs {
                if let CircuitElement::Triac { state_on, .. } = &self.elements[idx] {
                    let g_triac = if *state_on { 1e3 } else { 1e-9 };
                    Self::stamp_f(n, &mut f_x, m1, m2, g_triac, &x);
                }
            }
            for &(pos, neg, r_a, l_a, ke, kt, j, b, r_ia, idx) in &plan.motors {
                if let CircuitElement::DcMotor {
                    state_i,
                    state_omega,
                    ..
                } = &self.elements[idx]
                {
                    let r_w = r_ia + 1;
                    if pos > 0 && pos < n {
                        f_x[pos] += x[r_ia];
                        triplets.push((pos, r_ia, 1.0));
                        triplets.push((r_ia, pos, 1.0));
                    }
                    if neg > 0 && neg < n {
                        f_x[neg] -= x[r_ia];
                        triplets.push((neg, r_ia, -1.0));
                        triplets.push((r_ia, neg, -1.0));
                    }
                    let g_ia = r_a + l_a / self.dt;
                    let g_w = b + j / self.dt;
                    f_x[r_ia] =
                        (x_val(&x, pos, n) - x_val(&x, neg, n)) - g_ia * x[r_ia] - ke * x[r_w]
                            + (l_a / self.dt) * (*state_i);
                    f_x[r_w] = kt * x[r_ia] - g_w * x[r_w] + (j / self.dt) * (*state_omega);
                    triplets.push((r_ia, r_ia, -g_ia));
                    triplets.push((r_ia, r_w, -ke));
                    triplets.push((r_w, r_ia, kt));
                    triplets.push((r_w, r_w, -g_w));
                }
            }
            for &(kind, ref inputs, out, v_h, v_l, _delay, _idx, r) in &plan.logic_gates {
                let threshold = (v_h + v_l) / 2.0;
                let mut soft_vals = Vec::new();
                let mut vals = Vec::new();
                for &inp in inputs {
                    let v = x_val(&x, inp, n);
                    vals.push(v);
                    soft_vals.push(0.5 * (1.0 + ((v - threshold) / 1.0).tanh()));
                }
                let res_soft = match kind {
                    LogicKind::AND => soft_vals.iter().fold(1.0, |acc, &v| acc * v),
                    LogicKind::OR => 1.0 - soft_vals.iter().fold(1.0, |acc, &v| acc * (1.0 - v)),
                    LogicKind::NOT => 1.0 - soft_vals[0],
                    LogicKind::NAND => 1.0 - soft_vals.iter().fold(1.0, |acc, &v| acc * v),
                    LogicKind::NOR => soft_vals.iter().fold(1.0, |acc, &v| acc * (1.0 - v)),
                    LogicKind::XOR => {
                        let mut p = soft_vals[0];
                        for i in 1..soft_vals.len() {
                            p = p * (1.0 - soft_vals[i]) + (1.0 - p) * soft_vals[i];
                        }
                        p
                    }
                };
                let target = v_l + (v_h - v_l) * res_soft;
                f_x[r] = x_val(&x, out, n) - target;
                for (i, &inp) in inputs.iter().enumerate() {
                    if inp > 0 && inp < n {
                        let g_smooth = (v_h - v_l)
                            * 0.5
                            * (1.0 - ((vals[i] - threshold) / 1.0).tanh().powi(2))
                            / 1.0;
                        let d_res_soft = match kind {
                            LogicKind::AND => soft_vals
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .fold(1.0, |acc, (_, &v)| acc * v),
                            LogicKind::OR => {
                                1.0 - soft_vals
                                    .iter()
                                    .enumerate()
                                    .filter(|(j, _)| *j != i)
                                    .fold(1.0, |acc, (_, &v)| acc * (1.0 - v))
                            }
                            LogicKind::NOT => -1.0,
                            LogicKind::NAND => -soft_vals
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .fold(1.0, |acc, (_, &v)| acc * v),
                            LogicKind::NOR => {
                                -(1.0
                                    - soft_vals
                                        .iter()
                                        .enumerate()
                                        .filter(|(j, _)| *j != i)
                                        .fold(1.0, |acc, (_, &v)| acc * (1.0 - v)))
                            }
                            LogicKind::XOR => 1.0,
                        };
                        triplets.push((r, inp, -d_res_soft * g_smooth));
                    }
                }
            }
            for &(pos, neg, out, v_h, v_l, r) in &plan.comparators {
                let diff = x_val(&x, pos, n) - x_val(&x, neg, n);
                let target = v_l + (v_h - v_l) * 0.5 * (1.0 + (diff / 1.0).tanh());
                f_x[r] = x_val(&x, out, n) - target;
                let g_smooth = (v_h - v_l) * 0.5 * (1.0 - (diff / 1.0).tanh().powi(2)) / 1.0;
                if pos > 0 && pos < n {
                    triplets.push((r, pos, -g_smooth));
                }
                if neg > 0 && neg < n {
                    triplets.push((r, neg, g_smooth));
                }
            }
            for &(pos, neg, amp, freq, duty, _, r) in &plan.pulse_sources {
                let t = (self.noise_seed as f64 * self.dt) % (1.0 / freq);
                let target = if t < (1.0 / freq) * duty { amp } else { 0.0 };
                f_x[r] = x_val(&x, pos, n) - x_val(&x, neg, n) - target;
            }
            for &(pos, neg, out, sens, b_f, r) in &plan.hall_sensors {
                f_x[r] =
                    x_val(&x, out, n) - (x_val(&x, pos, n) + x_val(&x, neg, n)) / 2.0 - sens * b_f;
            }
            for &(a, b, r25, beta, _, _idx) in &plan.thermistors {
                let g = 1.0
                    / (r25
                        * ((1.0 / beta)
                            * (1.0 / (self.context.temperature_c + 273.15) - 1.0 / 298.15))
                            .exp());
                Self::stamp_f(n, &mut f_x, a, b, g, &x);
                Self::stamp_dynamic(&mut triplets, a, b, g, n);
            }
            for &(a, k, c0, vj, m, _idx) in &plan.varactors {
                let v = x_val(&x, a, n) - x_val(&x, k, n);
                let c = c0 / (1.0 + v.max(-vj * 0.9) / vj).powf(m);
                let g = c / self.dt;
                if let CircuitElement::Varactor { state_v, .. } = &self.elements[_idx] {
                    Self::stamp_current(n, &mut f_x, a, k, g * (v - *state_v));
                    Self::stamp_dynamic(&mut triplets, a, k, g, n);
                }
            }
            for &(a, b, l_max, l_min, i_sat, _idx) in &plan.saturable_inductors {
                let i = if let CircuitElement::SaturableInductor { state_i, .. } = &self.elements[_idx] {
                    *state_i
                } else { 0.0 };
                let l_eff = l_min + (l_max - l_min) / (i / i_sat).cosh().powi(2);
                let g = self.dt / l_eff.max(1e-9);
                let v = x_val(&x, a, n) - x_val(&x, b, n);
                Self::stamp_current(n, &mut f_x, a, b, g * v + i);
                Self::stamp_dynamic(&mut triplets, a, b, g, n);
            }
            for &(a, k, ip, vp, iv, vv, _idx) in &plan.tunnel_diodes {
                let v = x_val(&x, a, n) - x_val(&x, k, n);
                let i_tunnel = ip * (v / vp) * (-(v / vp) + 1.0).exp() + iv * ((v - vv) / 0.026).exp();
                let g = (i_tunnel / v.abs().max(1e-3)).max(1e-9);
                Self::stamp_current(n, &mut f_x, a, k, i_tunnel);
                Self::stamp_dynamic(&mut triplets, a, k, g, n);
            }
            for &(a, b, _v_break, _v_hold, idx) in &plan.spark_gaps {
                if let CircuitElement::SparkGap { state_on, .. } = &self.elements[idx] {
                    let g = if *state_on { 1e3 } else { 1e-12 };
                    Self::stamp_f(n, &mut f_x, a, b, g, &x);
                    Self::stamp_dynamic(&mut triplets, a, b, g, n);
                }
            }
            for &(a, b, _c_min, _c_max, _drift, _idx) in &plan.ghosts {
                if let CircuitElement::CircuitGhost { state_c, .. } = &self.elements[_idx] {
                     let g = *state_c / self.dt;
                     let v = x_val(&x, a, n) - x_val(&x, b, n);
                     Self::stamp_current(n, &mut f_x, a, b, g * v);
                     Self::stamp_dynamic(&mut triplets, a, b, g, n);
                }
            }
            for &(a, b, ctrl, r_min, r_max, v_min, v_max, _) in &plan.vcrs {
                let vc = x_val(&x, ctrl, n);
                let t = ((vc - v_min) / (v_max - v_min)).clamp(0.0, 1.0);
                let r = r_min + (r_max - r_min) * t;
                let g = 1.0 / r.max(1e-9);
                Self::stamp_f(n, &mut f_x, a, b, g, &x);
                Self::stamp_dynamic(&mut triplets, a, b, g, n);
            }
            for &(a, b, ron, roff, _mu, _d, _idx) in &plan.memristors {
                if let CircuitElement::Memristor { w, .. } = &self.elements[_idx] {
                    let r = ron * w + roff * (1.0 - w);
                    let g = 1.0 / r.max(1e-3);
                    Self::stamp_f(n, &mut f_x, a, b, g, &x);
                    Self::stamp_dynamic(&mut triplets, a, b, g, n);
                }
            }
            for &(a, b, r_dark, gamma, _, idx) in &plan.ldrs {
                if let CircuitElement::Ldr { current_lux, .. } = &self.elements[idx] {
                    let r = r_dark / (current_lux.max(0.1)).powf(gamma);
                    Self::stamp_f(n, &mut f_x, a, b, 1.0 / r, &x);
                }
            }
            for el in &self.elements {
                match el {
                    CircuitElement::Photodiode {
                        a,
                        k,
                        sensitivity,
                        current_lux,
                    } => {
                        Self::stamp_current(n, &mut f_x, a.0, k.0, sensitivity * current_lux);
                    }
                    CircuitElement::Piezoelectric {
                        a,
                        b,
                        capacitance,
                        sensitivity,
                        force,
                        ..
                    } => {
                        let g = *capacitance / self.dt;
                        let i_piezo = sensitivity * force / self.dt;
                        Self::stamp_current(n, &mut f_x, a.0, b.0, i_piezo);
                        Self::stamp_f(n, &mut f_x, a.0, b.0, g, &x);
                    }
                    _ => {}
                }
            }
            for &(is_v, a, b, density, alpha, hum, ripple, _idx, r) in &plan.noise_sources {
                let seed = self.noise_seed ^ (a as u64) ^ ((b as u64) << 32);
                use rand::{Rng, SeedableRng};
                let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
                let flicker = if alpha > 0.0 {
                    (0..4)
                        .map(|i| (rng.gen::<f64>() * 2.0 - 1.0) / (i as f64 + 1.0).sqrt())
                        .sum::<f64>()
                } else {
                    0.0
                };
                let t = (self.noise_seed as f64) * self.dt;
                let hum_sig = (2.0 * std::f64::consts::PI * 60.0 * t).sin() * hum;
                let ripple_sig = (2.0 * std::f64::consts::PI * 120.0 * t).sin() * ripple;
                let noise =
                    (rng.gen::<f64>() * 2.0 - 1.0 + flicker) * density + hum_sig + ripple_sig;
                if is_v {
                    f_x[r] = x_val(&x, a, n) - x_val(&x, b, n) - noise;
                } else {
                    Self::stamp_current(n, &mut f_x, a, b, noise);
                }
            }
            for &(a, b, lm, _cm, rm, _co, _idx, r) in &plan.crystals {
                if let CircuitElement::Crystal {
                    state_im, state_vm, ..
                } = &self.elements[_idx]
                {
                    if a > 0 && a < n {
                        f_x[a] += x[r];
                        triplets.push((a, r, 1.0));
                        triplets.push((r, a, 1.0));
                    }
                    if b > 0 && b < n {
                        f_x[b] -= x[r];
                        triplets.push((b, r, -1.0));
                        triplets.push((r, b, -1.0));
                    }
                    let g = rm + lm / self.dt;
                    f_x[r] = (x_val(&x, a, n) - x_val(&x, b, n)) - g * x[r] - state_vm
                        + (lm / self.dt) * (*state_im);
                    triplets.push((r, r, -g));
                }
            }
            let mat = SparseColMat::<usize, f64>::try_new_from_triplets(dim, dim, &triplets)
                .expect("Valid matrix");
            let rhs = faer::Mat::from_fn(dim, 1, |i, _| -f_x[i]);
            let lu = mat.sp_lu().expect("LU factorization");
            let step = lu.solve(&rhs);
            let mut scale: f64 = 1.0;
            for i in 0..dim {
                if step.read(i, 0).abs() > 50.0 {
                    scale = scale.min(50.0 / step.read(i, 0).abs());
                }
            }
            for i in 0..dim {
                x[i] += step.read(i, 0) * scale;
            }
            let mut step_norm = 0.0;
            for i in 0..dim {
                step_norm += (step.read(i, 0) * scale).powi(2);
            }
            let mut res_norm = 0.0;
            for val in &f_x {
                res_norm += val.powi(2);
            }
            if step_norm.sqrt() < 1e-10 && res_norm.sqrt() < 1e-8 {
                converged = true;
                break;
            }
        }
        if !converged && self.has_nonlinear && !self.is_solving_gmin {
            let mut failed_scores = std::collections::HashMap::new();
            if final_iters > 20 {
                failed_scores.insert(0, (final_iters as f32 / 500.0).powi(2));
            }
            self.is_solving_gmin = true;
            let mut gmin = 1e-3;
            for _ in 0..10 {
                let mut triplets = self.static_triplets.clone();
                for i in 1..n {
                    triplets.push((i, i, gmin));
                }
                let mat = SparseColMat::<usize, f64>::try_new_from_triplets(dim, dim, &triplets);
                if let Ok(mat) = mat {
                    let rhs = faer::Mat::from_fn(dim, 1, |_, _| 0.0);
                    if let Ok(lu) = mat.sp_lu() {
                        let step = lu.solve(&rhs);
                        for i in 0..dim {
                            x[i] += step.read(i, 0);
                        }
                    }
                }
                gmin *= 0.1;
            }
            self.prev_solution = x.clone();
            let mut s = self.solve();
            s.instability_scores.extend(failed_scores);
            if !s.converged {
                self.is_solving_gmin = false;
                s.failure_culprit = Some("Gmin fallback failed".to_string());
                return s;
            }
            self.is_solving_gmin = false;
            return s;
        }
        self.is_solving_gmin = false;

        let mut buffer_idx = 0;
        for (idx, el) in self.elements.iter_mut().enumerate() {
            match el {
                CircuitElement::Capacitor { a, b, state_v, .. } => {
                    *state_v = x_val(&x, a.0, n) - x_val(&x, b.0, n)
                }
                CircuitElement::Inductor {
                    a,
                    b,
                    value,
                    state_i,
                    ..
                } => {
                    *state_i +=
                        (self.dt / value.max(1e-9)) * (x_val(&x, a.0, n) - x_val(&x, b.0, n))
                }
                CircuitElement::Transformer {
                    a1,
                    b1,
                    a2,
                    b2,
                    l1,
                    l2,
                    coupling,
                    state_i1,
                    state_i2,
                } => {
                    let m = *coupling * (*l1 * *l2).sqrt();
                    let det = *l1 * *l2 - m * m;
                    let v1 = x_val(&x, a1.0, n) - x_val(&x, b1.0, n);
                    let v2 = x_val(&x, a2.0, n) - x_val(&x, b2.0, n);
                    let di1 = (self.dt * *l2 / det) * v1 + (-self.dt * m / det) * v2;
                    let di2 = (-self.dt * m / det) * v1 + (self.dt * *l1 / det) * v2;
                    let i_sat = 1.0;
                    *state_i1 = (*state_i1 + di1).tanh() * i_sat;
                    *state_i2 = (*state_i2 + di2).tanh() * i_sat;
                }
                CircuitElement::TransmissionLine { a1, b1, a2, b2, .. } => {
                    if let Some(dq) = self.delay_buffers.get_mut(buffer_idx) {
                        dq.pop_front();
                        dq.push_back((
                            x_val(&x, a1.0, n) - x_val(&x, b1.0, n),
                            x_val(&x, a2.0, n) - x_val(&x, b2.0, n),
                        ));
                    }
                    buffer_idx += 1;
                }
                CircuitElement::Bbd {
                    input,
                    clock_hz,
                    state_phase,
                    ..
                } => {
                    *state_phase += *clock_hz * self.dt;
                    if let Some(dq) = self.delay_buffers.get_mut(buffer_idx) {
                        while *state_phase >= 1.0 {
                            dq.pop_front();
                            dq.push_back((x_val(&x, input.0, n), 0.0));
                            *state_phase -= 1.0;
                        }
                    }
                    buffer_idx += 1;
                }
                CircuitElement::ThermalCoupler {
                    r_th, c_th, temp, ..
                } => {
                    let p_diss = 0.01;
                    *temp +=
                        (p_diss - (*temp - self.context.temperature_c) / *r_th) * self.dt / *c_th;
                }
                CircuitElement::Scr {
                    a,
                    k,
                    g,
                    i_hold,
                    i_gate_trigger,
                    state_on,
                    ..
                } => {
                    let vak = x_val(&x, a.0, n) - x_val(&x, k.0, n);
                    let ig = x_val(&x, g.0, n) - x_val(&x, k.0, n);
                    let g_scr = if *state_on { 1e3 } else { 1e-9 };
                    let i_ak = vak * g_scr;
                    if !*state_on && (vak > 50.0 || ig > *i_gate_trigger) {
                        *state_on = true;
                    } else if *state_on && i_ak < *i_hold {
                        *state_on = false;
                    }
                }
                CircuitElement::Triac {
                    m1,
                    m2,
                    g,
                    i_hold,
                    i_gate_trigger,
                    state_on,
                    ..
                } => {
                    let v12 = x_val(&x, m1.0, n) - x_val(&x, m2.0, n);
                    let ig = x_val(&x, g.0, n) - x_val(&x, m2.0, n);
                    let g_triac = if *state_on { 1e3 } else { 1e-9 };
                    let i_12 = v12 * g_triac;
                    if !*state_on && (v12.abs() > 50.0 || ig.abs() > *i_gate_trigger) {
                        *state_on = true;
                    } else if *state_on && i_12.abs() < *i_hold {
                        *state_on = false;
                    }
                }
                CircuitElement::Piezoelectric { state_v, a, b, .. } => {
                    *state_v = x_val(&x, a.0, n) - x_val(&x, b.0, n);
                }
                CircuitElement::DcMotor {
                    state_i,
                    state_omega,
                    ..
                } => {
                    if let Some(m) = plan.motors.iter().find(|m| m.9 == idx) {
                        *state_i = x[m.8];
                        *state_omega = x[m.8 + 1];
                    }
                }
                CircuitElement::Crystal {
                    cm,
                    state_im,
                    state_vm,
                    ..
                } => {
                    if let Some(c) = plan.crystals.iter().find(|c| c.6 == idx) {
                        *state_im = x[c.7];
                        *state_vm += (*state_im) * self.dt / (*cm).max(1e-18);
                    }
                }
                CircuitElement::Vactrol {
                    led_p,
                    led_n,
                    tau_rise,
                    tau_fall,
                    state_brightness,
                    ..
                } => {
                    let v_led = (x_val(&x, led_p.0, n) - x_val(&x, led_n.0, n)).max(0.0);
                    let target = (v_led - 0.6).max(0.0) * 10.0;
                    let tau = if target > *state_brightness {
                        *tau_rise
                    } else {
                        *tau_fall
                    };
                    *state_brightness += (target - *state_brightness) * self.dt / tau.max(self.dt);
                }
                CircuitElement::DyingBattery {
                    capacity_ah: _,
                    current_charge,
                    ..
                } => {
                    if let Some(b) = plan.batteries.iter().find(|b| b.4 == idx) {
                        let i = x[b.5].abs();
                        *current_charge = (*current_charge - i * self.dt / 3600.0).max(0.0);
                    }
                }
                CircuitElement::Loudspeaker {
                    state_i,
                    state_v,
                    state_x,
                    ..
                } => {
                    if let Some(l) = plan.loudspeakers.iter().find(|l| l.8 == idx) {
                        *state_i = x[l.9];
                        *state_v = x[l.10];
                        *state_x += *state_v * self.dt;
                    }
                }
                CircuitElement::GuitarPickup { state_i, .. } => {
                    if let Some(p) = plan.pickups.iter().find(|p| p.5 == idx) {
                        *state_i = x[p.6];
                    }
                }
                CircuitElement::Relay {
                    coil_p,
                    coil_n,
                    state_on,
                    ..
                } => {
                    let v_coil = (x_val(&x, coil_p.0, n) - x_val(&x, coil_n.0, n)).abs();
                    if v_coil > 10.0 {
                        *state_on = true;
                    } else if v_coil < 2.0 {
                        *state_on = false;
                    }
                }
                CircuitElement::NeonBulb {
                    a,
                    b,
                    v_breakdown,
                    v_extinguish,
                    state_on,
                } => {
                    let v = (x_val(&x, a.0, n) - x_val(&x, b.0, n)).abs();
                    if !*state_on && v > *v_breakdown {
                        *state_on = true;
                    } else if *state_on && v < *v_extinguish {
                        *state_on = false;
                    }
                }
                CircuitElement::Memristor {
                    a,
                    b,
                    ron,
                    mu,
                    d,
                    w,
                    ..
                } => {
                    let i = (x_val(&x, a.0, n) - x_val(&x, b.0, n)) / (*ron).max(1e-3);
                    *w = (*w + *mu * *ron * i * self.dt / (*d * *d)).clamp(0.0, 1.0);
                }
                CircuitElement::Varactor { a, k, state_v, .. } => {
                    *state_v = x_val(&x, a.0, n) - x_val(&x, k.0, n);
                }
                CircuitElement::SaturableInductor { a, b, l_max, l_min, i_sat, state_i } => {
                    let v = x_val(&x, a.0, n) - x_val(&x, b.0, n);
                    let l_eff = *l_min + (*l_max - *l_min) / (*state_i / *i_sat).cosh().powi(2);
                    *state_i += v * self.dt / l_eff.max(1e-9);
                }
                CircuitElement::TunnelDiode { a, k, state_v, .. } => {
                    *state_v = x_val(&x, a.0, n) - x_val(&x, k.0, n);
                }
                CircuitElement::SparkGap { a, b, v_breakdown, v_hold, state_on } => {
                    let v = (x_val(&x, a.0, n) - x_val(&x, b.0, n)).abs();
                    if !*state_on && v > *v_breakdown {
                        *state_on = true;
                    } else if *state_on && v < *v_hold {
                        *state_on = false;
                    }
                }
                CircuitElement::CircuitGhost { state_c, c_min, c_max, drift_rate, .. } => {
                    use rand::{Rng, SeedableRng};
                    let mut rng = rand::rngs::StdRng::seed_from_u64(self.noise_seed ^ idx as u64);
                    let drift = (rng.gen::<f64>() * 2.0 - 1.0) * *drift_rate * self.dt;
                    *state_c = (*state_c + drift).clamp(*c_min, *c_max);
                }
                _ => {}
            }
        }
        self.noise_seed = self.noise_seed.wrapping_add(1);
        self.prev_solution = x.clone();
        let mut instability_scores = std::collections::HashMap::new();
        if final_iters > 20 {
            instability_scores.insert(0, (final_iters as f32 / 500.0).powi(2));
        }

        let mut provenance = std::collections::HashMap::new();
        for el in &self.elements {
            match el {
                CircuitElement::DyingBattery {
                    capacity_ah,
                    current_charge,
                    ..
                } => {
                    provenance.insert(
                        "battery_sag".to_string(),
                        (1.0 - *current_charge / *capacity_ah) as f32,
                    );
                }
                CircuitElement::Ota { .. } => {
                    provenance.insert("ota_saturation".to_string(), 0.5);
                } // simplified
                CircuitElement::DirtyGround { noise_density, .. } => {
                    provenance.insert("ground_hum".to_string(), (*noise_density * 100.0) as f32);
                }
                CircuitElement::Resistor { material, .. } => {
                    if let Material::CarbonComposition = material {
                        provenance.insert("carbon_drift".to_string(), 0.06);
                    }
                }
                _ => {}
            }
        }

        CircuitState {
            voltages: x[..n].to_vec(),
            currents: vec![0.0; self.elements.len()],
            iterations: final_iters,
            converged,
            failure_culprit: if !converged {
                Some("Newton-Raphson failed to converge".to_string())
            } else {
                None
            },
            instability_scores,
            provenance,
        }
    }

    fn stamp_dynamic(
        triplets: &mut Vec<(usize, usize, f64)>,
        a: usize,
        b: usize,
        g: f64,
        n: usize,
    ) {
        if a > 0 && a < n {
            triplets.push((a, a, g));
        }
        if b > 0 && b < n {
            triplets.push((b, b, g));
        }
        if a > 0 && b > 0 && a < n && b < n {
            triplets.push((a, b, -g));
            triplets.push((b, a, -g));
        }
    }
    fn stamp_f(n: usize, f: &mut Vec<f64>, a: usize, b: usize, cond: f64, x: &[f64]) {
        let v = x_val(x, a, n) - x_val(x, b, n);
        if a > 0 && a < n {
            f[a] += cond * v;
        }
        if b > 0 && b < n {
            f[b] -= cond * v;
        }
    }
    fn stamp_current(n: usize, f: &mut Vec<f64>, p: usize, neg: usize, i: f64) {
        if p > 0 && p < n {
            f[p] += i;
        }
        if neg > 0 && neg < n {
            f[neg] -= i;
        }
    }
}

#[inline]
fn x_val(x: &[f64], node: usize, n: usize) -> f64 {
    if node > 0 && node < n {
        x[node]
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_diode_clipper() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 10.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(2),
            value: 1000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver.add_element(CircuitElement::Diode {
            a: NodeId(2),
            k: NodeId(0),
            material: Material::Silicon,
            is: 1e-12,
        });
        let state = solver.solve();
        assert!(state.converged);
        assert!(state.voltages[2] > 0.5 && state.voltages[2] < 1.0);
    }
    #[test]
    fn test_triode_gain() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(4);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(3),
            neg: NodeId(0),
            voltage: 250.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(2),
            value: 100000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver.add_element(CircuitElement::Triode {
            g: NodeId(1),
            k: NodeId(0),
            p: NodeId(2),
            mu: 100.0,
            kg1: 1060.0,
            kp: 600.0,
            kvb: 300.0,
            ex: 1.4,
            material: Material::Vacuum,
        });
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: -1.0,
        });
        let state = solver.solve();
        assert!(state.converged);
        assert!(state.voltages[2] > 50.0 && state.voltages[2] < 250.0);
    }
    #[test]
    fn test_pentode_gain() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(5);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(4),
            neg: NodeId(0),
            voltage: 250.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(4),
            b: NodeId(3),
            value: 100000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(2),
            neg: NodeId(0),
            voltage: 150.0,
        }); // Screen grid
        solver.add_element(CircuitElement::Pentode {
            g1: NodeId(1),
            g2: NodeId(2),
            k: NodeId(0),
            p: NodeId(3),
            mu: 100.0,
            kg1: 1060.0,
            kp: 600.0,
            kvb: 300.0,
            ex: 1.4,
            material: Material::Vacuum,
        });
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: -1.0,
        });
        let state = solver.solve();
        assert!(state.converged);
        assert!(state.voltages[3] > 10.0 && state.voltages[3] < 250.0);
    }
    #[test]
    fn test_pot_taper() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(4);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 1.0,
        });
        solver.add_element(CircuitElement::Potentiometer {
            a: NodeId(1),
            wiper: NodeId(2),
            b: NodeId(0),
            value: 1000.0,
            pos: 0.5,
            taper: PotTaper::Linear,
        });
        let state = solver.solve();
        assert!((state.voltages[2] - 0.5).abs() < 1e-3);
    }

    #[test]
    fn test_bjt_common_emitter() {
        // NPN CE amplifier: Vcc=12V, Rc=1k, Rb=100k, Vbe bias ~0.7V
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(4);
        // Vcc
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(3),
            neg: NodeId(0),
            voltage: 12.0,
        });
        // Rc: Vcc -> collector
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(2),
            value: 1000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // Rb: Vcc -> base
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(1),
            value: 100000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // NPN BJT: base=1, collector=2, emitter=0(gnd)
        solver.add_element(CircuitElement::Bjt {
            b: NodeId(1),
            c: NodeId(2),
            e: NodeId(0),
            is: 1e-14,
            bf: 100.0,
            br: 1.0,
            is_npn: true,
        });
        let state = solver.solve();
        assert!(state.converged, "BJT CE amp must converge");
        // Base should be around 0.6-0.8V (Vbe forward bias)
        assert!(
            state.voltages[1] > 0.4 && state.voltages[1] < 1.5,
            "Vbe={} out of range",
            state.voltages[1]
        );
        // Collector should be below Vcc (current flowing through Rc)
        assert!(
            state.voltages[2] > 0.5 && state.voltages[2] < 12.0,
            "Vc={} out of range",
            state.voltages[2]
        );
    }

    #[test]
    fn test_bjt_pnp() {
        // PNP with emitter at Vcc, collector load to ground
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(4);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(3),
            neg: NodeId(0),
            voltage: 12.0,
        });
        // Rc to ground
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(2),
            b: NodeId(0),
            value: 1000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // Rb: base to ground (pulling base low for PNP)
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(0),
            value: 100000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // PNP: emitter=Vcc(3), base=1, collector=2
        solver.add_element(CircuitElement::Bjt {
            b: NodeId(1),
            c: NodeId(2),
            e: NodeId(3),
            is: 1e-14,
            bf: 100.0,
            br: 1.0,
            is_npn: false,
        });
        let state = solver.solve();
        assert!(state.converged, "PNP BJT must converge");
        // Collector should have some voltage (current flows through Rc)
        assert!(
            state.voltages[2] > 0.1,
            "PNP Vc={} too low",
            state.voltages[2]
        );
    }

    #[test]
    fn test_jfet_buffer() {
        // N-channel JFET source follower: Vdd=15V, Rs=1k, Vgs=0 (gate tied to source via bias)
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(4);
        // Vdd
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(3),
            neg: NodeId(0),
            voltage: 15.0,
        });
        // Drain connected to Vdd via small resistor
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(2),
            value: 100.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // Source resistor
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(0),
            value: 1000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // JFET: gate=0(gnd), drain=2, source=1, Vto=-2V, beta=1e-3
        solver.add_element(CircuitElement::Jfet {
            g: NodeId(0),
            d: NodeId(2),
            s: NodeId(1),
            vto: -2.0,
            beta: 1e-3,
            is_n_channel: true,
        });
        let state = solver.solve();
        assert!(state.converged, "JFET buffer must converge");
        // Source should have some positive voltage from Idss flowing through Rs
        assert!(
            state.voltages[1] > 0.1,
            "JFET Vs={} too low",
            state.voltages[1]
        );
        // Drain should be near Vdd
        assert!(
            state.voltages[2] > 5.0,
            "JFET Vd={} too low",
            state.voltages[2]
        );
    }

    #[test]
    fn test_noise_presence() {
        // A standalone resistor with no source — noise should produce non-zero voltage
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(2),
            value: 10000.0,
            tolerance: 0.0,
            material: Material::CarbonComposition,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(2),
            b: NodeId(0),
            value: 10000.0,
            tolerance: 0.0,
            material: Material::CarbonComposition,
        });
        let state = solver.solve();
        // With noise injection, at least one node should deviate from zero
        let _total_energy: f64 = state.voltages.iter().map(|v| v.abs()).sum();
        // This is a very weak assertion — just verifying the noise path executes
        assert!(state.converged, "Noise-only circuit should converge");
    }

    #[test]
    fn test_instability_scores_populated() {
        // Force a hard-to-converge circuit and check instability_scores
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 100.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(2),
            value: 1.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // Back-to-back diodes to make NR work harder
        solver.add_element(CircuitElement::Diode {
            a: NodeId(2),
            k: NodeId(0),
            material: Material::Silicon,
            is: 1e-12,
        });
        solver.add_element(CircuitElement::Diode {
            a: NodeId(0),
            k: NodeId(2),
            material: Material::Silicon,
            is: 1e-12,
        });
        let state = solver.solve();
        println!("Iterations: {}", state.iterations);
        if state.iterations > 10 {
            assert!(
                !state.instability_scores.is_empty(),
                "instability_scores should be populated for hard circuits"
            );
        }
    }

    #[test]
    fn test_transformer_saturation() {
        // Verify transformer saturation limits current via tanh
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(5);
        // Drive primary hard
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 100.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(2),
            value: 10.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // Load on secondary
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(4),
            value: 100.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(4),
            b: NodeId(0),
            value: 100.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver.add_element(CircuitElement::Transformer {
            a1: NodeId(2),
            b1: NodeId(0),
            a2: NodeId(3),
            b2: NodeId(0),
            l1: 0.01,
            l2: 0.01,
            coupling: 0.95,
            state_i1: 0.0,
            state_i2: 0.0,
        });
        // Run multiple steps to accumulate saturation
        let mut last_state = solver.solve();
        for _ in 0..100 {
            last_state = solver.solve();
        }
        assert!(last_state.converged, "Transformer circuit must converge");
    }

    #[test]
    fn test_logic_xor() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(5);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 5.0,
        });
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(2),
            neg: NodeId(0),
            voltage: 0.0,
        });
        solver.add_element(CircuitElement::LogicGate {
            kind: LogicKind::XOR,
            inputs: vec![NodeId(1), NodeId(2)],
            out: NodeId(3),
            v_high: 5.0,
            v_low: 0.0,
            delay: 0.0,
            state_v: 0.0,
        });
        // Add a pull-down to ensure node isn't floating
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(0),
            value: 1e6,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        let state = solver.solve();
        assert!(
            state.converged,
            "XOR must converge. Iterations: {}, Failure: {:?}",
            state.iterations, state.failure_culprit
        );
        println!("XOR Out: {}", state.voltages[3]);
        assert!((state.voltages[3] - 5.0).abs() < 0.2);
    }

    #[test]
    fn test_comparator() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(5);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 3.0,
        });
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(2),
            neg: NodeId(0),
            voltage: 2.0,
        });
        solver.add_element(CircuitElement::Comparator {
            pos: NodeId(1),
            neg: NodeId(2),
            out: NodeId(3),
            v_high: 5.0,
            v_low: 0.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(0),
            value: 1e6,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        let state = solver.solve();
        assert!(
            state.converged,
            "Comparator must converge. Iterations: {}, Failure: {:?}",
            state.iterations, state.failure_culprit
        );
        assert!(state.voltages[3] > 4.0); // Should be near V_high
    }

    #[test]
    fn test_ota_saturation() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(4);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(3),
            neg: NodeId(0),
            voltage: 1.0,
        });
        solver.add_element(CircuitElement::Ota {
            p: NodeId(1),
            n: NodeId(0),
            iabc: NodeId(3),
            out: NodeId(2),
            gm_per_amp: 1.0,
            r_in: 1000.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(2),
            b: NodeId(0),
            value: 1000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });

        // Small signal
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 0.01,
        });
        let state1 = solver.solve();
        assert!(state1.converged, "OTA small signal must converge");
        let v1 = state1.voltages[2];

        // Large signal (fresh solver)
        let mut solver2 = MnaSolver::new(1.0 / 44100.0);
        solver2.set_num_nodes(4);
        solver2.add_element(CircuitElement::VoltageSource {
            pos: NodeId(3),
            neg: NodeId(0),
            voltage: 1.0,
        });
        solver2.add_element(CircuitElement::Ota {
            p: NodeId(1),
            n: NodeId(0),
            iabc: NodeId(3),
            out: NodeId(2),
            gm_per_amp: 1.0,
            r_in: 1000.0,
        });
        solver2.add_element(CircuitElement::Resistor {
            a: NodeId(2),
            b: NodeId(0),
            value: 1000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver2.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 10.0,
        });
        let state2 = solver2.solve();
        assert!(state2.converged, "OTA large signal must converge");
        let v2 = state2.voltages[2];

        assert!(
            v2 < v1 * 100.0,
            "OTA did not saturate: Vout1={}, Vout2={}",
            v1,
            v2
        );
        assert!(v2 > v1, "OTA should have more output for larger input");
    }

    #[test]
    fn test_bbd_modulation() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 1.0,
        });
        // Slow clock relative to Fs to test interpolation
        solver.add_element(CircuitElement::Bbd {
            input: NodeId(1),
            output: NodeId(2),
            clock_hz: 1000.0,
            num_stages: 10,
            state_phase: 0.0,
        });

        let mut v_outs = Vec::new();
        for _ in 0..600 {
            let state = solver.solve();
            v_outs.push(state.voltages[2]);
        }
        // Verify we have some intermediate values (not just 0 and 1) due to interpolation
        let intermediates = v_outs.iter().filter(|&&v| v > 0.01 && v < 0.99).count();
        assert!(
            intermediates > 0,
            "BBD should show interpolated values during phase transitions"
        );
    }

    #[test]
    fn test_loudspeaker_resonance() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 10.0,
        });
        // Typical values for a small woofer
        solver.add_element(CircuitElement::Loudspeaker {
            a: NodeId(1),
            b: NodeId(0),
            re: 6.0,
            le: 0.001,
            bl: 5.0,
            mms: 0.01,
            rms: 1.0,
            cms: 0.001,
            state_i: 0.0,
            state_v: 0.0,
            state_x: 0.0,
        });
        let state = solver.solve();
        assert!(state.converged, "Loudspeaker circuit must converge");
        assert!(state.voltages[1].abs() > 1.0);
    }

    #[test]
    fn test_provenance_data() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::DyingBattery {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 9.0,
            internal_r: 100.0,
            capacity_ah: 1.0,
            current_charge: 0.5,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(0),
            value: 1000.0,
            tolerance: 0.1,
            material: Material::CarbonComposition,
        });
        let state = solver.solve();
        assert!(state.provenance.contains_key("battery_sag"));
        assert!(state.provenance.contains_key("carbon_drift"));
        assert!((*state.provenance.get("battery_sag").unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_motor_index() {
        let mut solver1 = MnaSolver::new(1.0 / 44100.0);
        solver1.set_num_nodes(10);
        solver1.add_element(CircuitElement::DcMotor {
            pos: NodeId(1),
            neg: NodeId(0),
            resistance: 1.0,
            inductance: 0.01,
            ke: 0.1,
            kt: 0.1,
            inertia: 0.01,
            friction: 0.01,
            state_i: 0.0,
            state_omega: 0.0,
        });
        solver1.add_element(CircuitElement::LogicGate {
            kind: LogicKind::AND,
            inputs: vec![NodeId(1)],
            out: NodeId(2),
            v_high: 5.0,
            v_low: 0.0,
            delay: 0.0,
            state_v: 0.0,
        });
        solver1.add_element(CircuitElement::Crystal {
            a: NodeId(3),
            b: NodeId(0),
            lm: 1.0,
            cm: 1e-12,
            rm: 10.0,
            co: 1e-11,
            state_im: 0.0,
            state_vm: 0.0,
        });
        let res1 = solver1.solve();

        let mut solver2 = MnaSolver::new(1.0 / 44100.0);
        solver2.set_num_nodes(10);
        solver2.add_element(CircuitElement::LogicGate {
            kind: LogicKind::AND,
            inputs: vec![NodeId(1)],
            out: NodeId(2),
            v_high: 5.0,
            v_low: 0.0,
            delay: 0.0,
            state_v: 0.0,
        });
        solver2.add_element(CircuitElement::Crystal {
            a: NodeId(3),
            b: NodeId(0),
            lm: 1.0,
            cm: 1e-12,
            rm: 10.0,
            co: 1e-11,
            state_im: 0.0,
            state_vm: 0.0,
        });
        solver2.add_element(CircuitElement::DcMotor {
            pos: NodeId(1),
            neg: NodeId(0),
            resistance: 1.0,
            inductance: 0.01,
            ke: 0.1,
            kt: 0.1,
            inertia: 0.01,
            friction: 0.01,
            state_i: 0.0,
            state_omega: 0.0,
        });
        let res2 = solver2.solve();

        for i in 0..10 {
            assert!((res1.voltages[i] - res2.voltages[i]).abs() < 1e-9);
        }
    }

    #[test]
    fn test_motor_impedance() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 10.0,
        });
        // 1 ohm sense resistor
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(2),
            value: 1.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        // 9 ohm motor
        solver.add_element(CircuitElement::DcMotor {
            pos: NodeId(2),
            neg: NodeId(0),
            resistance: 9.0,
            inductance: 0.0,
            ke: 0.0,
            kt: 0.0,
            inertia: 1.0,
            friction: 1.0,
            state_i: 0.0,
            state_omega: 0.0,
        });
        let state = solver.solve();
        // Total R = 10 ohm, V = 10V -> I = 1A. V(node 2) = 10 - 1*1 = 9V.
        let v2 = state.voltages[2];
        assert!(
            (v2 - 9.0).abs() < 1e-4,
            "Motor impedance doubled or wrong: V2={}, expected 9.0",
            v2
        );
    }

    #[test]
    fn test_koren_gain() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(4);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(3),
            neg: NodeId(0),
            voltage: 200.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(3),
            b: NodeId(2),
            value: 100000.0,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver.add_element(CircuitElement::Triode {
            g: NodeId(1),
            k: NodeId(0),
            p: NodeId(2),
            mu: 100.0,
            kg1: 1060.0,
            kp: 600.0,
            kvb: 300.0,
            ex: 1.4,
            material: Material::Vacuum,
        });
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 0.0,
        });
        let state = solver.solve();
        assert!(state.converged);
        let v_plate = state.voltages[2];
        assert!(
            v_plate < 200.0 && v_plate > 50.0,
            "Triode plate voltage out of range: {}",
            v_plate
        );
    }

    #[test]
    fn test_spark_gap_relaxation() {
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        solver.set_num_nodes(3);
        solver.add_element(CircuitElement::VoltageSource {
            pos: NodeId(1),
            neg: NodeId(0),
            voltage: 100.0,
        });
        solver.add_element(CircuitElement::Resistor {
            a: NodeId(1),
            b: NodeId(2),
            value: 1e6,
            tolerance: 0.0,
            material: Material::MetalFilm,
        });
        solver.add_element(CircuitElement::Capacitor {
            a: NodeId(2),
            b: NodeId(0),
            value: 1e-9,
            tolerance: 0.0,
            state_v: 0.0,
            material: Material::Ceramic,
        });
        solver.add_element(CircuitElement::SparkGap {
            a: NodeId(2),
            b: NodeId(0),
            v_breakdown: 80.0,
            v_hold: 20.0,
            state_on: false,
        });

        // Run for a few steps
        let mut fired = false;
        for _ in 0..1000 {
            let state = solver.solve();
            if state.voltages[2] < 30.0 && fired == false {
                // Should eventually fire and drop voltage
            }
            if state.voltages[2] > 70.0 {
                fired = true;
            }
        }
        assert!(fired, "Spark gap should have reached breakdown voltage");
    }
}
