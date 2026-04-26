pub mod nodes;
pub mod osc;
pub mod offline;
pub mod freeze;
pub mod jit;
#[cfg(test)]
mod tests {
    mod null_test;
}

use dirtydata_core::ir::{Graph, EdgeKind};
use dirtydata_core::types::{StableId, NodeKind, PortDirection};
use dirtydata_core::graph_utils::topological_sort;
use crate::nodes::*;
use crate::nodes::legacy::{
    MidiEvent, EnvelopeNode, AutomationNode, SequencerNode, WavefolderNode,
    AddNode, MultiplyNode, ClipNode, TriggerNode, DelayNode,
    LorenzNode, MackeyGlassNode, GrayScottNode, SlewLimiterNode, SampleHoldNode,
    ClockNode, ProbabilityGateNode, ReverbNode, SpringReverbNode, GranularNode,
    LogicNode, SpectralFreezeNode, FFTConvolveNode, ZdfLadderNode, SvfNode,
    DiodeClipperNode, BbdDelayNode, KarplusStrongNode, ModalResonatorNode,
    ChuaCircuitNode, TapeMachineNode, MatrixMixerNode, EuclideanSequencerNode,
};
use crate::osc::OscHandler;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Sender, Receiver};

pub struct ParameterUpdate {
    pub node_id: StableId,
    pub param: String,
    pub value: f32,
    pub provenance: Vec<String>,
}

pub use offline::OfflineRenderer;

