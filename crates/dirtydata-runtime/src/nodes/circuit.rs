use super::base::*;
use dirtydata_core::types::ConfigSnapshot;

#[derive(Clone)]
pub struct CircuitSandboxNode {
    solver: dirtydata_dsp_circuit::MnaSolver,
    probe_voltages: Vec<f32>,
}

impl CircuitSandboxNode {
    pub fn new(sample_rate: f32) -> Self {
        let mut solver = dirtydata_dsp_circuit::MnaSolver::new(1.0 / sample_rate as f64);
        solver.set_num_nodes(7);

        solver.add_element(dirtydata_dsp_circuit::CircuitElement::VoltageSource {
            pos: dirtydata_dsp_circuit::NodeId(1),
            neg: dirtydata_dsp_circuit::NodeId(0),
            voltage: 0.0,
        });

        solver.add_element(dirtydata_dsp_circuit::CircuitElement::VoltageSource {
            pos: dirtydata_dsp_circuit::NodeId(2),
            neg: dirtydata_dsp_circuit::NodeId(0),
            voltage: 0.7,
        });

        for i in 0..4 {
            let n_in = if i == 0 { 1 } else { 3 + i - 1 };
            let n_out = 3 + i;

            solver.add_element(dirtydata_dsp_circuit::CircuitElement::Diode {
                a: dirtydata_dsp_circuit::NodeId(n_in),
                k: dirtydata_dsp_circuit::NodeId(n_out),
                material: dirtydata_dsp_circuit::Material::Silicon,
                is: 1e-12,
            });
            solver.add_element(dirtydata_dsp_circuit::CircuitElement::Capacitor {
                a: dirtydata_dsp_circuit::NodeId(n_out),
                b: dirtydata_dsp_circuit::NodeId(0),
                value: 1e-8,
                state_v: 0.0,
                tolerance: 0.1,
                material: dirtydata_dsp_circuit::Material::Ceramic,
            });
        }

        Self {
            solver,
            probe_voltages: vec![0.0; 256],
        }
    }
}

impl DspNode for CircuitSandboxNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0) as f64;
        let cutoff = config
            .get("cutoff")
            .and_then(|v| v.as_float())
            .unwrap_or(0.7) as f64;

        if let Some(temp) = config.get("temp_c").and_then(|v| v.as_float()) {
            self.solver.context.temperature_c = temp as f64;
        }
        if let Some(drift) = config.get("drift").and_then(|v| v.as_float()) {
            self.solver.context.global_drift = drift as f64;
        }
        if let Some(vcc) = config.get("vcc").and_then(|v| v.as_float()) {
            self.solver.context.vcc = vcc as f64;
        }

        if let Some(dirtydata_dsp_circuit::CircuitElement::VoltageSource { voltage, .. }) =
            self.solver.add_element_dummy_handle(0)
        {
            *voltage = input;
        }
        if let Some(dirtydata_dsp_circuit::CircuitElement::VoltageSource { voltage, .. }) =
            self.solver.add_element_dummy_handle(1)
        {
            *voltage = cutoff;
        }

        let state = self.solver.solve();
        let out = state.voltages.get(6).copied().unwrap_or(0.0) as f32;

        if ctx.sample_rate > 0.0 {
            self.probe_voltages.rotate_left(1);
            if let Some(last) = self.probe_voltages.last_mut() {
                *last = out;
            }
        }

        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct CircuitModuleNode {
    solver: dirtydata_dsp_circuit::MnaSolver,
    input_v_sources: Vec<usize>,
    output_nodes: Vec<usize>,
}

