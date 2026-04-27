//! Rack State & Rail Drawing
//!
//! ラックの状態管理とレール描画。
//!
//! # 決定論的創造環境の憲法
//! 1. すべてのアクションは `handle_action` を経由する。
//! 2. `Commit` インテントのみが永続ヒストリー (`event_queue`) に記録される。
//! 3. 浮動小数点は bitパターン (u32) でシリアライズする。
//! 4. ID はアトミックカウンターにより決して再利用されない。

use dirtyrack_modules::runner::{Connection, GraphSnapshot};
pub use dirtyrack_modules::{
    AllocationPolicy, IntentBoundary, IntentClass, IntentMetadata, ModuleDescriptor,
    ModuleRegistry, ModuleState, PatchEvent, PortDirection, ProvenanceZone, RackDspNode, SeedScope,
    SignalType,
};
use egui::{vec2, Color32, Painter, Pos2, Rect, Stroke, Vec2};
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub const HP_PIXELS: f32 = 15.0;
pub const RACK_HEIGHT: f32 = 380.0;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModBinding {
    pub source_stable_id: u64,
    pub source_port_idx: usize,
    pub amount: f32,
}

pub struct ModuleInstance {
    pub descriptor: Arc<ModuleDescriptor>,
    pub params: BTreeMap<String, f32>,
    pub param_modulations: BTreeMap<String, Vec<ModBinding>>,
    pub output_values: Vec<f32>,
    pub input_values: Vec<f32>,
    pub stable_id: u64,
    pub dsp: Box<dyn RackDspNode>,
    pub hp_position: f32,
    pub row: usize,
    pub bypassed: bool,
}

impl ModuleInstance {
    pub fn new(descriptor: Arc<ModuleDescriptor>, sample_rate: f32) -> Self {
        let mut params = BTreeMap::new();
        for p in &descriptor.params {
            params.insert(p.name.to_string(), p.default);
        }

        let in_count = descriptor
            .ports
            .iter()
            .filter(|p| p.direction == PortDirection::Input)
            .count();
        let out_count = descriptor
            .ports
            .iter()
            .filter(|p| p.direction == PortDirection::Output)
            .count();

        Self {
            descriptor: Arc::clone(&descriptor),
            params,
            param_modulations: BTreeMap::new(),
            output_values: vec![0.0; out_count],
            input_values: vec![0.0; in_count],
            stable_id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            dsp: (descriptor.factory)(sample_rate),
            hp_position: 0.0,
            row: 0,
            bypassed: false,
        }
    }

    pub fn world_rect(&self) -> Rect {
        let x = self.hp_position * HP_PIXELS;
        let y = self.row as f32 * (RACK_HEIGHT + 20.0);
        Rect::from_min_size(
            Pos2::new(x, y),
            vec2(self.descriptor.hp_width as f32 * HP_PIXELS, RACK_HEIGHT),
        )
    }
}

pub struct Cable {
    pub from_module: usize,
    pub from_port: String,
    pub to_module: usize,
    pub to_port: String,
    pub color: Color32,
    pub channels: u8,
}

#[derive(Debug, Clone)]
pub enum CableAction {
    StartDrag {
        module_idx: usize,
        port_name: String,
        is_output: bool,
    },
    EndDrag {
        pointer_pos: Pos2,
    },
    StartModuleDrag {
        module_idx: usize,
        press_pos: Pos2,
    },
    MoveModule {
        module_idx: usize,
        pointer_pos: Pos2,
    },
    DisconnectPort {
        module_idx: usize,
        port_name: String,
    },
    ParamUpdate {
        module_idx: usize,
        name: String,
        value: f32,
        intent: IntentBoundary,
    },
    RemoveModule {
        module_idx: usize,
    },
    ToggleBypass {
        module_idx: usize,
    },
    RandomizeParams {
        module_idx: usize,
    },
    ResetModule {
        module_idx: usize,
    },
    AddModMapping {
        target_module_idx: usize,
        param_name: String,
        src_stable_id: u64,
        src_port_idx: usize,
    },
    ClearModMappings {
        module_idx: usize,
        param_name: String,
    },
    InspectForensics {
        stable_id: u64,
    },
    SelectModule {
        stable_id: u64,
        additive: bool,
    },
    CopySelection,
    PasteSelection {
        pointer_pos: Pos2,
    },
    CancelDrag,
}

pub struct DraggingCable {
    pub from_module: usize,
    pub from_port: String,
    pub is_from_output: bool,
}