pub enum EngineCommand {
    UpdateParameter(ParameterUpdate),
    ReplaceGraph(Graph, Option<jit::JitProgram>),
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct SignalMetrics {
    pub rms: f32,
    pub peak: f32,
    pub dc_offset: f32,
    pub dominant_freq: f32,
    pub activity_score: f32,
    pub saturation: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticRecord {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

pub struct SharedState {
    node_metrics: Arc<dashmap::DashMap<StableId, SignalMetrics>>,
    scope_buffer: Arc<crossbeam_queue::ArrayQueue<f32>>,
    probe_buffers: Arc<dashmap::DashMap<StableId, Arc<crossbeam_queue::ArrayQueue<f32>>>>,
    convergence_info: Arc<dashmap::DashMap<StableId, usize>>,
    circuit_instability: Arc<dashmap::DashMap<StableId, f32>>,
    parameter_provenance: Arc<dashmap::DashMap<StableId, HashMap<String, Vec<String>>>>,
    node_diagnostics: Arc<dashmap::DashMap<StableId, DiagnosticRecord>>,
    engine_logs: Arc<crossbeam_queue::ArrayQueue<String>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            node_metrics: Arc::new(dashmap::DashMap::new()),
            scope_buffer: Arc::new(crossbeam_queue::ArrayQueue::new(4096)),
            probe_buffers: Arc::new(dashmap::DashMap::new()),
            convergence_info: Arc::new(dashmap::DashMap::new()),
            circuit_instability: Arc::new(dashmap::DashMap::new()),
            parameter_provenance: Arc::new(dashmap::DashMap::new()),
            node_diagnostics: Arc::new(dashmap::DashMap::new()),
            engine_logs: Arc::new(crossbeam_queue::ArrayQueue::new(100)),
        }
    }

    pub fn log(&self, msg: impl Into<String>) {
        let _ = self.engine_logs.push(msg.into());
    }

    pub fn get_node_metrics(&self, node_id: &StableId) -> Option<SignalMetrics> {
        self.node_metrics.get(node_id).map(|m| *m)
    }

    pub fn get_diagnostic(&self, node_id: &StableId) -> Option<DiagnosticRecord> {
        self.node_diagnostics.get(node_id).map(|d| d.clone())
    }

    pub fn scope_buffer(&self) -> Arc<crossbeam_queue::ArrayQueue<f32>> {
        self.scope_buffer.clone()
    }

    pub fn get_circuit_instability(&self, node_id: &StableId) -> Option<f32> {
        self.circuit_instability.get(node_id).map(|i| *i)
    }
}

pub struct ModulationMapping {
    pub source_node_id: StableId,
    pub source_port_idx: usize,
    pub target_node_idx: usize,
    pub target_param: String,
    pub amount: f32,
}

pub struct DspRunner {
    nodes: Vec<(StableId, Box<dyn base::DspNode>)>,
    node_outputs: HashMap<StableId, Vec<[f32; 2]>>,
    graph: Graph,
    feedback_latches: Vec<[f32; 2]>,
    feedback_reads: Vec<Vec<(usize, usize)>>,
    feedback_writes: Vec<Vec<(usize, usize)>>,
    modulation_mappings: Vec<ModulationMapping>,
    node_saturation: HashMap<StableId, f32>,
    jit_program: Option<jit::JitProgram>,
    parameter_provenance: HashMap<StableId, HashMap<String, Vec<String>>>,
}

impl DspRunner {
    #[tracing::instrument(skip(graph, midi_rx))]
    pub fn new(graph: Graph, midi_rx: Option<Receiver<MidiEvent>>, sample_rate: f32) -> Self {
        tracing::debug!("Creating DspRunner with {} nodes", graph.nodes.len());
        let (sorted_ids, _) = topological_sort(&graph);
        let mut nodes: Vec<(StableId, Box<dyn base::DspNode>)> = Vec::new();
        let mut node_outputs = HashMap::new();

        for &id in &sorted_ids {
            if let Some(node) = graph.nodes.get(&id) {
                let dsp_node: Box<dyn base::DspNode> = match &node.kind {
                    NodeKind::Foreign(plugin_name) => {
                        Box::new(legacy::ForeignNode::new(plugin_name.clone(), 256))
                    }
                    _ => {
                        let name = node.config.get("name").and_then(|v| v.as_string());
                        match name.map(|s| s.as_str()).unwrap_or("Unknown") {
                            "Oscillator" | "Sine" => Box::new(OscillatorNode::new()),
                            "Noise" => Box::new(NoiseNode::new(format!("{}", id).as_bytes().len() as u64)),
                            "Gain" => Box::new(GainNode::new()),
                            "Add" => Box::new(AddNode::new()),
                            "Multiply" => Box::new(MultiplyNode::new()),
                            "Clip" => Box::new(ClipNode::new()),
                            "Filter" | "Biquad" => Box::new(BiquadFilterNode::new()),
                            "Compressor" | "Dynamics" => Box::new(CompressorNode::new()),
                            "Delay" => Box::new(DelayNode::new(sample_rate as usize)),
                            "Sampler" | "AssetReader" => {
                                let path_val = node.config.get("path").and_then(|v| v.as_string());
                                let data = if let Some(path) = path_val {
                                    match hound::WavReader::open(path) {
                                        Ok(mut reader) => {
                                            let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect();
                                            Arc::new(samples)
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to load asset {}: {}", path, e);
                                            Arc::new(vec![])
                                        }
                                    }
                                } else {
                                    Arc::new(vec![])
                                };
                                Box::new(AssetReaderNode::new(data))
                            }
                            "Trigger" => Box::new(TriggerNode::new()),
                            "Envelope" | "ADSR" => Box::new(EnvelopeNode::new()),
                            "Automation" => Box::new(AutomationNode::new()),
                            "MidiIn" => {
                                if let Some(rx) = &midi_rx {
                                    Box::new(MidiInNode::new(rx.clone()))
                                } else {
                                    Box::new(GainNode::new())
                                }
                            }
                            "Sequencer" => Box::new(SequencerNode::new()),
                            "Wavefolder" => Box::new(WavefolderNode::new()),
                            "Lorenz" => Box::new(LorenzNode::new()),
                            "MackeyGlass" => Box::new(MackeyGlassNode::new(10.0, sample_rate)),
                            "GrayScott" | "ReactionDiffusion" => Box::new(GrayScottNode::new(256)),
                            
                            // Destruction
                            "BitCrush" => Box::new(BitCrushNode::new()),
                            "WaveShaper" => Box::new(WaveShaperNode::new()),
                            "Pll" => Box::new(PllNode::new()),

                            // Legacy & Specialized Nodes
                            "SlewLimiter" | "Slew" => Box::new(SlewLimiterNode::new()),
                            "SampleHold" | "S&H" => Box::new(SampleHoldNode::new()),
                            "Clock" => Box::new(ClockNode::new()),
                            "ProbabilityGate" => Box::new(ProbabilityGateNode::new()),
                            "Reverb" => Box::new(ReverbNode::new(sample_rate)),
                            "SpringReverb" => Box::new(SpringReverbNode::new(sample_rate)),
                            "Granular" => Box::new(GranularNode::new(sample_rate)),
                            "Logic" => Box::new(LogicNode::new()),
                            "SpectralFreeze" => Box::new(SpectralFreezeNode::new(1024)),
                            "FFTConvolve" => Box::new(FFTConvolveNode::new(1024)),
                            "ZdfLadder" | "TB303Ladder" | "Ladder" => Box::new(ZdfLadderNode::new(sample_rate)),
                            "Svf" | "SVFFilter" => Box::new(SvfNode::new(sample_rate)),
                            "DiodeClipper" => Box::new(DiodeClipperNode::new()),
                            "BbdDelay" => Box::new(BbdDelayNode::new(sample_rate)),
                            "KarplusStrong" => Box::new(KarplusStrongNode::new(sample_rate)),
                            "ModalResonator" => Box::new(ModalResonatorNode::new(sample_rate)),
                            "ChuaCircuit" | "Chua" => Box::new(ChuaCircuitNode::new(sample_rate)),
                            "TapeMachine" | "Tape" => Box::new(TapeMachineNode::new(sample_rate)),
                            "MatrixMixer" => Box::new(MatrixMixerNode::new(8, 8)),
                            "Euclidean" | "EuclideanSequencer" => Box::new(EuclideanSequencerNode::new()),
                            _ => Box::new(GainNode::new()),
                        }
                    }
                };
                nodes.push((id, dsp_node));
                let port_count = node.ports.iter().filter(|p| p.direction == PortDirection::Output).count().max(1);
                node_outputs.insert(id, vec![[0.0, 0.0]; port_count]);
            }
        }

        let mut feedback_latches = Vec::new();
        let mut feedback_reads = vec![Vec::new(); nodes.len()];
        let mut feedback_writes = vec![Vec::new(); nodes.len()];

        for edge in graph.edges.values() {
            if edge.kind == EdgeKind::Feedback {
                let latch_idx = feedback_latches.len();
                feedback_latches.push([0.0, 0.0]);
                if let Some(src_idx) = nodes.iter().position(|(id, _)| *id == edge.source.node_id) {
                    feedback_writes[src_idx].push((0, latch_idx));
                }
                if let Some(tgt_idx) = nodes.iter().position(|(id, _)| *id == edge.target.node_id) {
                    feedback_reads[tgt_idx].push((0, latch_idx));
                }
            }
        }

        let mut modulation_mappings = Vec::new();
        for m in graph.modulations.values() {
            if let Some(target_idx) = nodes.iter().position(|(id, _)| *id == m.target_node) {
                modulation_mappings.push(ModulationMapping {
                    source_node_id: m.source.node_id,
                    source_port_idx: 0,
                    target_node_idx: target_idx,
                    target_param: m.target_param.clone(),
                    amount: m.amount,
                });
            }
        }

        let mut node_saturation = HashMap::new();
        for id in graph.nodes.keys() {
            node_saturation.insert(*id, 0.0);
        }

        Self { nodes, node_outputs, graph, feedback_latches, feedback_reads, feedback_writes, modulation_mappings, node_saturation, jit_program: None, parameter_provenance: HashMap::new() }
    }

