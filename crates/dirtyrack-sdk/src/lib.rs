//! DirtyRack SDK — The Constitution of Signal

pub use wide::f32x4;

/// 信号タイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    Audio,
    VoltPerOct,
    UniCV,
    BiCV,
    Gate,
    Trigger,
    Clock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy)]
pub struct PortDescriptor {
    pub name: &'static str,
    pub direction: PortDirection,
    pub signal_type: SignalType,
    pub max_channels: u8,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Copy)]
pub enum ParamKind {
    Knob,
    Slider,
    Switch { positions: u8 },
    Button,
}

#[derive(Debug, Clone, Copy)]
pub enum ParamResponse {
    Immediate,
    Smoothed { ms: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct ParamDescriptor {
    pub name: &'static str,
    pub kind: ParamKind,
    pub response: ParamResponse,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub position: [f32; 2],
    pub unit: &'static str,
}

/// 実行コンテキスト
#[derive(Debug, Clone, Copy)]
pub struct RackProcessContext {
    pub sample_rate: f32,
    pub sample_index: u64,
    pub sample_time: f64,
    pub project_seed: u64,
    /// 経年劣化ノブ (0.0: 新品, 1.0: 20年物)
    pub aging: f32,
    /// アナログ的な「不完全さ」のデータ (16ボイス分)
    pub imperfection: ImperfectionData,
}

/// 鑑識用データ: モジュールの内部状態を詳細に可視化するための構造体
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForensicData {
    pub personality_offsets: [f32; 16],
    pub current_drift: [f32; 16],
    pub thermal_heat: [f32; 16],
    pub internal_state_summary: String,
    /// 16ボイス分の信号履歴 (可視化用)
    pub signal_trace: Option<Vec<[f32; 16]>>,
    /// エンジン統計データ
    pub stats: EngineStats,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct EngineStats {
    pub peak_db: f32,
    pub clipping_count: u64,
    pub denormal_count: u64,
    pub dc_offset: f32, // Moving average
    pub energy_delta: f32, // Change in energy
    pub aliasing_floor_db: f32,
}

impl Default for ForensicData {
    fn default() -> Self {
        Self {
            personality_offsets: [0.0; 16],
            current_drift: [0.0; 16],
            thermal_heat: [0.0; 16],
            internal_state_summary: String::new(),
            signal_trace: None,
            stats: EngineStats::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImperfectionData {
    /// 個体差: モジュール・ボイス固有の静的なオフセット値 [-1, 1]
    pub personality: [f32; 16],
    /// 熱ドリフト: 時間経過で変化する動的な揺らぎ [-1, 1]
    pub drift: [f32; 16],
}

impl Default for ImperfectionData {
    fn default() -> Self {
        Self {
            personality: [0.0; 16],
            drift: [0.0; 16],
        }
    }
}

impl RackProcessContext {
    pub fn new(sample_rate: f32, seed: u64) -> Self {
        Self {
            sample_rate,
            sample_index: 0,
            project_seed: seed,
            sample_time: 0.0,
            aging: 0.0,
            imperfection: ImperfectionData::default(),
        }
    }
    pub fn advance(&mut self) {
        self.sample_index += 1;
        self.sample_time = self.sample_index as f64 / self.sample_rate as f64;
    }
}

/// モジュールコア・トレイト
pub trait RackDspNode: Send + Sync {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        ctx: &RackProcessContext,
    );

    /// 鑑識用データの抽出
    fn get_forensic_data(&self) -> Option<ForensicData> {
        None
    }

    fn process_x4(
        &mut self,
        _inputs: &[f32x4],
        _outputs: &mut [f32x4],
        _params: &[f32x4],
        _ctx: &RackProcessContext,
    ) {
    }

    fn reset(&mut self) {}
    fn randomize(&mut self, _seed: u64) {}

    fn extract_state(&self) -> Option<Vec<u8>> {
        None
    }
    fn inject_state(&mut self, _data: &[u8]) {}

    fn on_midi(&mut self, _note: u8, _note_id: i32, _velocity: u8, _is_on: bool) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelTexture {
    BrushedAluminium,
    MatteBlack,
    VintageCream,
    IndustrialGrey,
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleVisuals {
    pub background_color: [u8; 3],
    pub text_color: [u8; 3],
    pub accent_color: [u8; 3],
    pub panel_texture: PanelTexture,
}

impl ModuleVisuals {
    pub const fn default_const() -> Self {
        Self {
            background_color: [35, 35, 35],
            text_color: [220, 220, 220],
            accent_color: [255, 100, 50],
            panel_texture: PanelTexture::MatteBlack,
        }
    }
}

impl Default for ModuleVisuals {
    fn default() -> Self {
        Self::default_const()
    }
}

/// 外部ロード用エントリポイント情報
pub struct ModuleDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub manufacturer: &'static str,
    pub hp_width: u32,
    pub visuals: ModuleVisuals,
    pub tags: &'static [&'static str],
    pub params: &'static [ParamDescriptor],
    pub ports: &'static [PortDescriptor],
    pub factory: fn(sample_rate: f32) -> Box<dyn RackDspNode>,
}

#[macro_export]
macro_rules! export_dirty_module {
    ($descriptor_fn:expr) => {
        #[no_mangle]
        pub extern "C" fn get_dirty_module_descriptor() -> *const $crate::ModuleDescriptor {
            $descriptor_fn() as *const _
        }
    };
}