impl CircuitModuleNode {
    pub fn new(sample_rate: f32, definition_json: &str) -> Option<Self> {
        let def: dirtydata_core::types::CircuitDefinition =
            serde_json::from_str(definition_json).ok()?;
        let elements: Vec<dirtydata_dsp_circuit::CircuitElement> =
            serde_json::from_str(&def.elements_json).ok()?;

        let mut solver = dirtydata_dsp_circuit::MnaSolver::new(1.0 / sample_rate as f64);

        let mut max_node = 0;
        for el in &elements {
            let nodes = match el {
                dirtydata_dsp_circuit::CircuitElement::Resistor { a, b, .. }
                | dirtydata_dsp_circuit::CircuitElement::Capacitor { a, b, .. }
                | dirtydata_dsp_circuit::CircuitElement::Inductor { a, b, .. }
                | dirtydata_dsp_circuit::CircuitElement::Diode { a, k: b, .. }
                | dirtydata_dsp_circuit::CircuitElement::Zener { a, k: b, .. }
                | dirtydata_dsp_circuit::CircuitElement::Switch { a, b, .. }
                | dirtydata_dsp_circuit::CircuitElement::VoltageSource { pos: a, neg: b, .. }
                | dirtydata_dsp_circuit::CircuitElement::CurrentSource { pos: a, neg: b, .. } => {
                    vec![*a, *b]
                }
                dirtydata_dsp_circuit::CircuitElement::Triode { g, k, p, .. } => vec![*g, *k, *p],
                dirtydata_dsp_circuit::CircuitElement::Bjt { b, c, e, .. } => vec![*b, *c, *e],
                dirtydata_dsp_circuit::CircuitElement::Jfet { g, d, s, .. } => vec![*g, *d, *s],
                dirtydata_dsp_circuit::CircuitElement::Transformer { a1, b1, a2, b2, .. } => {
                    vec![*a1, *b1, *a2, *b2]
                }
                dirtydata_dsp_circuit::CircuitElement::OpAmp { pos, neg, out, .. } => {
                    vec![*pos, *neg, *out]
                }
                dirtydata_dsp_circuit::CircuitElement::Potentiometer { a, wiper, b, .. } => {
                    vec![*a, *wiper, *b]
                }
                dirtydata_dsp_circuit::CircuitElement::ControlledSource {
                    target_a,
                    target_b,
                    control_a,
                    control_b,
                    ..
                } => vec![*target_a, *target_b, *control_a, *control_b],
                dirtydata_dsp_circuit::CircuitElement::TransmissionLine {
                    a1, b1, a2, b2, ..
                } => vec![*a1, *b1, *a2, *b2],
                dirtydata_dsp_circuit::CircuitElement::Memristor { a, b, .. } => vec![*a, *b],
                dirtydata_dsp_circuit::CircuitElement::ThermalCoupler { a, b, .. } => vec![*a, *b],
                _ => vec![],
            };
            for n in nodes {
                max_node = max_node.max(n.0);
            }
        }
        solver.set_num_nodes(max_node + 1);

        let mut input_v_sources = Vec::new();
        for (_, &node_id) in &def.input_mappings {
            let idx = solver.num_elements();
            solver.add_element(dirtydata_dsp_circuit::CircuitElement::VoltageSource {
                pos: dirtydata_dsp_circuit::NodeId(node_id),
                neg: dirtydata_dsp_circuit::NodeId(0),
                voltage: 0.0,
            });
            input_v_sources.push(idx);
        }

        for el in elements {
            solver.add_element(el);
        }

        let mut output_nodes = Vec::new();
        for (_, &node_id) in &def.output_mappings {
            output_nodes.push(node_id);
        }

        Some(Self {
            solver,
            input_v_sources,
            output_nodes,
        })
    }
}