    pub fn process_sample(&mut self, ctx: &ProcessContext) -> [f32; 2] {
        if let Some(jit) = &mut self.jit_program {
            return jit.execute(ctx);
        }

        for m in &self.modulation_mappings {
            if let Some(outputs) = self.node_outputs.get(&m.source_node_id) {
                let val = (outputs[m.source_port_idx][0] + outputs[m.source_port_idx][1]) * 0.5;
                let (_, node) = &mut self.nodes[m.target_node_idx];
                node.update_parameter(&m.target_param, val * m.amount);
            }
        }

        for (i, (id, node)) in self.nodes.iter_mut().enumerate() {
            let mut inputs = Vec::new();
            for edge in self.graph.edges.values() {
                if edge.kind == EdgeKind::Normal && edge.target.node_id == *id {
                    if let Some(prev_outputs) = self.node_outputs.get(&edge.source.node_id) {
                        let val = prev_outputs[0];
                        inputs.push(val[0]);
                        inputs.push(val[1]);
                    }
                }
            }

            for (_, latch_idx) in &self.feedback_reads[i] {
                let latch = self.feedback_latches[*latch_idx];
                if inputs.is_empty() {
                    inputs.push(latch[0]);
                    inputs.push(latch[1]);
                }
            }

            let outputs = self.node_outputs.get_mut(id).unwrap();

            let ctx = ProcessContext {
                sample_rate: ctx.sample_rate,
                global_sample_index: ctx.global_sample_index,
                crash_flag: ctx.crash_flag,
                osc_tx: ctx.osc_tx,
                convergence_info: ctx.convergence_info,
                node_diagnostics: ctx.node_diagnostics,
                node_id: Some(*id),
            };
            node.process(&inputs, &mut outputs[..], &self.graph.nodes.get(id).unwrap().config, &ctx);

            // --- Pre-emptive Safety Saturation ---
            let mut sat_accum = 0.0;
            for port_out in outputs.iter_mut() {
                for sample in port_out.iter_mut() {
                    let s: &mut f32 = sample;
                    let x = *s;
                    if x.abs() > 1.2 {
                        *s = 1.2 * x.signum();
                    } else {
                        *s = x - (x.powi(3) * 0.23); 
                    }
                    sat_accum += (x - *s).abs();
                }
            }
            if let Some(entry) = self.node_saturation.get_mut(id) {
                *entry += sat_accum;
            }

            for (_, latch_idx) in &self.feedback_writes[i] {
                let val: [f32; 2] = outputs[0];
                self.feedback_latches[*latch_idx] = val;
            }
        }

        let mut final_out = [0.0, 0.0];
        for (id, _) in &self.nodes {
            if let Some(node) = self.graph.nodes.get(id) {
                if node.kind == NodeKind::Sink {
                    let out = self.node_outputs.get(id).unwrap()[0];
                    final_out[0] += out[0];
                    final_out[1] += out[1];
                }
            }
        }
        final_out
    }

