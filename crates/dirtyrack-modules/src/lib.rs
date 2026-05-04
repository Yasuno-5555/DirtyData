//! dirtyrack-modules — Eurorack Module System
//!
//! モノラル信号がすべての基盤。
//! DirtyData の `[f32; 2]` ステレオ世界とは明確に境界を切る。
//!
//! # 信号規約
//!
//! | 信号タイプ | 電圧範囲 | 内部表現 |
//! |---|---|---|
//! | Audio | ±5V (10Vpp) | `f32` -5.0..5.0 |
//! | CV (1V/Oct) | 0V..10V | `f32` 0.0..10.0 |
//! | Gate | 0V / 5V | `f32` 0.0 or 5.0 |
//! | Trigger | 0V→5V pulse (1ms) | `f32` 0.0→5.0 1サンプル以上 |
//!
//! # サンプル精度保証
//!
//! すべての `RackDspNode::process()` は **1サンプル単位** で呼ばれる。
//! ブロック処理は存在しない。Gate/Triggerの立ち上がり検出は
//! サンプル境界で正確に行われることを契約とする。

pub mod attenuverter;
pub mod bernoulli;
pub mod biquad;
pub mod chaos;
pub mod clock;
pub mod clock_tree;
pub mod compressor;
pub mod delay;
pub mod drift;
pub mod drift_engine;
pub mod envelope;
pub mod input;
pub mod lfo;
pub mod logic;
pub mod mackeyglass;
pub mod macro_ctrl;
pub mod midi;
pub mod mixer;
pub mod mod_matrix;
pub mod noise;
pub mod output;
pub mod quantizer;
pub mod recorder;
pub mod registry;
pub mod renderer;
pub mod reverb;
pub mod runner;
pub mod saturation;
pub mod scope;
pub mod sequencer;
pub mod sh;
pub mod signal;
pub mod switch;
pub mod vca;
pub mod vcf;
pub mod vco;
pub mod wavefolder;
pub mod wdf;
pub mod wdf_filter;
pub mod xfade;
pub mod zdf_filter;

pub use registry::{ModuleDescriptor, ModuleRegistry};
pub use signal::{
    f32x4, AllocationPolicy, BuiltinModuleDescriptor, IntentBoundary, IntentClass, IntentMetadata,
    ModuleState, ModuleVisuals, PanelTexture, ParamDescriptor, ParamKind, ParamResponse,
    PatchEvent, PortDescriptor, PortDirection, ProvenanceZone, RackDspNode, RackProcessContext,
    SeedScope, SignalType,
};
