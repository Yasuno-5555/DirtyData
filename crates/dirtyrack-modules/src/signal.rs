//! Signal Interface Bridge — Re-exporting from SDK
//!
//! 内蔵モジュールも SDK の定義に従うことで、サードパーティ製との完全な互換性を保証。

pub use dirtyrack_sdk::{
    f32x4, EngineStats, ForensicData, ImperfectionData, ModuleDescriptor as BuiltinModuleDescriptor,
    ModuleVisuals, PanelTexture, ParamDescriptor, ParamKind, ParamResponse, PortDescriptor,
    PortDirection, RackDspNode, RackProcessContext, SignalType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SeedScope {
    Global(u64),
    Module(u64),
    Voice(u64),
}

/// 意図・鑑識用の型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IntentClass {
    Edit,
    Experiment,
    Fix,
    Structural,
    Performance,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum IntentBoundary {
    Begin,
    Intermediate,
    Commit(IntentClass, Option<IntentMetadata>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IntentMetadata {
    pub note: String,
    pub confidence_score: f32,
    pub hypothesis_tag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProvenanceZone {
    Safe,
    Quarantined,
    Unknown,
}

/// VCF 動作モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VcfMode {
    Clean,
    Character,
    Danger,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PatchEvent {
    ModuleAdded {
        id: String,
        stable_id: u64,
        ancestry: Option<u64>,
        zone: ProvenanceZone,
    },
    ModuleRemoved {
        stable_id: u64,
    },
    CableConnected {
        from_id: u64,
        from_port: String,
        to_id: u64,
        to_port: String,
    },
    CableDisconnected {
        to_id: u64,
        to_port: String,
    },
    ParamChanged {
        stable_id: u64,
        name: String,
        value_bits: u32,
        intent: IntentBoundary,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleState {
    pub params: std::collections::BTreeMap<String, f32>,
    pub bypassed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AllocationPolicy {
    Static,
    Dynamic,
}

pub const GATE_THRESHOLD: f32 = 1.0;

/// 周波数変換 (1V/Oct -> Hz)
pub fn voct_to_hz(voct: f32) -> f32 {
    16.35159783 * libm::powf(2.0, voct)
}

/// トリガー検出ユーティリティ
#[derive(Debug, Clone, Copy)]
pub struct TriggerDetector {
    last_val: f32,
}

impl TriggerDetector {
    pub fn new() -> Self {
        Self { last_val: 0.0 }
    }
    pub fn process(&mut self, val: f32) -> bool {
        let trig = val > 1.0 && self.last_val <= 1.0;
        self.last_val = val;
        trig
    }
}

/// ゲート追跡
#[derive(Debug, Clone, Copy)]
pub struct GateTracker {
    last_val: f32,
}
impl GateTracker {
    pub fn new() -> Self {
        Self { last_val: 0.0 }
    }
    pub fn is_high(&self, val: f32) -> bool {
        val > GATE_THRESHOLD
    }
    pub fn process(&mut self, val: f32) -> (bool, bool, bool) {
        let high = self.is_high(val);
        let last_high = self.is_high(self.last_val);
        let rising = high && !last_high;
        let falling = !high && last_high;
        self.last_val = val;
        (high, rising, falling)
    }
}

/// パラメータ平滑化ユーティリティ
pub struct SmoothedParam {
    current: f32,
    target: f32,
    coeff: f32,
}

impl SmoothedParam {
    pub fn new(initial: f32, sr: f32, ms: f32) -> Self {
        let coeff = if ms > 0.0 {
            libm::expf(-1.0 / (ms * 0.001 * sr))
        } else {
            0.0
        };
        Self {
            current: initial,
            target: initial,
            coeff,
        }
    }
    pub fn set(&mut self, target: f32) {
        self.target = target;
    }
    pub fn next(&mut self, jitter: f32) -> f32 {
        // jitter [-1, 1] を使って係数をわずかに変調 (反応速度の揺らぎ)
        let j_coeff = (self.coeff + jitter * 0.0001).clamp(0.0, 0.9999);
        self.current = j_coeff * self.current + (1.0 - j_coeff) * self.target;
        self.current
    }
}

/// 多項式近似 tanh (SIMD-native)
pub fn simd_tanh_x4(x: f32x4) -> f32x4 {
    let x2 = x * x;
    let a = x * (f32x4::from(1.0) + x2 * f32x4::from(0.16489087));
    let b = f32x4::from(1.0) + x2 * f32x4::from(0.4982926);
    a / b
}