    pub fn get_graph(&self) -> &Graph {
        &self.graph
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<(StableId, Box<dyn DspNode>)> {
        &mut self.nodes
    }

    pub fn update_parameter(&mut self, node_id: StableId, param: &str, value: f32, provenance: Vec<String>) {
        self.parameter_provenance.entry(node_id).or_default().insert(param.to_string(), provenance);
        if let Some((_, node)) = self.nodes.iter_mut().find(|(id, _)| *id == node_id) {
            node.update_parameter(param, value);
        }
    }

    pub fn extract_all_states(&self) -> HashMap<StableId, NodeState> {
        self.nodes.iter().map(|(id, node)| (*id, node.extract_state())).collect()
    }

    pub fn inject_all_states(&mut self, states: &HashMap<StableId, NodeState>) {
        for (id, node) in &mut self.nodes {
            if let Some(state) = states.get(id) {
                node.inject_state(state);
            }
        }
    }

    pub fn get_node_outputs(&self, id: &StableId) -> Option<&Vec<[f32; 2]>> {
        self.node_outputs.get(id)
    }
}

impl std::fmt::Debug for DspRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DspRunner")
            .field("nodes", &self.nodes.len())
            .field("feedback_latches", &self.feedback_latches.len())
            .field("modulation_mappings", &self.modulation_mappings.len())
            .finish()
    }
}

pub struct AudioEngine {
    _stream: cpal::Stream,
    command_tx: Sender<EngineCommand>,
    shared_state: Arc<SharedState>,
}

