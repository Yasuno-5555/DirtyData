//! Shared type definitions for DirtyData.
//!
//! Every struct here is a word in the DirtyData vocabulary.
//! If it's not defined here, it doesn't exist.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use std::fmt;

// ──────────────────────────────────────────────
// Identity types — ULID based
// "ID と Identity を混ぜるな。恋愛みたいな事故が起きる。"
// ──────────────────────────────────────────────

/// Stable identifier for entities (Nodes, Edges).
/// Uses ULID: sortable, human-readable, text-friendly.
/// Content identity is handled separately by BLAKE3 hashes.
/// Layer 5: Verification & Trust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Verification {
    pub null_test: bool,
    pub hash: String,
    pub trust_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub spec_version: String,
    pub last_revision: u64,
    pub timestamp: i64,
    pub verification: Verification,
    pub author_id: String,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StableId(pub ulid::Ulid);

impl StableId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl Default for StableId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for StableId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(ulid::Ulid::from_string(s)?))
    }
}

/// Patch identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PatchId(pub ulid::Ulid);

impl PatchId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl Default for PatchId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PatchId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(ulid::Ulid::from_string(s)?))
    }
}

/// Intent identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IntentId(pub ulid::Ulid);

impl IntentId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for IntentId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(ulid::Ulid::from_string(s)?))
    }
}

// ──────────────────────────────────────────────
// Primitives
// ──────────────────────────────────────────────

/// BLAKE3 hash — content identity, not entity identity.
pub type Hash = [u8; 32];

/// Monotonic revision counter for a Graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Revision(pub u64);

impl Revision {
    pub fn zero() -> Self {
        Self(0)
    }
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

/// Timestamp in microseconds since Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let dur = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self(dur.as_micros() as i64)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

// ──────────────────────────────────────────────
// §4.2 Execution Domains
// "Background は Sample を絶対に block してはならない。"
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionDomain {
    /// Hard real-time: DSP / audio thread.
    Sample,
    /// Soft real-time: FFT / loudness / analysis.
    Block,
    /// Build semantics: render / stem export / incremental rebuild.
    Timeline,
    /// Async heavy computation: ML / restoration / batch processing.
    Background,
}

// ──────────────────────────────────────────────
// Node classification
// ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    /// Audio file, input device.
    Source,
    /// EQ, compressor, gain, etc.
    Processor,
    /// Loudness meter, spectrum analyzer.
    Analyzer,
    /// Output, export target.
    Sink,
    /// Routing, bus, junction.
    Junction,
    /// External plugin — §8 Foreign Object Boundary.
    Foreign(String),
    /// Intent node — §3.1.
    Intent,
    /// Nested container node.
    SubGraph,
    /// Bridge node: Input from parent graph into subgraph.
    InputProxy,
    /// Bridge node: Output from subgraph back to parent.
    OutputProxy,
    /// Metadata carrier node.
    Metadata,
    /// Trust boundary marker — §8/§13.
    Boundary,
    /// Modularized MNA circuit node.
    CircuitModule {
        definition_id: StableId,
    },
}

/// §SSS: Circuit Definition for modularization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitDefinition {
    pub id: StableId,
    pub name: String,
    pub archetype: Option<CircuitArchetype>,
    /// The serialized elements of the circuit.
    pub elements_json: String, 
    pub input_mappings: BTreeMap<String, usize>,
    pub output_mappings: BTreeMap<String, usize>,
    /// The "DNA" — changes that led to this specific instance.
    pub mutation_history: Vec<MutationRecord>,
}

