use dirtydata_core::types::ConfigSnapshot;

/// A helper for smoothing parameter changes using a One-Pole LPF.
#[derive(Clone)]
pub struct SmoothedValue {
    current: f32,
    target: f32,
    coeff: f32,
}

impl SmoothedValue {
    pub fn new(initial: f32, sample_rate: f32, time_constant_ms: f32) -> Self {
        let tau = time_constant_ms * 0.001;
        let coeff = 1.0 - (-1.0 / (sample_rate * tau)).exp();
        Self {
            current: initial,
            target: initial,
            coeff,
        }
    }
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }
    pub fn next(&mut self) -> f32 {
        self.current += self.coeff * (self.target - self.current);
        self.current
    }
    pub fn current(&self) -> f32 {
        self.current
    }
}

pub fn rk4_step<F>(state: &mut [f32], dt: f32, t: f32, derivative: F)
where
    F: Fn(&[f32], f32) -> Vec<f32>,
{
    let k1 = derivative(state, t);
    let mut s2 = state.to_vec();
    for i in 0..state.len() {
        s2[i] += k1[i] * dt * 0.5;
    }
    let k2 = derivative(&s2, t + dt * 0.5);
    let mut s3 = state.to_vec();
    for i in 0..state.len() {
        s3[i] += k2[i] * dt * 0.5;
    }
    let k3 = derivative(&s3, t + dt * 0.5);
    let mut s4 = state.to_vec();
    for i in 0..state.len() {
        s4[i] += k3[i] * dt;
    }
    let k4 = derivative(&s4, t + dt);
    for i in 0..state.len() {
        state[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
}

pub fn rk4_step_fixed<const N: usize, F>(state: &mut [f32; N], dt: f32, t: f32, derivative: F)
where
    F: Fn(&[f32; N], f32) -> [f32; N],
{
    let k1 = derivative(state, t);
    let mut s2 = *state;
    for i in 0..N {
        s2[i] += k1[i] * dt * 0.5;
    }
    let k2 = derivative(&s2, t + dt * 0.5);
    let mut s3 = *state;
    for i in 0..N {
        s3[i] += k2[i] * dt * 0.5;
    }
    let k3 = derivative(&s3, t + dt * 0.5);
    let mut s4 = *state;
    for i in 0..N {
        s4[i] += k3[i] * dt;
    }
    let k4 = derivative(&s4, t + dt);
    for i in 0..N {
        state[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
}

pub struct OscMessage {
    pub addr: String,
    pub args: Vec<rosc::OscType>,
}

pub struct ProcessContext<'a> {
    pub sample_rate: f32,
    pub global_sample_index: u64,
    pub crash_flag: Option<&'a std::sync::atomic::AtomicBool>,
    pub osc_tx: Option<&'a crossbeam_channel::Sender<OscMessage>>,
    pub convergence_info: Option<&'a dashmap::DashMap<dirtydata_core::types::StableId, usize>>,
    pub node_diagnostics:
        Option<&'a dashmap::DashMap<dirtydata_core::types::StableId, crate::DiagnosticRecord>>,
    pub node_id: Option<dirtydata_core::types::StableId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NodeState {
    Empty,
    Serialized(serde_json::Value),
}

impl NodeState {
    pub fn from_json<T: serde::Serialize>(data: T) -> Self {
        Self::Serialized(serde_json::to_value(data).unwrap_or(serde_json::Value::Null))
    }
    pub fn to_json<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        if let Self::Serialized(val) = self {
            serde_json::from_value(val.clone()).ok()
        } else {
            None
        }
    }
}

pub trait DspNode: Send + Sync + dyn_clone::DynClone {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    );
    fn update_parameter(&mut self, _param: &str, _value: f32) {}
    fn extract_state(&self) -> NodeState {
        NodeState::Empty
    }
    fn inject_state(&mut self, _state: &NodeState) {}
    fn as_mna_solver_mut(&mut self) -> Option<&mut dirtydata_dsp_circuit::MnaSolver> {
        None
    }
}

dyn_clone::clone_trait_object!(DspNode);