impl DspNode for CircuitModuleNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        for (i, &v_idx) in self.input_v_sources.iter().enumerate() {
            if let Some(val) = inputs.get(i) {
                if let Some(dirtydata_dsp_circuit::CircuitElement::VoltageSource {
                    voltage, ..
                }) = self.solver.add_element_dummy_handle(v_idx)
                {
                    *voltage = *val as f64;
                }
            }
        }

        let state = self.solver.solve();

        if let (Some(info), Some(id)) = (ctx.convergence_info.as_ref(), ctx.node_id) {
            info.insert(id, state.iterations);
        }

        if !state.converged {
            if let (Some(diag), Some(id)) = (ctx.node_diagnostics.as_ref(), ctx.node_id) {
                diag.insert(
                    id,
                    crate::DiagnosticRecord {
                        message: state.failure_culprit.clone().unwrap_or_default(),
                        severity: crate::DiagnosticSeverity::Error,
                        timestamp: ctx.global_sample_index,
                    },
                );
            }
        }

        for (i, &node_id) in self.output_nodes.iter().enumerate() {
            if let Some(out_pair) = outputs.get_mut(i) {
                let v = state.voltages.get(node_id).copied().unwrap_or(0.0) as f32;
                *out_pair = [v, v];
            }
        }
    }
    fn as_mna_solver_mut(&mut self) -> Option<&mut dirtydata_dsp_circuit::MnaSolver> {
        Some(&mut self.solver)
    }
}

#[derive(Clone)]
pub struct DiodeClipperNode {
    inner: dirtydata_dsp_clipper::DiodeClipper,
}
impl DiodeClipperNode {
    pub fn new() -> Self {
        Self {
            inner: dirtydata_dsp_clipper::DiodeClipper::new(),
        }
    }
}
impl DspNode for DiodeClipperNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let drive = config
            .get("drive")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;
        let asymmetry = config
            .get("asymmetry")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32;

        let out = self.inner.process(input, drive, asymmetry);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct ZdfLadderNode {
    inner: dirtydata_dsp_zdf::ZdfLadder,
}
impl ZdfLadderNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_zdf::ZdfLadder::new(sample_rate),
        }
    }
}
impl DspNode for ZdfLadderNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let cutoff = config
            .get("cutoff")
            .and_then(|v| v.as_float())
            .unwrap_or(1000.0) as f32;
        let res = config
            .get("resonance")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32;
        let drive = config
            .get("drive")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32;

        let out = self.inner.process(input, cutoff, res, drive);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct SvfNode {
    inner: dirtydata_dsp_svf::Svf,
}
impl SvfNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_svf::Svf::new(sample_rate),
        }
    }
}
impl DspNode for SvfNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let cutoff = config
            .get("cutoff")
            .and_then(|v| v.as_float())
            .unwrap_or(1000.0) as f32;
        let q = config.get("q").and_then(|v| v.as_float()).unwrap_or(0.707) as f32;
        let mode = config.get("mode").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        let drive = config
            .get("drive")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32;

        let svf_out = if drive > 0.01 {
            self.inner.process_nonlinear(input, cutoff, q, drive)
        } else {
            self.inner.process(input, cutoff, q)
        };
        let out = match mode as i32 {
            0 => svf_out.lp,
            1 => svf_out.hp,
            2 => svf_out.bp,
            3 => svf_out.notch,
            4 => svf_out.ap,
            _ => svf_out.peak,
        };
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct WdfSimpleRcNode {
    inner: dirtydata_dsp_wdf::WdfSimpleRc,
}
impl WdfSimpleRcNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_wdf::WdfSimpleRc::new(1000.0, 1e-6, sample_rate),
        }
    }
}
impl DspNode for WdfSimpleRcNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let out = self.inner.process(input);
        for o in outputs {
            *o = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct WdfDiodeClipperNode {
    inner: dirtydata_dsp_wdf::WdfDiodeClipper,
}
impl WdfDiodeClipperNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            inner: dirtydata_dsp_wdf::WdfDiodeClipper::new(4700.0, 10e-9, sample_rate),
        }
    }
}
impl DspNode for WdfDiodeClipperNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        _config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let input = inputs.get(0).copied().unwrap_or(0.0);
        let out = self.inner.process(input);
        for o in outputs {
            *o = [out, out];
        }
    }
}