impl CircuitDefinition {
    pub fn hash(&self) -> crate::types::Hash {
        let json = serde_json::to_string(self).unwrap();
        *blake3::hash(json.as_bytes()).as_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitArchetype {
    Tb303Ladder,
    MinimoogFilter,
    JunoChorus,
    La2aCompressor,
    TapeEchoHead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MutationIntensity {
    Safe,        // Param Drift only
    Wild,        // Component Swaps
    Radioactive, // Topology Changes
    Forbidden,   // Stability Boundary Violations
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationReport {
    pub instability_score: f32, // 0..1 (1 is self-oscillation/chaos)
    pub novelty_score: f32,     // How much it deviates from "Legend"
    pub risk_level: f32,        // Chance of total failure
    pub warmth_delta: f32,
    pub dna_string: String,     // Human readable "系譜"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MutationType {
    /// Tier 1: Drift & Tolerance
    ParamDrift { index: usize, key: String, amount: f32 },
    /// Tier 2: Component Surgery
    ComponentSwap { index: usize, old_type: String, new_type: String },
    /// Tier 3: Topology Mutation
    TopologyChange { description: String, added_nodes: Vec<usize>, removed_nodes: Vec<usize> },
    /// Tier 4: Evolution (Goal oriented)
    Evolution { goal: String, generations: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationRecord {
    pub timestamp: Timestamp,
    pub intensity: MutationIntensity,
    pub changes: Vec<MutationType>,
    pub report: MutationReport,
}

// ──────────────────────────────────────────────
// Ports & Connections
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortDirection {
    Input,
    Output,
}

/// §SSS: Port Semantics — Human-readable roles for terminals.
/// "人類が読むためにある。input1 は罪。"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PortSemantic {
    // Basic
    #[default]
    None,
    Signal,
    Control,
    Gate,
    Trigger,
    Clock,
    Reset,
    
    // Circuit (MNA)
    VoltageIn,
    VoltageOut,
    CurrentIn,
    CurrentOut,
    Ground,
    
    // Electronic Components
    Anode,
    Cathode,
    Emitter,
    Base,
    Collector,
    GateTerminal, // Renamed to avoid collision
    Drain,
    Source,
    InPositive,
    InNegative,
    Feedback,
    
    // Musical
    Pitch,
    Velo,
    Modulation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PortPolarity {
    Unipolar,   // 0.0 to 1.0
    Bipolar,    // -1.0 to 1.0
    #[default]
    Unknown,
}

/// Data type flowing through ports.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    /// Audio samples.
    Audio { channels: u32 },
    /// Control signal (automation, modulation).
    Control,
    /// MIDI events.
    Midi,
    /// Frequency-domain data.
    Spectral { bins: u32 },
    /// Opaque binary blob.
    Blob,
    /// Metadata / annotations.
    Meta,
}

/// Typed port on a Node — §5.1.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedPort {
    pub name: String,
    pub direction: PortDirection,
    pub domain: ExecutionDomain,
    pub data_type: DataType,
    #[serde(default)]
    pub semantic: PortSemantic,
    #[serde(default)]
    pub polarity: PortPolarity,
}

/// Reference to a specific port on a specific node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortRef {
    pub node_id: StableId,
    pub port_name: String,
}

// ──────────────────────────────────────────────
// Configuration
// ──────────────────────────────────────────────

/// Configuration value — recursive, deterministically ordered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    String(String),
    List(Vec<ConfigValue>),
    Map(BTreeMap<String, ConfigValue>),
}

impl ConfigValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Immutable snapshot of a node's configuration.
/// BTreeMap guarantees deterministic iteration order.
pub type ConfigSnapshot = BTreeMap<String, ConfigValue>;

/// Delta between two configurations.
pub type ConfigDelta = BTreeMap<String, ConfigChange>;

/// A single config key change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigChange {
    pub old: Option<ConfigValue>,
    pub new: Option<ConfigValue>,
}

/// Metadata reference — points to a Metadata node, not a raw string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetadataRef(pub Option<StableId>);

impl Default for MetadataRef {
    fn default() -> Self {
        Self(None)
    }
}

/// Edge delta for ModifyEdge operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeDelta {
    pub source: Option<PortRef>,
    pub target: Option<PortRef>,
    pub kind: Option<crate::ir::EdgeKind>,
}

// ──────────────────────────────────────────────
// §13 Security Model — Trust Boundaries
// "AIが勝手に master bus に limiter 20個挿します。人類ならやる。"
// ──────────────────────────────────────────────

/// Trust level for operations and sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Verified internal operations.
    Trusted,
    /// Unknown origin, needs validation.
    Untrusted,
    /// Isolated for inspection.
    Quarantined,
    /// Awaiting human review.
    ReviewRequired,
}

/// Source of a patch or operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatchSource {
    /// Direct user action in GUI/CLI.
    UserDirect,
    /// User-edited DSL file.
    UserDsl,
    /// External script execution.
    Script(String),
    /// AI-generated PatchSet.
    AiGenerated(String),
    /// Third-party extension.
    Extension(String),
    /// System internal.
    System,
}

// ──────────────────────────────────────────────
// §3 Intent Graph — Constraint-based
// "音楽は最適化問題じゃない。制約付き妥協問題です。"
// ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterBound {
    pub target: String,
    pub range_start: f32,
    pub range_end: f32,
}

/// Constraint-based intent expression.
/// Must / Prefer / Avoid / Never — not weighted floats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntentConstraint {
    /// Non-negotiable requirement.
    Must(ParameterBound),
    /// Desirable if achievable.
    Prefer(ParameterBound),
    /// Should be minimized.
    Avoid(ParameterBound),
    /// Absolute prohibition.
    Never(ParameterBound),
}

/// Intent lifecycle status — §3.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentStatus {
    /// Sketch stage — hypothesis.
    Proposal,
    /// Committed — attached to patches.
    Attached,
    /// Attached and satisfied.
    Resolved,
    /// Explicitly discarded.
    Discarded,
    /// Kept for exploration.
    Exploratory,
}

// ──────────────────────────────────────────────
// §10 Observer Confidence Score
// "Dirty State として UI に明示される。"
// ──────────────────────────────────────────────

/// Confidence score for observed state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConfidenceScore {
    /// 0 — Unobservable.
    Unknown = 0,
    /// 30 — Nondeterministic plugin, inconsistent timestamps.
    Suspicious = 30,
    /// 70 — File timestamp, UI inference.
    Inferred = 70,
    /// 100 — Hash match, direct API completion.
    Verified = 100,
}