pub struct DraggingModule {
    pub module_idx: usize,
    pub offset: Vec2,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableModule {
    pub id: String,
    pub stable_id: u64,
    pub params: BTreeMap<String, f32>,
    pub param_modulations: BTreeMap<String, Vec<ModBinding>>,
    pub hp_position: f32,
    pub row: usize,
    pub bypassed: bool,
    pub dsp_state: Option<Vec<u8>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableCable {
    pub from_stable_id: u64,
    pub from_port: String,
    pub to_stable_id: u64,
    pub to_port: String,
    pub color: [u8; 4], // Color32 as RGBA
    pub channels: u8,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableRack {
    pub version: String,
    pub engine_hash: String,
    pub modules: Vec<SerializableModule>,
    pub cables: Vec<SerializableCable>,
    pub project_seed: u64,
    pub aging: f32,
    pub cable_opacity: f32,
    pub cable_tension: f32,
    pub causality_log: Vec<CausalityEvent>,
    pub snapshots: BTreeMap<String, BTreeMap<u64, BTreeMap<String, f32>>>,
}

pub struct RackState {
    pub modules: Vec<ModuleInstance>,
    pub cables: Vec<Cable>,
    pub dragging_cable: Option<DraggingCable>,
    pub dragging_module: Option<DraggingModule>,
    pub sample_rate: f32,
    pub project_seed: u64,
    pub aging: f32,
    pub cable_opacity: f32,
    pub cable_tension: f32,
    pub event_queue: Vec<PatchEvent>,
    pub schema_version: u32,
    /// Snapshots for Diff Viewer [snapshot_name] -> [module_stable_id] -> [param_name] -> value
    pub snapshots: BTreeMap<String, BTreeMap<u64, BTreeMap<String, f32>>>,
    pub snapshot_blend: f32, // 0.0 = A, 1.0 = B
    pub blend_targets: (String, String), // (SnapA, SnapB)
    pub selection: Vec<u64>, // List of stable_ids
    pub box_select_start: Option<Pos2>, // World position
    pub clipboard: Option<SerializableRack>,
    pub history: VecDeque<SerializableRack>,
    pub causality_log: Vec<CausalityEvent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CausalityEvent {
    pub timestamp: f64,
    pub description: String,
    pub event_type: String, // "PARAM", "SNAPSHOT", "DIVERGENCE", "FAILURE"
}

const MAX_HISTORY: usize = 100;

impl RackState {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            cables: Vec::new(),
            dragging_cable: None,
            dragging_module: None,
            sample_rate: 44100.0,
            project_seed: 0xDE7E_B11D,
            aging: 0.0,
            cable_opacity: 0.8,
            cable_tension: 0.15,
            event_queue: Vec::new(),
            schema_version: 2,
            snapshots: BTreeMap::new(),
            snapshot_blend: 0.0,
            blend_targets: ("A".to_string(), "B".to_string()),
            selection: Vec::new(),
            box_select_start: None,
            clipboard: None,
            history: VecDeque::with_capacity(MAX_HISTORY),
            causality_log: Vec::new(),
        }
    }

    pub fn log_event(&mut self, description: &str, event_type: &str, time: f64) {
        self.causality_log.push(CausalityEvent {
            timestamp: time,
            description: description.to_string(),
            event_type: event_type.to_string(),
        });
    }

    pub fn push_history(&mut self) {
        let serial = self.to_serializable();
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(serial);
    }

    pub fn take_snapshot(&mut self, name: &str) {
        let mut snap = BTreeMap::new();
        for m in &self.modules {
            snap.insert(m.stable_id, m.params.clone());
        }
        self.snapshots.insert(name.to_string(), snap);
        self.causality_log.push(CausalityEvent {
            timestamp: 0.0,
            event_type: "SNAPSHOT".to_string(),
            description: format!("Snapshot '{}' created", name),
        });
    }

    pub fn apply_blend(&mut self) {
        let (name_a, name_b) = &self.blend_targets;
        let t = self.snapshot_blend;
        
        let snap_a = if let Some(s) = self.snapshots.get(name_a) { s } else { return; };
        let snap_b = if let Some(s) = self.snapshots.get(name_b) { s } else { return; };

        for m in &mut self.modules {
            if let (Some(params_a), Some(params_b)) = (snap_a.get(&m.stable_id), snap_b.get(&m.stable_id)) {
                for (name, val_a) in params_a {
                    if let Some(val_b) = params_b.get(name) {
                        let blended = val_a * (1.0 - t) + val_b * t;
                        m.params.insert(name.clone(), blended);
                        
                        // Notify engine
                        self.event_queue.push(PatchEvent::ParamChanged {
                            stable_id: m.stable_id,
                            name: name.clone(),
                            value_bits: blended.to_bits(),
                            intent: IntentBoundary::Commit(IntentClass::Structural, None),
                        });
                    }
                }
            }
        }
    }

    pub fn hash_patch(&self) -> String {
        let serial = self.to_serializable();
        let json = serde_json::to_string(&serial).unwrap_or_default();
        blake3::hash(json.as_bytes()).to_hex().to_string()
    }

    pub fn to_serializable(&self) -> SerializableRack {
        SerializableRack {
            version: env!("CARGO_PKG_VERSION").to_string(),
            engine_hash: "TODO_CALC_DSP_HASH".to_string(),
            modules: self.modules.iter().map(|m| SerializableModule {
                id: m.descriptor.id.to_string(),
                stable_id: m.stable_id,
                params: m.params.clone(),
                param_modulations: m.param_modulations.clone(),
                hp_position: m.hp_position,
                row: m.row,
                bypassed: m.bypassed,
                dsp_state: m.dsp.extract_state(),
            }).collect(),
            cables: self.cables.iter().map(|c| {
                let from_stable = self.modules.get(c.from_module).map(|m| m.stable_id).unwrap_or(0);
                let to_stable = self.modules.get(c.to_module).map(|m| m.stable_id).unwrap_or(0);
                SerializableCable {
                    from_stable_id: from_stable,
                    from_port: c.from_port.clone(),
                    to_stable_id: to_stable,
                    to_port: c.to_port.clone(),
                    color: [c.color.r(), c.color.g(), c.color.b(), c.color.a()],
                    channels: c.channels,
                }
            }).collect(),
            project_seed: self.project_seed,
            aging: self.aging,
            cable_opacity: self.cable_opacity,
            cable_tension: self.cable_tension,
            causality_log: self.causality_log.clone(),
            snapshots: self.snapshots.clone(),
        }
    }

    pub fn from_serializable(serial: SerializableRack, registry: &ModuleRegistry, sample_rate: f32) -> Self {
        let mut modules = Vec::new();
        let mut max_stable_id = 0;

        for sm in serial.modules {
            if let Some(desc) = registry.find(&sm.id) {
                let mut inst = ModuleInstance::new(desc, sample_rate);
                inst.stable_id = sm.stable_id;
                inst.params = sm.params;
                inst.param_modulations = sm.param_modulations;
                inst.hp_position = sm.hp_position;
                inst.row = sm.row;
                inst.bypassed = sm.bypassed;
                if let Some(state) = sm.dsp_state {
                    inst.dsp.inject_state(&state);
                }
                if inst.stable_id > max_stable_id {
                    max_stable_id = inst.stable_id;
                }
                modules.push(inst);
            }
        }

        // Update global ID counter to avoid collisions
        NEXT_ID.store(max_stable_id + 1, Ordering::Relaxed);

        let mut stable_to_idx = BTreeMap::new();
        for (i, m) in modules.iter().enumerate() {
            stable_to_idx.insert(m.stable_id, i);
        }

        let mut cables = Vec::new();
        for sc in serial.cables {
            if let (Some(&from_idx), Some(&to_idx)) = (stable_to_idx.get(&sc.from_stable_id), stable_to_idx.get(&sc.to_stable_id)) {
                cables.push(Cable {
                    from_module: from_idx,
                    from_port: sc.from_port,
                    to_module: to_idx,
                    to_port: sc.to_port,
                    color: Color32::from_rgba_unmultiplied(sc.color[0], sc.color[1], sc.color[2], sc.color[3]),
                    channels: sc.channels,
                });
            }
        }

        Self {
            modules,
            cables,
            dragging_cable: None,
            dragging_module: None,
            sample_rate,
            project_seed: serial.project_seed,
            aging: serial.aging,
            cable_opacity: serial.cable_opacity,
            cable_tension: serial.cable_tension,
            event_queue: Vec::new(),
            schema_version: 2,
            snapshots: serial.snapshots,
            snapshot_blend: 0.0,
            blend_targets: (String::new(), String::new()),
            selection: Vec::new(),
            box_select_start: None,
            clipboard: None,
            history: VecDeque::with_capacity(MAX_HISTORY),
            causality_log: serial.causality_log,
        }
    }

    pub fn handle_action(&mut self, action: CableAction, registry: &ModuleRegistry, zoom: f32, pan: Vec2) {
        match action {
// ... (I'll add resolve_overlaps here after the handle_action block or inside it)
            CableAction::StartDrag {
                module_idx,
                port_name,
                is_output,
            } => {
                self.dragging_cable = Some(DraggingCable {
                    from_module: module_idx,
                    from_port: port_name,
                    is_from_output: is_output,
                });
            }
            CableAction::EndDrag { pointer_pos } => {
                if let Some(drag) = self.dragging_cable.take() {
                    let world_pos = (pointer_pos.to_vec2() - pan) / zoom;
                    let world_pos = world_pos.to_pos2();

                    if let Some((to_mod, to_port, is_to_output)) = self.find_port_at(world_pos) {
                        if drag.is_from_output != is_to_output {
                            let (src_mod, src_port, dst_mod, dst_port) = if drag.is_from_output {
                                (drag.from_module, drag.from_port, to_mod, to_port)
                            } else {
                                (to_mod, to_port, drag.from_module, drag.from_port)
                            };

                            if src_mod != dst_mod {
                                let color = crate::cable::CABLE_COLORS
                                    [self.cables.len() % crate::cable::CABLE_COLORS.len()];
                                let channels = self.modules[src_mod]
                                    .descriptor
                                    .ports
                                    .iter()
                                    .find(|p| p.name == src_port)
                                    .map(|p| p.max_channels)
                                    .unwrap_or(1);

                                self.cables.push(Cable {
                                    from_module: src_mod,
                                    from_port: src_port.clone(),
                                    to_module: dst_mod,
                                    to_port: dst_port.clone(),
                                    color,
                                    channels,
                                });
                                self.event_queue.push(PatchEvent::CableConnected {
                                    from_id: self.modules[src_mod].stable_id,
                                    from_port: src_port,
                                    to_id: self.modules[dst_mod].stable_id,
                                    to_port: dst_port,
                                });
                            }
                        }
                    }
                }
            }
            CableAction::DisconnectPort {
                module_idx,
                port_name,
            } => {
                let _stable_id = self.modules[module_idx].stable_id;
                self.cables.retain(|c| {
                    let match_from = c.from_module == module_idx && c.from_port == port_name;
                    let match_to = c.to_module == module_idx && c.to_port == port_name;
                    if match_from || match_to {
                        self.event_queue.push(PatchEvent::CableDisconnected {
                            to_id: self.modules[c.to_module].stable_id,
                            to_port: c.to_port.clone(),
                        });
                        false
                    } else {
                        true
                    }
                });
            }
            CableAction::ParamUpdate {
                module_idx,
                name,
                value,
                intent,
            } => {
                if let Some(m) = self.modules.get_mut(module_idx) {
                    m.params.insert(name.clone(), value);
                    if let IntentBoundary::Commit(_class, _) = intent {
                        self.causality_log.push(CausalityEvent {
                            timestamp: 0.0, // Should use real time if possible
                            event_type: "PARAM".to_string(),
                            description: format!("Module {} param '{}' -> {:.3}", m.descriptor.name, name, value),
                        });
                        self.event_queue.push(PatchEvent::ParamChanged {
                            stable_id: m.stable_id,
                            name,
                            value_bits: value.to_bits(),
                            intent,
                        });
                    }
                }
            }
            CableAction::StartModuleDrag {
                module_idx,
                press_pos,
            } => {
                let module = &self.modules[module_idx];
                let world_rect = module.world_rect();
                let screen_pos = (world_rect.min.to_vec2() * zoom + pan).to_pos2();
                self.dragging_module = Some(DraggingModule {
                    module_idx,
                    offset: screen_pos - press_pos,
                });
            }
            CableAction::MoveModule {
                module_idx: _,
                pointer_pos,
            } => {
                if let Some(drag) = &self.dragging_module {
                    let target_world_pos = (pointer_pos + drag.offset - pan) / zoom;
                    
                    let new_hp = target_world_pos.x / HP_PIXELS;
                    let new_row_f = target_world_pos.y / (RACK_HEIGHT + 20.0);
                    let new_row = new_row_f.round().max(0.0) as usize;

                    let old_hp = self.modules[drag.module_idx].hp_position;
                    let old_row = self.modules[drag.module_idx].row;

                    let delta_hp = new_hp - old_hp;
                    let delta_row = new_row as i32 - old_row as i32;

                    if delta_hp.abs() > 0.001 || delta_row != 0 {
                        let dragging_stable_id = self.modules[drag.module_idx].stable_id;
                        if self.selection.contains(&dragging_stable_id) {
                            // Move entire selection
                            for m_id in &self.selection {
                                if let Some(m) = self.modules.iter_mut().find(|m| m.stable_id == *m_id) {
                                    m.hp_position += delta_hp;
                                    let r = m.row as i32 + delta_row;
                                    m.row = r.max(0) as usize;
                                }
                            }
                        } else {
                            // Just move this one
                            self.modules[drag.module_idx].hp_position = new_hp;
                            self.modules[drag.module_idx].row = new_row;
                        }

                        // Resolve overlaps (Push logic)
                        self.resolve_overlaps(drag.module_idx);
                    }
                }
            }
            CableAction::RemoveModule { module_idx } => {
                self.remove_module(module_idx);
            }
            CableAction::ToggleBypass { module_idx } => {
                if let Some(m) = self.modules.get_mut(module_idx) {
                    m.bypassed = !m.bypassed;
                    self.event_queue.push(PatchEvent::ParamChanged {
                        stable_id: m.stable_id,
                        name: "bypassed".to_string(),
                        value_bits: if m.bypassed { 1 } else { 0 },
                        intent: IntentBoundary::Commit(IntentClass::Structural, None),
                    });
                }
            }
            CableAction::RandomizeParams { module_idx } => {
                if let Some(m) = self.modules.get_mut(module_idx) {
                    let seed = m.stable_id;
                    for (i, p) in m.descriptor.params.iter().enumerate() {
                        // Better scramble to avoid identical values for different params
                        let h = (seed.wrapping_add(i as u64)).wrapping_mul(0x517cc1b727220a95);
                        let h = h ^ (h >> 32);
                        let hash = (h as f64 / u64::MAX as f64) as f32;
                        
                        let val = p.min + hash.abs() * (p.max - p.min);
                        m.params.insert(p.name.to_string(), val);
                        self.event_queue.push(PatchEvent::ParamChanged {
                            stable_id: m.stable_id,
                            name: p.name.to_string(),
                            value_bits: val.to_bits(),
                            intent: IntentBoundary::Commit(IntentClass::Performance, None),
                        });
                    }
                }
            }
            CableAction::ResetModule { module_idx } => {
                if let Some(m) = self.modules.get_mut(module_idx) {
                    for p in m.descriptor.params.iter() {
                        let val = p.default;
                        m.params.insert(p.name.to_string(), val);
                        self.event_queue.push(PatchEvent::ParamChanged {
                            stable_id: m.stable_id,
                            name: p.name.to_string(),
                            value_bits: val.to_bits(),
                            intent: IntentBoundary::Commit(IntentClass::Structural, None),
                        });
                    }
                    m.dsp.reset();
                }
            }
            CableAction::AddModMapping { target_module_idx, param_name, src_stable_id, src_port_idx } => {
                if let Some(m) = self.modules.get_mut(target_module_idx) {
                    let bindings = m.param_modulations.entry(param_name).or_insert(Vec::new());
                    bindings.push(ModBinding {
                        source_stable_id: src_stable_id,
                        source_port_idx: src_port_idx,
                        amount: 1.0, // Default full depth
                    });
                    // Structural change requires rebuild
                    self.event_queue.push(PatchEvent::ParamChanged {
                        stable_id: m.stable_id,
                        name: "mod_mappings".to_string(),
                        value_bits: 0,
                        intent: IntentBoundary::Commit(IntentClass::Structural, None),
                    });
                }
            }
            CableAction::ClearModMappings { module_idx, param_name } => {
                if let Some(m) = self.modules.get_mut(module_idx) {
                    m.param_modulations.remove(&param_name);
                    self.event_queue.push(PatchEvent::ParamChanged {
                        stable_id: m.stable_id,
                        name: "mod_mappings".to_string(),
                        value_bits: 0,
                        intent: IntentBoundary::Commit(IntentClass::Structural, None),
                    });
                }
            }
            CableAction::InspectForensics { .. } => {}
            CableAction::SelectModule { stable_id, additive } => {
                if additive {
                    if let Some(pos) = self.selection.iter().position(|&id| id == stable_id) {
                        self.selection.remove(pos);
                    } else {
                        self.selection.push(stable_id);
                    }
                } else {
                    self.selection.clear();
                    self.selection.push(stable_id);
                }
            }
            CableAction::CopySelection => {
                if self.selection.is_empty() { return; }
                
                // Get selected modules
                let selected_modules: Vec<_> = self.modules.iter()
                    .enumerate()
                    .filter(|(_, m)| self.selection.contains(&m.stable_id))
                    .collect();
                
                let min_hp = selected_modules.iter().map(|(_, m)| m.hp_position).fold(f32::INFINITY, f32::min);
                
                let mut serial_modules = Vec::new();
                let mut old_to_new_idx = BTreeMap::new();
                
                for (i, (old_idx, m)) in selected_modules.iter().enumerate() {
                    serial_modules.push(SerializableModule {
                        id: m.descriptor.id.to_string(),
                        stable_id: m.stable_id,
                        params: m.params.clone(),
                        param_modulations: m.param_modulations.clone(),
                        hp_position: m.hp_position - min_hp, // Relative to selection
                        row: m.row,
                        bypassed: m.bypassed,
                        dsp_state: m.dsp.extract_state(),
                    });
                    old_to_new_idx.insert(*old_idx, i);
                }
                
                let mut serial_cables = Vec::new();
                for c in &self.cables {
                    if let (Some(&from_new), Some(&to_new)) = (old_to_new_idx.get(&c.from_module), old_to_new_idx.get(&c.to_module)) {
                        serial_cables.push(SerializableCable {
                            from_stable_id: selected_modules[from_new].1.stable_id,
                            from_port: c.from_port.clone(),
                            to_stable_id: selected_modules[to_new].1.stable_id,
                            to_port: c.to_port.clone(),
                            color: [c.color.r(), c.color.g(), c.color.b(), c.color.a()],
                            channels: c.channels,
                        });
                    }
                }
                
                self.clipboard = Some(SerializableRack {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    engine_hash: String::new(),
                    modules: serial_modules,
                    cables: serial_cables,
                    project_seed: self.project_seed,
                    aging: self.aging,
                    cable_opacity: self.cable_opacity,
                    cable_tension: self.cable_tension,
                    causality_log: Vec::new(), // Clipboards don't need full history
                    snapshots: BTreeMap::new(),
                });
            }
            CableAction::PasteSelection { pointer_pos } => {
                if let Some(serial) = self.clipboard.clone() {
                    let base_hp = (pointer_pos.x / HP_PIXELS).round();
                    
                    let mut new_modules = Vec::new();
                    let start_module_idx = self.modules.len();
                    
                    let mut old_stable_to_new_idx = BTreeMap::new();
                    
                    for (i, sm) in serial.modules.iter().enumerate() {
                        if let Some(desc) = registry.find(&sm.id) {
                            let mut inst = ModuleInstance::new(desc, self.sample_rate);
                            inst.params = sm.params.clone();
                            inst.param_modulations = sm.param_modulations.clone();
                            inst.hp_position = base_hp + sm.hp_position;
                            inst.row = sm.row;
                            inst.bypassed = sm.bypassed;
                            if let Some(state) = &sm.dsp_state {
                                inst.dsp.inject_state(state);
                            }
                            old_stable_to_new_idx.insert(sm.stable_id, start_module_idx + i);
                            new_modules.push(inst);
                        }
                    }
                    
                    for c in serial.cables {
                        if let (Some(&from_idx), Some(&to_idx)) = (old_stable_to_new_idx.get(&c.from_stable_id), old_stable_to_new_idx.get(&c.to_stable_id)) {
                            self.cables.push(Cable {
                                from_module: from_idx,
                                from_port: c.from_port,
                                to_module: to_idx,
                                to_port: c.to_port,
                                color: Color32::from_rgba_unmultiplied(c.color[0], c.color[1], c.color[2], c.color[3]),
                                channels: c.channels,
                            });
                        }
                    }
                    
                    self.modules.extend(new_modules);
                    // Rebuild will happen after event processing
                }
            }
            CableAction::CancelDrag => {
                self.dragging_cable = None;
                self.dragging_module = None;
            }
        }
    }

    pub fn resolve_overlaps(&mut self, dragging_idx: usize) {
        let row = self.modules[dragging_idx].row;
        
        // Sort modules in this row by position
        let mut row_indices: Vec<usize> = (0..self.modules.len())
            .filter(|&i| self.modules[i].row == row)
            .collect();
        
        row_indices.sort_by(|&a, &b| self.modules[a].hp_position.partial_cmp(&self.modules[b].hp_position).unwrap_or(std::cmp::Ordering::Equal));

        // Recursive push (Simplified iterative version)
        for _ in 0..self.modules.len() {
            let mut changed = false;
            for i in 0..row_indices.len() {
                for j in 0..row_indices.len() {
                    if i == j { continue; }
                    let idx_a = row_indices[i];
                    let idx_b = row_indices[j];
                    
                    let a_start = self.modules[idx_a].hp_position;
                    let a_end = a_start + self.modules[idx_a].descriptor.hp_width as f32;
                    let b_start = self.modules[idx_b].hp_position;
                    let b_end = b_start + self.modules[idx_b].descriptor.hp_width as f32;

                    if a_start < b_end && a_end > b_start {
                        // Overlap!
                        // Push to the right
                        if i < j {
                            self.modules[idx_b].hp_position = a_end;
                            changed = true;
                        } else {
                            // If i > j, it means idx_b is to the left of idx_a but they overlap
                            // We should push idx_a to the right of idx_b
                            self.modules[idx_a].hp_position = b_end;
                            changed = true;
                        }
                    }
                }
            }
            if !changed { break; }
        }

        // Ensure no module is at HP < 0
        // Find the min HP and shift everything if it's < 0
        let mut min_hp: f32 = 0.0;
        for &idx in &row_indices {
            min_hp = min_hp.min(self.modules[idx].hp_position);
        }
        if min_hp < 0.0 {
            for &idx in &row_indices {
                self.modules[idx].hp_position -= min_hp;
            }
        }
    }

    pub fn find_port_at(&self, pos: Pos2) -> Option<(usize, String, bool)> {
        for (m_idx, module) in self.modules.iter().enumerate() {
            let rect = module.world_rect();
            if rect.contains(pos) {
                for port in &module.descriptor.ports {
                    let p_pos = Pos2::new(
                        rect.left() + port.position[0] * rect.width(),
                        rect.top() + port.position[1] * rect.height(),
                    );
                    if p_pos.distance(pos) < 15.0 {
                        return Some((
                            m_idx,
                            port.name.to_string(),
                            port.direction == PortDirection::Output,
                        ));
                    }
                }
            }
        }
        None
    }

    pub fn port_world_pos(&self, module_idx: usize, port_name: &str) -> Option<Pos2> {
        let module = self.modules.get(module_idx)?;
        let port = module
            .descriptor
            .ports
            .iter()
            .find(|p| p.name == port_name)?;
        let rect = module.world_rect();
        Some(Pos2::new(
            rect.left() + port.position[0] * rect.width(),
            rect.top() + port.position[1] * rect.height(),
        ))
    }

    pub fn add_module(&mut self, descriptor: Arc<ModuleDescriptor>) {
        let mut inst = ModuleInstance::new(Arc::clone(&descriptor), self.sample_rate);
        let mut next_hp: f32 = 0.0;
        for m in &self.modules {
            let end = m.hp_position + m.descriptor.hp_width as f32;
            if end > next_hp {
                next_hp = end;
            }
        }
        inst.hp_position = next_hp;
        self.event_queue.push(PatchEvent::ModuleAdded {
            id: descriptor.id.to_string(),
            stable_id: inst.stable_id,
            ancestry: None,
            zone: descriptor.zone,
        });
        self.modules.push(inst);
    }

    pub fn remove_module(&mut self, idx: usize) {
        if idx >= self.modules.len() {
            return;
        }
        let stable_id = self.modules[idx].stable_id;
        self.event_queue
            .push(PatchEvent::ModuleRemoved { stable_id });
        self.cables.retain(|c| {
            if c.from_module == idx || c.to_module == idx {
                self.event_queue.push(PatchEvent::CableDisconnected {
                    to_id: self.modules[c.to_module].stable_id,
                    to_port: c.to_port.clone(),
                });
                false
            } else {
                true
            }
        });
        for cable in &mut self.cables {
            if cable.from_module > idx {
                cable.from_module -= 1;
            }
            if cable.to_module > idx {
                cable.to_module -= 1;
            }
        }
        self.modules.remove(idx);
    }

    pub fn build_snapshot(&self) -> (GraphSnapshot, Vec<Box<dyn RackDspNode>>, Vec<Vec<f32>>) {
        let n = self.modules.len();
        let mut order = Vec::with_capacity(n);
        let mut visited = vec![false; n];
        let mut visiting = vec![false; n];

        fn dfs(
            idx: usize,
            modules: &[ModuleInstance],
            cables: &[Cable],
            visited: &mut [bool],
            visiting: &mut [bool],
            order: &mut Vec<usize>,
        ) {
            if visited[idx] {
                return;
            }
            visiting[idx] = true;
            for cable in cables {
                if cable.to_module == idx {
                    let from = cable.from_module;
                    if !visited[from] && !visiting[from] {
                        dfs(from, modules, cables, visited, visiting, order);
                    }
                }
            }
            visiting[idx] = false;
            visited[idx] = true;
            order.push(idx);
        }

        let mut start_nodes: Vec<usize> = (0..n).collect();
        start_nodes.sort_by(|&a, &b| {
            self.modules[a]
                .hp_position
                .partial_cmp(&self.modules[b].hp_position)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for i in start_nodes {
            dfs(
                i,
                &self.modules,
                &self.cables,
                &mut visited,
                &mut visiting,
                &mut order,
            );
        }

        let mut node_ids = Vec::with_capacity(n);
        let mut node_type_ids = Vec::with_capacity(n);
        let mut new_nodes = Vec::with_capacity(n);
        let mut node_params = Vec::with_capacity(n);
        for &idx in &order {
            let m = &self.modules[idx];
            node_ids.push(m.stable_id);
            node_type_ids.push(m.descriptor.id.to_string());
            new_nodes.push((m.descriptor.factory)(self.sample_rate));
            
            let mut p_vals = Vec::new();
            for p_desc in &m.descriptor.params {
                p_vals.push(*m.params.get(p_desc.name).unwrap_or(&p_desc.default));
            }
            node_params.push(p_vals);
        }

        let port_counts = order
            .iter()
            .map(|&idx| {
                let m = &self.modules[idx];
                let ins = m
                    .descriptor
                    .ports
                    .iter()
                    .filter(|p| p.direction == PortDirection::Input)
                    .count();
                let outs = m
                    .descriptor
                    .ports
                    .iter()
                    .filter(|p| p.direction == PortDirection::Output)
                    .count();
                (ins, outs)
            })
            .collect();

        let mut index_map = vec![0; n];
        for (new_idx, &old_idx) in order.iter().enumerate() {
            index_map[old_idx] = new_idx;
        }

        let mut connections = Vec::new();
        for cable in &self.cables {
            let from_new = index_map[cable.from_module];
            let to_new = index_map[cable.to_module];

            let from_port_idx = self.modules[cable.from_module]
                .descriptor
                .ports
                .iter()
                .filter(|p| p.direction == PortDirection::Output)
                .position(|p| p.name == cable.from_port)
                .unwrap_or(0);
            let to_port_idx = self.modules[cable.to_module]
                .descriptor
                .ports
                .iter()
                .filter(|p| p.direction == PortDirection::Input)
                .position(|p| p.name == cable.to_port)
                .unwrap_or(0);

            connections.push(Connection {
                from_module: from_new,
                from_port: from_port_idx,
                to_module: to_new,
                to_port: to_port_idx,
            });
        }

        (
            GraphSnapshot {
                modulations: vec![Vec::new(); order.len()],
                order: (0..n).collect(),
                connections,
                port_counts,
                node_ids,
                node_type_ids,
                forward_edges: Vec::new(),
                back_edges: Vec::new(),
            },
            new_nodes,
            node_params,
        )
    }
}

pub fn draw_rack_rails(painter: &Painter, viewport: Rect, zoom: f32, pan: Vec2) {
    let rail_color = Color32::from_rgb(60, 55, 50);
    let screw_color = Color32::from_rgb(120, 115, 100);
    let rail_h = 12.0 * zoom;
    for row in 0..4 {
        let base_y = row as f32 * (RACK_HEIGHT + 20.0) * zoom + pan.y;
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(viewport.left(), base_y),
                vec2(viewport.width(), rail_h),
            ),
            0.0,
            rail_color,
        );
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(viewport.left(), base_y + RACK_HEIGHT * zoom - rail_h),
                vec2(viewport.width(), rail_h),
            ),
            0.0,
            rail_color,
        );
        let screw_spacing = 10.0 * HP_PIXELS * zoom;
        let mut x = pan.x % screw_spacing;
        while x < viewport.width() {
            for rail_y in [
                base_y + rail_h * 0.5,
                base_y + RACK_HEIGHT * zoom - rail_h * 0.5,
            ] {
                painter.circle_filled(Pos2::new(x, rail_y), 3.0 * zoom, screw_color);
                painter.circle_stroke(
                    Pos2::new(x, rail_y),
                    3.0 * zoom,
                    Stroke::new(0.5, Color32::from_gray(80)),
                );
            }
            x += screw_spacing;
        }
    }
}
