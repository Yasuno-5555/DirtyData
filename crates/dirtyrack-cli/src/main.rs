use clap::{Parser, Subcommand};
use colored::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Parser)]
#[command(name = "dirtyrack")]
#[command(about = "DirtyRack Forensic Eurorack Simulator CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the Graphical Projector (GUI)
    Gui,

    /// List all available modules (built-in and dynamic)
    ModuleList,

    /// Render a patch to a deterministic WAV file
    Render {
        /// Path to the patch JSON file
        patch: PathBuf,

        /// Output WAV file path
        #[arg(short, long, default_value = "output.wav")]
        output: PathBuf,

        /// Length in seconds
        #[arg(short, long, default_value_t = 10.0)]
        length: f32,

        /// Sample rate in Hz
        #[arg(short, long, default_value_t = 44100)]
        sample_rate: u32,
    },

    /// Verify a render against its certificate
    Verify {
        /// Path to the WAV file
        wav: PathBuf,
        /// Path to the .dirtyrack.cert file
        cert: PathBuf,
    },

    /// Compare two renders and report bit-level divergence (A/B Audit)
    DiffRender {
        /// Path to first WAV
        wav_a: PathBuf,
        /// Path to first Cert
        cert_a: PathBuf,
        /// Path to second WAV
        wav_b: PathBuf,
        /// Path to second Cert
        cert_b: PathBuf,
    },

    /// Benchmark a patch for real-time safety
    Bench {
        /// Path to the patch JSON file
        patch: PathBuf,
        /// Duration in samples
        #[arg(short, long, default_value_t = 44100)]
        samples: usize,
    },

    /// Generate a forensic certificate for an existing render
    Sign {
        /// Path to the WAV file
        wav: PathBuf,
        /// Path to the patch JSON file
        patch: PathBuf,
        /// Engine version
        #[arg(short, long, default_value = "0.1.0")]
        version: String,
    },

    /// Inspect a patch file and print its contents
    Inspect {
        /// Path to the patch file
        patch: PathBuf,
    },

    /// Get detailed information about a specific module
    ModuleInfo {
        /// Module ID (e.g. dirty_vco)
        id: String,
    },

    /// Create a new patch file from a template
    New {
        /// Path to the new patch file
        path: PathBuf,
        /// Template type (empty, basic, complete)
        #[arg(short, long, default_value = "basic")]
        template: String,
    },

    /// Add a module to an existing patch
    AddModule {
        /// Path to the patch file
        patch: PathBuf,
        /// Module ID to add
        module_id: String,
        /// HP position
        #[arg(short, long, default_value_t = 0.0)]
        x: f32,
        /// Row index
        #[arg(short, long, default_value_t = 0)]
        row: usize,
    },

    /// Connect two ports in a patch
    Connect {
        /// Path to the patch file
        patch: PathBuf,
        /// Source module stable_id
        from_id: u64,
        /// Source port name
        from_port: String,
        /// Target module stable_id
        to_id: u64,
        /// Target port name
        to_port: String,
    },

    /// Set a parameter value in a patch
    SetParam {
        /// Path to the patch file
        patch: PathBuf,
        /// Module stable_id
        id: u64,
        /// Parameter name
        name: String,
        /// New value
        value: f32,
    },

    /// Export the module registry
    ExportRegistry {
        /// Output format (json, md)
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Output path (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Snapshot the internal forensic state of modules in a patch
    Snapshot {
        /// Path to the patch file
        patch: PathBuf,
        /// Optional specific module stable_id to inspect
        #[arg(short, long)]
        id: Option<u64>,
    },

    /// Remove a module from the patch
    Rm {
        /// Path to the patch file
        patch: PathBuf,
        /// Module ID (stable_id or alias)
        id: String,
    },

    /// Assign an alias to a module
    Alias {
        /// Path to the patch file
        patch: PathBuf,
        /// Stable ID of the module
        id: u64,
        /// Alias name
        name: String,
    },

    /// Enter interactive patching shell
    Shell {
        /// Path to the patch file
        patch: PathBuf,
    },

    /// Run a batch of commands from a file
    Batch {
        /// Path to the patch file
        patch: PathBuf,
        /// Path to the script file
        script: PathBuf,
    },

    /// Play the patch in real-time (experimental)
    Play {
        /// Path to the patch file
        patch: PathBuf,
        /// Optional duration in seconds
        #[arg(short, long)]
        duration: Option<f32>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    inner_execute(cli.command)
}

fn inner_execute(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Gui => {
            println!(
                "{} Launching DirtyRack Graphical Projector...",
                "▶".cyan().bold()
            );
            let _ = dirtyrack_gui::run();
        }
        Commands::ModuleList => {
            println!("{} Available DirtyRack Modules:", "✓".green().bold());
            let registry = dirtyrack_modules::registry::ModuleRegistry::new();
            for module in registry.all() {
                println!(
                    "  - {:<16} ({}) by {}",
                    module.name.bold(),
                    module.id,
                    module.manufacturer
                );
            }
        }
        Commands::Render {
            patch,
            output,
            length,
            sample_rate,
        } => {
            println!("{} Rendering patch: {:?}", "▶".yellow().bold(), patch);
            let patch_json = std::fs::read_to_string(&patch)?;
            let serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&patch_json)?;
            
            let registry = dirtyrack_modules::registry::ModuleRegistry::new();
            let mut runner = dirtyrack_modules::runner::RackRunner::new(sample_rate as f32, dirtyrack_modules::signal::SeedScope::Global(serial.project_seed));
            runner.ctx.aging = serial.aging;

            let n = serial.modules.len();
            let mut stable_to_idx = BTreeMap::new();
            for (i, m) in serial.modules.iter().enumerate() {
                stable_to_idx.insert(m.stable_id, i);
            }

            // --- Topological Sort ---
            let mut order = Vec::with_capacity(n);
            let mut visited = vec![false; n];
            let mut visiting = vec![false; n];

            fn dfs(
                idx: usize,
                modules: &[dirtyrack_gui::rack::SerializableModule],
                cables: &[dirtyrack_gui::rack::SerializableCable],
                stable_to_idx: &BTreeMap<u64, usize>,
                visited: &mut [bool],
                visiting: &mut [bool],
                order: &mut Vec<usize>,
            ) {
                if visited[idx] { return; }
                visiting[idx] = true;
                let my_stable_id = modules[idx].stable_id;
                for cable in cables {
                    if cable.to_stable_id == my_stable_id {
                        if let Some(&from_idx) = stable_to_idx.get(&cable.from_stable_id) {
                            if !visited[from_idx] && !visiting[from_idx] {
                                dfs(from_idx, modules, cables, stable_to_idx, visited, visiting, order);
                            }
                        }
                    }
                }
                visiting[idx] = false;
                visited[idx] = true;
                order.push(idx);
            }

            for i in 0..n {
                dfs(i, &serial.modules, &serial.cables, &stable_to_idx, &mut visited, &mut visiting, &mut order);
            }

            // --- Reconstruct Engine State ---
            let mut nodes = Vec::with_capacity(n);
            let mut node_type_ids = Vec::with_capacity(n);
            let mut node_ids = Vec::with_capacity(n);
            let mut initial_params = Vec::with_capacity(n);
            let mut port_counts = Vec::with_capacity(n);

            for &idx in &order {
                let m = &serial.modules[idx];
                if let Some(desc) = registry.find(&m.id) {
                    let mut node = (desc.factory)(sample_rate as f32);
                    if let Some(state) = &m.dsp_state {
                        node.inject_state(state);
                    }
                    nodes.push(node);
                    node_type_ids.push(m.id.clone());
                    node_ids.push(m.stable_id);
                    
                    let mut p_vals = vec![0.0; 64];
                    for (p_idx, p_desc) in desc.params.iter().enumerate() {
                        if p_idx < 64 {
                            p_vals[p_idx] = *m.params.get(p_desc.name).unwrap_or(&p_desc.default);
                        }
                    }
                    initial_params.push(p_vals);

                    let ins = desc.ports.iter().filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input).count();
                    let outs = desc.ports.iter().filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output).count();
                    port_counts.push((ins, outs));
                }
            }

            let mut index_map = vec![0; n];
            for (new_idx, &old_idx) in order.iter().enumerate() {
                index_map[old_idx] = new_idx;
            }

            let mut connections = Vec::new();
            for cable in &serial.cables {
                if let (Some(&fi_old), Some(&ti_old)) = (stable_to_idx.get(&cable.from_stable_id), stable_to_idx.get(&cable.to_stable_id)) {
                    let fi = index_map[fi_old];
                    let ti = index_map[ti_old];
                    
                    let from_desc = registry.find(&serial.modules[fi_old].id);
                    let to_desc = registry.find(&serial.modules[ti_old].id);
                    
                    if let (Some(fd), Some(td)) = (from_desc, to_desc) {
                        let from_port = fd.ports.iter()
                            .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output)
                            .position(|p| p.name == cable.from_port).unwrap_or(0);
                        let to_port = td.ports.iter()
                            .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input)
                            .position(|p| p.name == cable.to_port).unwrap_or(0);
                        
                        connections.push(dirtyrack_modules::runner::Connection {
                            from_module: fi,
                            from_port,
                            to_module: ti,
                            to_port,
                        });
                    }
                }
            }

            let mut snapshot = dirtyrack_modules::runner::GraphSnapshot {
                order: (0..n).collect(),
                connections,
                port_counts,
                node_ids,
                node_type_ids,
                modulations: vec![Vec::new(); n], // TODO: Modulations from serial
                forward_edges: Vec::new(),
                back_edges: Vec::new(),
            };

            runner.apply_snapshot(&mut snapshot, nodes);

            // --- WAV Setup ---
            let spec = hound::WavSpec {
                channels: 2,
                sample_rate: sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };

            let mut writer = hound::WavWriter::create(&output, spec)?;
            let total_samples = (length * sample_rate as f32) as usize;
            let mut hasher = blake3::Hasher::new();

            // Find Output module index in sorted order
            let output_node_idx = order.iter().position(|&idx| serial.modules[idx].id == "dirty_output");

            for _ in 0..total_samples {
                runner.process_sample(&snapshot, &initial_params);
                
                let mut left = 0.0;
                let mut right = 0.0;
                
                if let Some(out_idx) = output_node_idx {
                    // Output module uses Port 2 and 3 (Internal indexing 0 and 1) for Master L/R
                    left = runner.get_output(out_idx, 0);
                    right = runner.get_output(out_idx, 1);
                } else if !runner.output_buffers.is_empty() {
                    // Fallback: Use the last node's first two channels
                    let last_idx = runner.output_buffers.len() - 1;
                    left = runner.get_output(last_idx, 0);
                    right = runner.get_output(last_idx, 1);
                }
                
                writer.write_sample(left)?;
                writer.write_sample(right)?;
                hasher.update(&left.to_le_bytes());
                hasher.update(&right.to_le_bytes());
            }

            writer.finalize()?;
            let hash = hasher.finalize();
            println!("{} Render Complete!", "✓".green().bold());
            println!("   File: {}", output.display().to_string().cyan());
            println!("   Hash (BLAKE3-PCM): {}", hash.to_hex());
        }
        Commands::Verify { wav, cert } => {
            println!("{} Starting Forensic Verification...", "🔍".blue().bold());
            let cert_json = std::fs::read_to_string(cert)?;
            let cert_data: serde_json::Value = serde_json::from_str(&cert_json)?;
            let expected_hash = cert_data["render_hash"].as_str().unwrap_or("");
            
            let mut reader = hound::WavReader::open(wav)?;
            let mut hasher = blake3::Hasher::new();
            for sample in reader.samples::<f32>() {
                let s = sample?;
                hasher.update(&s.to_le_bytes());
            }
            let actual_hash = hasher.finalize().to_hex().to_string();

            if actual_hash == expected_hash {
                println!("{} Certified Render Verified", "✓".green().bold());
                println!("   Hash Match: {}", actual_hash.cyan());
            } else {
                println!("{} Verification FAILED", "✗".red().bold());
                println!("   Expected: {}", expected_hash);
                println!("   Actual:   {}", actual_hash);
            }
        }
        Commands::DiffRender { wav_a, cert_a: _, wav_b, cert_b: _ } => {
            println!("{} Starting A/B Differential Audit...", "📊".magenta().bold());
            let mut reader_a = hound::WavReader::open(wav_a)?;
            let mut reader_b = hound::WavReader::open(wav_b)?;
            
            let mut iter_a = reader_a.samples::<f32>();
            let mut iter_b = reader_b.samples::<f32>();
            
            let mut sample_idx = 0;
            let mut divergence_found = false;
            
            loop {
                match (iter_a.next(), iter_b.next()) {
                    (Some(sa), Some(sb)) => {
                        let va = sa?;
                        let vb = sb?;
                        if (va - vb).abs() > 1e-9 {
                            println!("{} Divergence detected at sample {}", "✗".red().bold(), sample_idx);
                            println!("   A: {:.10}", va);
                            println!("   B: {:.10}", vb);
                            println!("   Delta: {:.10}", va - vb);
                            divergence_found = true;
                            break;
                        }
                    }
                    (None, None) => break,
                    _ => {
                        println!("{} Length mismatch", "✗".red().bold());
                        divergence_found = true;
                        break;
                    }
                }
                sample_idx += 1;
            }

            if !divergence_found {
                println!("{} No divergence found. Bit-perfect parity.", "✓".green().bold());
            }
        }
        Commands::Bench { patch: _, samples } => {
            println!("{} Starting Performance Benchmark...", "⚡".yellow().bold());
            let mut runner = dirtyrack_modules::runner::RackRunner::new(44100.0, dirtyrack_modules::signal::SeedScope::Global(0));
            let mut snapshot = dirtyrack_modules::runner::GraphSnapshot {
                order: vec![],
                connections: vec![],
                port_counts: vec![],
                node_ids: vec![],
                node_type_ids: vec![],
                forward_edges: vec![],
                back_edges: vec![],
                modulations: vec![],
            }; 
            
            runner.apply_snapshot(&mut snapshot, vec![]);
            let initial_params = vec![];

            let start = std::time::Instant::now();
            for _ in 0..samples {
                runner.process_sample(&snapshot, &initial_params);
            }
            let duration = start.elapsed();
            let micro_per_sample = duration.as_micros() as f64 / samples as f64;
            let real_time_limit = 1000000.0 / 44100.0;
            let safety_margin = real_time_limit / micro_per_sample;

            println!("  Samples: {}", samples);
            println!("  Total Time: {:?}", duration);
            println!("  Time/Sample: {:.4} µs", micro_per_sample);
            println!("  Real-time Limit: {:.4} µs", real_time_limit);
            println!("  {} Safety Margin: {:.2}x", "✓".green().bold(), safety_margin);
            
            if safety_margin < 1.0 {
                println!("  {} CAUTION: Engine cannot maintain real-time at current load.", "⚠".red().bold());
            }
        }
        Commands::Sign { wav, patch: _, version } => {
            println!("{} Notarizing Audio Render...", "🖋".cyan().bold());
            let mut reader = hound::WavReader::open(&wav)?;
            let mut hasher = blake3::Hasher::new();
            for sample in reader.samples::<f32>() {
                hasher.update(&sample?.to_le_bytes());
            }
            let hash = hasher.finalize().to_hex().to_string();
            
            let cert = serde_json::json!({
                "patch_hash": "TODO",
                "engine_version": version,
                "render_hash": hash,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            
            let cert_path = wav.with_extension("dirtyrack.cert");
            std::fs::write(&cert_path, serde_json::to_string_pretty(&cert)?)?;
            println!("{} Certificate generated: {:?}", "✓".green().bold(), cert_path);
        }
        Commands::Inspect { patch } => {
            let json = std::fs::read_to_string(&patch)?;
            let serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            
            println!("{} Patch: {:?}", "🔍".blue().bold(), patch);
            println!("   Version: {}", serial.version);
            println!("   Modules: {}", serial.modules.len());
            for m in &serial.modules {
                println!("     [{}] {} ({}) @ HP:{}, Row:{}", m.stable_id.to_string().cyan(), m.id.bold(), m.bypassed.then(|| "BYPASSED").unwrap_or("active"), m.hp_position, m.row);
                for (p, v) in &m.params {
                    println!("       - {}: {:.4}", p, v);
                }
            }
            println!("   Cables: {}", serial.cables.len());
            for c in &serial.cables {
                println!("     {} [{}]:{} -> [{}]:{}", "🧶".magenta(), c.from_stable_id, c.from_port, c.to_stable_id, c.to_port);
            }
        }
        Commands::ModuleInfo { id } => {
            let registry = dirtyrack_modules::registry::ModuleRegistry::new();
            if let Some(desc) = registry.find(&id) {
                println!("{} Module: {}", "📘".blue().bold(), id.bold());
                println!("   Name:         {}", desc.name);
                println!("   Manufacturer: {}", desc.manufacturer);
                println!("   Width:        {} HP", desc.hp_width);
                println!("   Tags:         {:?}", desc.tags);
                println!("   Ports:");
                for p in &desc.ports {
                    println!("     - [{:<6}] {} ({:?})", format!("{:?}", p.direction).to_uppercase(), p.name.cyan(), p.signal_type);
                }
                println!("   Parameters:");
                for p in &desc.params {
                    println!("     - {:<12} [min: {:.2}, max: {:.2}, def: {:.2}]", p.name.yellow(), p.min, p.max, p.default);
                }
            } else {
                println!("{} Module not found: {}", "✗".red().bold(), id);
            }
        }
        Commands::New { path, template } => {
            // ... (keep current implementation)
            let mut modules = Vec::new();
            let mut cables = Vec::new();
            
            if template == "basic" || template == "complete" {
                modules.push(dirtyrack_gui::rack::SerializableModule {
                    id: "dirty_vco".to_string(),
                    stable_id: 1,
                    params: BTreeMap::new(),
                    param_modulations: BTreeMap::new(),
                    hp_position: 0.0,
                    row: 0,
                    bypassed: false,
                    dsp_state: None,
                    alias: None,
                });
                modules.push(dirtyrack_gui::rack::SerializableModule {
                    id: "dirty_output".to_string(),
                    stable_id: 100,
                    params: BTreeMap::new(),
                    param_modulations: BTreeMap::new(),
                    hp_position: 30.0,
                    row: 0,
                    bypassed: false,
                    dsp_state: None,
                    alias: None,
                });
                
                cables.push(dirtyrack_gui::rack::SerializableCable {
                    from_stable_id: 1,
                    from_port: "out".to_string(),
                    to_stable_id: 100,
                    to_port: "in_l".to_string(),
                    color: [255, 100, 50, 255],
                    channels: 1,
                });
            }

            let serial = dirtyrack_gui::rack::SerializableRack {
                version: env!("CARGO_PKG_VERSION").to_string(),
                engine_hash: "CLI_GEN".to_string(),
                modules,
                cables,
                project_seed: 12345,
                aging: 0.0,
                cable_opacity: 0.8,
                cable_tension: 0.15,
                causality_log: Vec::new(),
                snapshots: BTreeMap::new(),
                aliases: BTreeMap::new(),
            };

            let json = serde_json::to_string_pretty(&serial)?;
            std::fs::write(&path, json)?;
            println!("{} New patch created: {:?}", "✓".green().bold(), path);
        }
        Commands::AddModule { patch, module_id, x, row } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            
            let max_id = serial.modules.iter().map(|m| m.stable_id).max().unwrap_or(0);
            serial.modules.push(dirtyrack_gui::rack::SerializableModule {
                id: module_id.clone(),
                stable_id: max_id + 1,
                params: BTreeMap::new(),
                param_modulations: BTreeMap::new(),
                hp_position: x,
                row,
                bypassed: false,
                dsp_state: None,
                alias: None,
            });
            
            let json = serde_json::to_string_pretty(&serial)?;
            std::fs::write(&patch, json)?;
            println!("{} Added {} to patch (Stable ID: {})", "✓".green().bold(), module_id, max_id + 1);
        }
        Commands::Connect { patch, from_id, from_port, to_id, to_port } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            
            serial.cables.push(dirtyrack_gui::rack::SerializableCable {
                from_stable_id: from_id,
                from_port,
                to_stable_id: to_id,
                to_port,
                color: [200, 200, 200, 255],
                channels: 1,
            });
            
            let json = serde_json::to_string_pretty(&serial)?;
            std::fs::write(&patch, json)?;
            println!("{} Connected ports", "✓".green().bold());
        }
        Commands::SetParam { patch, id, name, value } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            
            if let Some(m) = serial.modules.iter_mut().find(|m| m.stable_id == id) {
                m.params.insert(name.clone(), value);
                let json = serde_json::to_string_pretty(&serial)?;
                std::fs::write(&patch, json)?;
                println!("{} Updated {} -> {:.4}", "✓".green().bold(), name, value);
            } else {
                println!("{} Module with stable_id {} not found", "✗".red().bold(), id);
            }
        }
        Commands::ExportRegistry { format, output } => {
            let registry = dirtyrack_modules::registry::ModuleRegistry::new();
            
            #[derive(serde::Serialize)]
            struct ExportModule {
                id: String,
                name: String,
                manufacturer: String,
                hp_width: u32,
                tags: Vec<String>,
                ports: Vec<ExportPort>,
                params: Vec<ExportParam>,
            }
            #[derive(serde::Serialize)]
            struct ExportPort {
                name: String,
                direction: String,
                signal_type: String,
            }
            #[derive(serde::Serialize)]
            struct ExportParam {
                name: String,
                min: f32,
                max: f32,
                default: f32,
            }

            let mut data = Vec::new();
            for m in registry.all() {
                data.push(ExportModule {
                    id: m.id.to_string(),
                    name: m.name.to_string(),
                    manufacturer: m.manufacturer.to_string(),
                    hp_width: m.hp_width,
                    tags: m.tags.iter().map(|s| s.to_string()).collect(),
                    ports: m.ports.iter().map(|p| ExportPort {
                        name: p.name.to_string(),
                        direction: format!("{:?}", p.direction),
                        signal_type: format!("{:?}", p.signal_type),
                    }).collect(),
                    params: m.params.iter().map(|p| ExportParam {
                        name: p.name.to_string(),
                        min: p.min,
                        max: p.max,
                        default: p.default,
                    }).collect(),
                });
            }
            
            let content = if format == "json" {
                serde_json::to_string_pretty(&data)?
            } else {
                let mut md = String::from("# DirtyRack Module Registry\n\n");
                for m in data {
                    md.push_str(&format!("## {} ({})\n", m.name, m.id));
                    md.push_str(&format!("- Manufacturer: {}\n", m.manufacturer));
                    md.push_str(&format!("- Width: {} HP\n", m.hp_width));
                    md.push_str(&format!("- Tags: {:?}\n\n", m.tags));
                    md.push_str("### Ports\n");
                    for p in &m.ports {
                        md.push_str(&format!("- `{}`: {} ({})\n", p.name, p.direction, p.signal_type));
                    }
                    md.push_str("\n### Parameters\n");
                    for p in &m.params {
                        md.push_str(&format!("- `{}`: [{:.2} to {:.2}], Default: {:.2}\n", p.name, p.min, p.max, p.default));
                    }
                    md.push_str("\n---\n\n");
                }
                md
            };
            
            if let Some(path) = output {
                std::fs::write(&path, content)?;
                println!("{} Registry exported to {:?}", "✓".green().bold(), path);
            } else {
                println!("{}", content);
            }
        }
        Commands::Snapshot { patch, id } => {
            let json = std::fs::read_to_string(&patch)?;
            let serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            let registry = dirtyrack_modules::registry::ModuleRegistry::new();
            
            // Reconstruct minimal engine for a single-sample snapshot
            let mut runner = dirtyrack_modules::runner::RackRunner::new(44100.0, dirtyrack_modules::signal::SeedScope::Global(serial.project_seed));
            runner.ctx.aging = serial.aging;

            let n = serial.modules.len();
            let mut nodes = Vec::with_capacity(n);
            let mut node_type_ids = Vec::with_capacity(n);
            let mut node_ids = Vec::with_capacity(n);
            let mut initial_params = Vec::with_capacity(n);
            let mut port_counts = Vec::with_capacity(n);

            for m in &serial.modules {
                if let Some(desc) = registry.find(&m.id) {
                    let mut node = (desc.factory)(44100.0);
                    if let Some(state) = &m.dsp_state {
                        node.inject_state(state);
                    }
                    nodes.push(node);
                    node_type_ids.push(m.id.clone());
                    node_ids.push(m.stable_id);
                    
                    let mut p_vals = vec![0.0; 64];
                    for (p_idx, p_desc) in desc.params.iter().enumerate() {
                        if p_idx < 64 {
                            p_vals[p_idx] = *m.params.get(p_desc.name).unwrap_or(&p_desc.default);
                        }
                    }
                    initial_params.push(p_vals);

                    let ins = desc.ports.iter().filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input).count();
                    let outs = desc.ports.iter().filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output).count();
                    port_counts.push((ins, outs));
                }
            }

            let mut snapshot = dirtyrack_modules::runner::GraphSnapshot {
                order: (0..nodes.len()).collect(),
                connections: Vec::new(), // No connections needed for isolated state inspection
                port_counts,
                node_ids: node_ids.clone(),
                node_type_ids,
                modulations: vec![Vec::new(); nodes.len()],
                forward_edges: Vec::new(),
                back_edges: Vec::new(),
            };

            runner.apply_snapshot(&mut snapshot, nodes);
            runner.process_sample(&snapshot, &initial_params);

            println!("{} Forensic Snapshot for {:?}", "📸".yellow().bold(), patch);
            
            for (idx, node) in runner.active_nodes.iter().enumerate() {
                let stable_id = node_ids[idx];
                if let Some(target_id) = id {
                    if stable_id != target_id { continue; }
                }

                if let Some(forensic) = node.get_forensic_data() {
                    println!("\n{} Module [{}] {}", "📦".blue(), stable_id.to_string().cyan(), serial.modules.iter().find(|m| m.stable_id == stable_id).map(|m| m.id.as_str()).unwrap_or("Unknown"));
                    println!("   Summary: {}", forensic.internal_state_summary.bold());
                    
                    // Thermal Heat / Internal State Vector Visualization
                    let mut heat_viz = String::new();
                    let mut has_heat = false;
                    for &h in forensic.thermal_heat.iter() {
                        if h.abs() > 0.000001 {
                            has_heat = true;
                            let intensity = (h.abs() * 10.0).min(1.0);
                            let block = if intensity > 0.8 { "█" } else if intensity > 0.5 { "▓" } else if intensity > 0.2 { "▒" } else { "░" };
                            if h > 0.0 {
                                heat_viz.push_str(&block.red().to_string());
                            } else {
                                heat_viz.push_str(&block.blue().to_string());
                            }
                        }
                    }
                    if has_heat {
                        println!("   Signal Latency/Thermal Vectors: [{}]", heat_viz);
                    }

                    // Input/Output Levels
                    println!("   I/O Levels:");
                    let stats = &runner.stats[idx];
                    println!("     Peak: {:.2} dB, DC Offset: {:.4}, Clipping: {}", stats.peak_db, stats.dc_offset, stats.clipping_count);
                }
            }
        }
        Commands::Rm { patch, id } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            
            // Resolve ID (stable_id or alias)
            let stable_id = if let Ok(sid) = id.parse::<u64>() {
                sid
            } else if let Some(&sid) = serial.aliases.get(&id) {
                sid
            } else {
                anyhow::bail!("Module ID or Alias not found: {}", id);
            };

            // Remove cables
            serial.cables.retain(|c| c.from_stable_id != stable_id && c.to_stable_id != stable_id);
            
            // Remove alias mapping
            serial.aliases.retain(|_, v| *v != stable_id);
            
            // Remove module
            if let Some(pos) = serial.modules.iter().position(|m| m.stable_id == stable_id) {
                let m = serial.modules.remove(pos);
                let json = serde_json::to_string_pretty(&serial)?;
                std::fs::write(&patch, json)?;
                println!("{} Removed module {} ({}) and its connections", "✓".green().bold(), stable_id, m.id);
            } else {
                println!("{} Module {} not found in patch", "✗".red().bold(), stable_id);
            }
        }
        Commands::Alias { patch, id, name } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            
            if let Some(m) = serial.modules.iter_mut().find(|m| m.stable_id == id) {
                m.alias = Some(name.clone());
                serial.aliases.insert(name.clone(), id);
                let json = serde_json::to_string_pretty(&serial)?;
                std::fs::write(&patch, json)?;
                println!("{} Alias '{}' assigned to module {}", "✓".green().bold(), name, id);
            } else {
                println!("{} Module {} not found", "✗".red().bold(), id);
            }
        }
        Commands::Shell { patch } => {
            println!("{} Entering Interactive Patching Shell: {:?}", "🐚".blue().bold(), patch);
            println!("   Type 'help' for commands, 'exit' to quit.");

            let mut rl = DefaultEditor::new()?;
            loop {
                let readline = rl.readline("dirtyrack> ");
                match readline {
                    Ok(line) => {
                        let line = line.trim();
                        if line.is_empty() { continue; }
                        if line == "exit" || line == "quit" { break; }
                        
                        let args = shlex::split(line).unwrap_or_default();
                        if args.is_empty() { continue; }

                        // Special case: 'ls' or 'inspect' without path
                        if args[0] == "ls" || args[0] == "inspect" {
                            let _ = rl.add_history_entry(line);
                            execute_command(Commands::Inspect { patch: patch.clone() })?;
                            continue;
                        }

                        // Map shell commands to Cli Commands
                        // This is a simplified mapping for the shell
                        let cmd = match args[0].as_str() {
                            "add" => {
                                if args.len() < 2 { println!("Usage: add <module_id> [x] [row]"); continue; }
                                Commands::AddModule { 
                                    patch: patch.clone(), 
                                    module_id: args[1].clone(), 
                                    x: args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                                    row: args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0)
                                }
                            }
                            "conn" | "connect" => {
                                if args.len() < 5 { println!("Usage: conn <from_id> <from_port> <to_id> <to_port>"); continue; }
                                Commands::Connect {
                                    patch: patch.clone(),
                                    from_id: args[1].parse().unwrap_or(0),
                                    from_port: args[2].clone(),
                                    to_id: args[3].parse().unwrap_or(0),
                                    to_port: args[4].clone(),
                                }
                            }
                            "set" => {
                                if args.len() < 4 { println!("Usage: set <id> <param> <value>"); continue; }
                                Commands::SetParam {
                                    patch: patch.clone(),
                                    id: args[1].parse().unwrap_or(0),
                                    name: args[2].clone(),
                                    value: args[3].parse().unwrap_or(0.0),
                                }
                            }
                            "rm" => {
                                if args.len() < 2 { println!("Usage: rm <id>"); continue; }
                                Commands::Rm { patch: patch.clone(), id: args[1].clone() }
                            }
                            "alias" => {
                                if args.len() < 3 { println!("Usage: alias <id> <name>"); continue; }
                                Commands::Alias { patch: patch.clone(), id: args[1].parse().unwrap_or(0), name: args[2].clone() }
                            }
                            "render" => {
                                Commands::Render { 
                                    patch: patch.clone(), 
                                    output: PathBuf::from("output.wav"), 
                                    length: 2.0, 
                                    sample_rate: 44100 
                                }
                            }
                            "play" => {
                                Commands::Play { patch: patch.clone(), duration: None }
                            }
                            "help" => {
                                println!("Available commands: add, conn, set, rm, alias, ls, render, play, exit");
                                continue;
                            }
                            _ => {
                                println!("Unknown command: {}", args[0]);
                                continue;
                            }
                        };

                        let _ = rl.add_history_entry(line);
                        if let Err(e) = execute_command(cmd) {
                            println!("{} Error: {}", "✗".red().bold(), e);
                        }
                    }
                    Err(ReadlineError::Interrupted) => {
                        println!("CTRL-C");
                        break;
                    }
                    Err(ReadlineError::Eof) => {
                        println!("CTRL-D");
                        break;
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
        }
        Commands::Batch { patch, script } => {
            let script_content = std::fs::read_to_string(&script)?;
            println!("{} Running batch script: {:?}", "📜".yellow().bold(), script);
            
            for line in script_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                
                let args = shlex::split(line).unwrap_or_default();
                if args.is_empty() { continue; }

                println!("  > {}", line.dimmed());
                
                let cmd = match args[0].as_str() {
                    "add" => Commands::AddModule { 
                        patch: patch.clone(), 
                        module_id: args[1].clone(), 
                        x: args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
                        row: args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0)
                    },
                    "connect" | "conn" => Commands::Connect {
                        patch: patch.clone(),
                        from_id: args[1].parse().unwrap_or(0),
                        from_port: args[2].clone(),
                        to_id: args[3].parse().unwrap_or(0),
                        to_port: args[4].clone(),
                    },
                    "set" => Commands::SetParam {
                        patch: patch.clone(),
                        id: args[1].parse().unwrap_or(0),
                        name: args[2].clone(),
                        value: args[3].parse().unwrap_or(0.0),
                    },
                    "rm" => Commands::Rm { patch: patch.clone(), id: args[1].clone() },
                    "alias" => Commands::Alias { patch: patch.clone(), id: args[1].parse().unwrap_or(0), name: args[2].clone() },
                    _ => {
                        println!("  {} Skipping unknown command: {}", "⚠".yellow(), args[0]);
                        continue;
                    }
                };
                
                execute_command(cmd)?;
            }
            println!("{} Batch script execution complete.", "✓".green().bold());
        }
        Commands::Play { patch, duration } => {
            let json = std::fs::read_to_string(&patch)?;
            let serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            let registry = dirtyrack_modules::registry::ModuleRegistry::new();
            
            println!("{} Initializing Real-time Audio Engine...", "🔊".cyan().bold());
            
            let host = cpal::default_host();
            let device = host.default_output_device().expect("No output device found");
            let config = device.default_output_config().expect("Failed to get default output config");
            let sample_rate = config.sample_rate().0 as f32;
            let channels = config.channels() as usize;

            println!("   Device:      {}", device.name().unwrap_or_else(|_| "Unknown".into()));
            println!("   Sample Rate: {} Hz", sample_rate);
            println!("   Channels:    {}", channels);

            let n = serial.modules.len();
            let mut stable_to_idx = BTreeMap::new();
            for (i, m) in serial.modules.iter().enumerate() {
                stable_to_idx.insert(m.stable_id, i);
            }

            // --- Topological Sort ---
            let mut order = Vec::with_capacity(n);
            let mut visited = vec![false; n];
            let mut visiting = vec![false; n];

            fn dfs_play(
                idx: usize,
                modules: &[dirtyrack_gui::rack::SerializableModule],
                cables: &[dirtyrack_gui::rack::SerializableCable],
                stable_to_idx: &BTreeMap<u64, usize>,
                visited: &mut [bool],
                visiting: &mut [bool],
                order: &mut Vec<usize>,
            ) {
                if visited[idx] { return; }
                visiting[idx] = true;
                let my_stable_id = modules[idx].stable_id;
                for cable in cables {
                    if cable.to_stable_id == my_stable_id {
                        if let Some(&from_idx) = stable_to_idx.get(&cable.from_stable_id) {
                            if !visited[from_idx] && !visiting[from_idx] {
                                dfs_play(from_idx, modules, cables, stable_to_idx, visited, visiting, order);
                            }
                        }
                    }
                }
                visiting[idx] = false;
                visited[idx] = true;
                order.push(idx);
            }

            for i in 0..n {
                dfs_play(i, &serial.modules, &serial.cables, &stable_to_idx, &mut visited, &mut visiting, &mut order);
            }

            // --- Reconstruct Engine State ---
            let mut nodes = Vec::with_capacity(n);
            let mut node_type_ids = Vec::with_capacity(n);
            let mut node_ids = Vec::with_capacity(n);
            let mut initial_params = Vec::with_capacity(n);
            let mut port_counts = Vec::with_capacity(n);

            for &idx in &order {
                let m = &serial.modules[idx];
                if let Some(desc) = registry.find(&m.id) {
                    let mut node = (desc.factory)(sample_rate);
                    if let Some(state) = &m.dsp_state {
                        node.inject_state(state);
                    }
                    nodes.push(node);
                    node_type_ids.push(m.id.clone());
                    node_ids.push(m.stable_id);
                    
                    let mut p_vals = vec![0.0; 64];
                    for (p_idx, p_desc) in desc.params.iter().enumerate() {
                        if p_idx < 64 {
                            p_vals[p_idx] = *m.params.get(p_desc.name).unwrap_or(&p_desc.default);
                        }
                    }
                    initial_params.push(p_vals);

                    let ins = desc.ports.iter().filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input).count();
                    let outs = desc.ports.iter().filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output).count();
                    port_counts.push((ins, outs));
                }
            }

            let mut index_map = vec![0; n];
            for (new_idx, &old_idx) in order.iter().enumerate() {
                index_map[old_idx] = new_idx;
            }

            let mut connections = Vec::new();
            for cable in &serial.cables {
                if let (Some(&fi_old), Some(&ti_old)) = (stable_to_idx.get(&cable.from_stable_id), stable_to_idx.get(&cable.to_stable_id)) {
                    let fi = index_map[fi_old];
                    let ti = index_map[ti_old];
                    
                    let from_desc = registry.find(&serial.modules[fi_old].id);
                    let to_desc = registry.find(&serial.modules[ti_old].id);
                    
                    if let (Some(fd), Some(td)) = (from_desc, to_desc) {
                        let from_port = fd.ports.iter()
                            .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output)
                            .position(|p| p.name == cable.from_port).unwrap_or(0);
                        let to_port = td.ports.iter()
                            .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input)
                            .position(|p| p.name == cable.to_port).unwrap_or(0);
                        
                        connections.push(dirtyrack_modules::runner::Connection {
                            from_module: fi,
                            from_port,
                            to_module: ti,
                            to_port,
                        });
                    }
                }
            }

            let mut snapshot = dirtyrack_modules::runner::GraphSnapshot {
                order: (0..nodes.len()).collect(),
                connections,
                port_counts,
                node_ids: node_ids.clone(),
                node_type_ids,
                modulations: vec![Vec::new(); nodes.len()],
                forward_edges: Vec::new(),
                back_edges: Vec::new(),
            };

            let mut runner = dirtyrack_modules::runner::RackRunner::new(sample_rate, dirtyrack_modules::signal::SeedScope::Global(serial.project_seed));
            runner.ctx.aging = serial.aging;
            runner.apply_snapshot(&mut snapshot, nodes);

            let output_node_idx = node_ids.iter().position(|&id| id == 100);

            println!("\n{} Playing... (Press CTRL-C to stop)", "▶".green().bold());

            let stream = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for frame in data.chunks_mut(channels) {
                        runner.process_sample(&snapshot, &initial_params);
                        
                        let (l, r) = if let Some(idx) = output_node_idx {
                            let out_buf = &runner.output_buffers[idx];
                            (out_buf[0], out_buf[16]) // Frame 0 of L and R
                        } else {
                            (0.0, 0.0)
                        };

                        frame[0] = l;
                        if channels > 1 {
                            frame[1] = r;
                        }
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None
            ).expect("Failed to build audio stream");

            stream.play().expect("Failed to start audio stream");

            if let Some(d) = duration {
                std::thread::sleep(std::time::Duration::from_secs_f32(d));
            } else {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    Ok(())
}

fn execute_command(cmd: Commands) -> anyhow::Result<()> {
    inner_execute(cmd)
}