impl AudioEngine {
    #[tracing::instrument(skip(shared_state, midi_rx))]
    pub fn new(shared_state: Arc<SharedState>, midi_rx: Receiver<MidiEvent>) -> Self {
        tracing::info!("Initializing AudioEngine");
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no output device available");
        let config = device.default_output_config().unwrap();
        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;

        let (command_tx, command_rx) = crossbeam_channel::unbounded::<EngineCommand>();
        let shared_state_for_audio = shared_state.clone();
        let crash_flag_for_audio = Arc::new(AtomicBool::new(false));

        // Refactored OSC Handling
        let osc_handler = OscHandler::new(command_tx.clone());
        osc_handler.spawn_input_thread("127.0.0.1:8000");

        let (osc_tx, osc_rx) = crossbeam_channel::bounded::<OscMessage>(1024);
        OscHandler::spawn_output_thread(osc_rx, "127.0.0.1:9001".to_string());

        let mut current_runner: Option<DspRunner> = None;
        let mut global_sample_index: u64 = 0;
        let midi_rx_internal = midi_rx.clone();
        let crash_flag_callback = crash_flag_for_audio.clone();
        
        // Metrics Accumulators: node_id -> (sum, sum_sq, peak, sat_sum, count)
        let mut metrics_acc: HashMap<StableId, (f32, f32, f32, f32, usize)> = HashMap::new();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    while let Ok(cmd) = command_rx.try_recv() {
                        match cmd {
                            EngineCommand::UpdateParameter(update) => {
                                shared_state_for_audio.parameter_provenance.entry(update.node_id).or_default().insert(update.param.clone(), update.provenance.clone());
                                if let Some(runner) = &mut current_runner {
                                    runner.update_parameter(update.node_id, &update.param, update.value, update.provenance);
                                }
                            }
                            EngineCommand::ReplaceGraph(graph, jit_prog) => {
                                shared_state_for_audio.log(format!("Replacing Graph: {} nodes", graph.nodes.len()));
                                let mut new_runner = DspRunner::new(graph, Some(midi_rx_internal.clone()), sample_rate);
                                if let Some(old_runner) = &current_runner {
                                    let states = old_runner.extract_all_states();
                                    new_runner.inject_all_states(&states);
                                }
                                new_runner.jit_program = jit_prog;
                                current_runner = Some(new_runner);
                                shared_state_for_audio.log("Graph Replacement (JIT enabled) OK.");
                            }
                        }
                    }

                    let Some(runner) = &mut current_runner else { data.fill(0.0); return; };

                    for frame in data.chunks_mut(channels) {
                        let ctx = ProcessContext {
                            sample_rate,
                            global_sample_index,
                            crash_flag: Some(&crash_flag_callback),
                            osc_tx: Some(&osc_tx),
                            convergence_info: Some(&shared_state_for_audio.convergence_info),
                            node_diagnostics: Some(&shared_state_for_audio.node_diagnostics),
                            node_id: None,
                        };
                        let out = runner.process_sample(&ctx);
                        for (node_id, ports) in &runner.node_outputs {
                            let val = (ports[0][0] + ports[0][1]) * 0.5;
                            let acc = metrics_acc.entry(*node_id).or_insert((0.0, 0.0, 0.0, 0.0, 0));
                            acc.0 += val; // sum for DC
                            acc.1 += val * val; // sum_sq for RMS
                            acc.2 = acc.2.max(val.abs()); // peak
                            
                            let sat = runner.node_saturation.get_mut(node_id).map(|s| {
                                let v = *s;
                                *s = 0.0;
                                v
                            }).unwrap_or(0.0);
                            acc.3 += sat; // sum for Saturation
                            acc.4 += 1;

                            if acc.4 >= 128 {
                                let (sum, sum_sq, peak, sat_sum, count) = *acc;
                                let f_count = count as f32;
                                let dc = sum / f_count;
                                let rms = (sum_sq / f_count).sqrt();
                                
                                shared_state_for_audio.node_metrics.insert(*node_id, SignalMetrics {
                                    rms,
                                    peak,
                                    dc_offset: dc,
                                    dominant_freq: 0.0, // TODO: FFT or Zero-crossing
                                    activity_score: (rms * 10.0).min(1.0),
                                    saturation: sat_sum / f_count,
                                });
                                *acc = (0.0, 0.0, 0.0, 0.0, 0);
                            }

                            let probes = &shared_state_for_audio.probe_buffers;
                            if let Some(buf_ref) = probes.get(node_id) {
                                let _ = buf_ref.value().push(val);
                            }
                            if val.is_nan() { crash_flag_callback.store(true, Ordering::SeqCst); }
                        }
                        frame[0] = out[0];
                        if channels > 1 { frame[1] = out[1]; }
                        global_sample_index += 1;
                    }
                },
                |err| {
                    tracing::error!("an error occurred on stream: {}", err);
                },
                None
            ).unwrap(),
            _ => panic!("unsupported sample format"),
        };

        stream.play().unwrap();
        Self { _stream: stream, command_tx, shared_state }
    }

    pub fn shared_state(&self) -> Arc<SharedState> {
        self.shared_state.clone()
    }

    pub fn update_parameter(&self, node_id: StableId, param: String, value: f32) {
        let _ = self.command_tx.send(EngineCommand::UpdateParameter(ParameterUpdate {
            node_id,
            param,
            value,
            provenance: vec!["host".to_string()],
        }));
    }

    pub fn replace_graph(&self, graph: Graph, jit_prog: Option<jit::JitProgram>) {
        let _ = self.command_tx.send(EngineCommand::ReplaceGraph(graph, jit_prog));
    }
}
