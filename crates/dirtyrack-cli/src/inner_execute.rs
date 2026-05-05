#![allow(clippy::all, unused, dead_code)]

use crate::Commands;
use colored::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::BTreeMap;

pub fn inner_execute(command: Commands) -> anyhow::Result<()> {
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
            let mut runner = dirtyrack_modules::runner::RackRunner::new(
                sample_rate as f32,
                dirtyrack_modules::signal::SeedScope::Global(serial.project_seed),
            );
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
                if visited[idx] {
                    return;
                }
                visiting[idx] = true;
                let my_stable_id = modules[idx].stable_id;
                for cable in cables {
                    if cable.to_stable_id == my_stable_id {
                        if let Some(&from_idx) = stable_to_idx.get(&cable.from_stable_id) {
                            if !visited[from_idx] && !visiting[from_idx] {
                                dfs(
                                    from_idx,
                                    modules,
                                    cables,
                                    stable_to_idx,
                                    visited,
                                    visiting,
                                    order,
                                );
                            }
                        }
                    }
                }
                visiting[idx] = false;
                visited[idx] = true;
                order.push(idx);
            }

            for i in 0..n {
                dfs(
                    i,
                    &serial.modules,
                    &serial.cables,
                    &stable_to_idx,
                    &mut visited,
                    &mut visiting,
                    &mut order,
                );
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

                    let ins = desc
                        .ports
                        .iter()
                        .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input)
                        .count();
                    let outs = desc
                        .ports
                        .iter()
                        .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output)
                        .count();
                    port_counts.push((ins, outs));
                }
            }

            let mut index_map = vec![0; n];
            for (new_idx, &old_idx) in order.iter().enumerate() {
                index_map[old_idx] = new_idx;
            }

            let mut connections = Vec::new();
            for cable in &serial.cables {
                if let (Some(&fi_old), Some(&ti_old)) = (
                    stable_to_idx.get(&cable.from_stable_id),
                    stable_to_idx.get(&cable.to_stable_id),
                ) {
                    let fi = index_map[fi_old];
                    let ti = index_map[ti_old];

                    let from_desc = registry.find(&serial.modules[fi_old].id);
                    let to_desc = registry.find(&serial.modules[ti_old].id);

                    if let (Some(fd), Some(td)) = (from_desc, to_desc) {
                        let from_port = fd
                            .ports
                            .iter()
                            .filter(|p| {
                                p.direction == dirtyrack_modules::signal::PortDirection::Output
                            })
                            .position(|p| p.name == cable.from_port)
                            .unwrap_or(0);
                        let to_port = td
                            .ports
                            .iter()
                            .filter(|p| {
                                p.direction == dirtyrack_modules::signal::PortDirection::Input
                            })
                            .position(|p| p.name == cable.to_port)
                            .unwrap_or(0);

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
                modulations: vec![Vec::new(); n],
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

            let output_node_idx = order
                .iter()
                .position(|&idx| serial.modules[idx].id == "dirty_output");

            for _ in 0..total_samples {
                runner.process_sample(&snapshot, &initial_params);

                let mut left = 0.0;
                let mut right = 0.0;

                if let Some(out_idx) = output_node_idx {
                    left = runner.get_output(out_idx, 0);
                    right = runner.get_output(out_idx, 1);
                } else if !runner.output_buffers.is_empty() {
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
        Commands::DiffRender {
            wav_a,
            cert_a: _,
            wav_b,
            cert_b: _,
        } => {
            println!(
                "{} Starting A/B Differential Audit...",
                "📊".magenta().bold()
            );
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
                            println!(
                                "{} Divergence detected at sample {}",
                                "✗".red().bold(),
                                sample_idx
                            );
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
                println!(
                    "{} No divergence found. Bit-perfect parity.",
                    "✓".green().bold()
                );
            }
        }
        Commands::Bench { patch: _, samples } => {
            println!("{} Starting Performance Benchmark...", "⚡".yellow().bold());
            let mut runner = dirtyrack_modules::runner::RackRunner::new(
                44100.0,
                dirtyrack_modules::signal::SeedScope::Global(0),
            );
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
            println!(
                "  {} Safety Margin: {:.2}x",
                "✓".green().bold(),
                safety_margin
            );

            if safety_margin < 1.0 {
                println!(
                    "  {} CAUTION: Engine cannot maintain real-time at current load.",
                    "⚠".red().bold()
                );
            }
        }
        Commands::Sign {
            wav,
            patch: _,
            version,
        } => {
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
            println!(
                "{} Certificate generated: {:?}",
                "✓".green().bold(),
                cert_path
            );
        }
        Commands::Inspect { patch } => {
            let json = std::fs::read_to_string(&patch)?;
            let serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;

            println!("{} Patch: {:?}", "🔍".blue().bold(), patch);
            println!("   Version: {}", serial.version);
            println!("   Modules: {}", serial.modules.len());
            for m in &serial.modules {
                println!(
                    "     [{}] {} ({}) @ HP:{}, Row:{}",
                    m.stable_id.to_string().cyan(),
                    m.id.bold(),
                    m.bypassed.then(|| "BYPASSED").unwrap_or("active"),
                    m.hp_position,
                    m.row
                );
                for (p, v) in &m.params {
                    println!("       - {}: {:.4}", p, v);
                }
            }
            println!("   Cables: {}", serial.cables.len());
            for c in &serial.cables {
                println!(
                    "     {} [{}]:{} -> [{}]:{}",
                    "🧶".magenta(),
                    c.from_stable_id,
                    c.from_port,
                    c.to_stable_id,
                    c.to_port
                );
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
                    println!(
                        "     - [{:<6}] {} ({:?})",
                        format!("{:?}", p.direction).to_uppercase(),
                        p.name.cyan(),
                        p.signal_type
                    );
                }
                println!("   Parameters:");
                for p in &desc.params {
                    println!(
                        "     - {:<12} [min: {:.2}, max: {:.2}, def: {:.2}]",
                        p.name.yellow(),
                        p.min,
                        p.max,
                        p.default
                    );
                }
            } else {
                println!("{} Module not found: {}", "✗".red().bold(), id);
            }
        }
        Commands::New { path, template } => {
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
                    subpatch_path: None,
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
                    subpatch_path: None,
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
        Commands::AddModule {
            patch,
            module_id,
            x,
            row,
        } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;

            let max_id = serial
                .modules
                .iter()
                .map(|m| m.stable_id)
                .max()
                .unwrap_or(0);
            serial
                .modules
                .push(dirtyrack_gui::rack::SerializableModule {
                    id: module_id.clone(),
                    stable_id: max_id + 1,
                    params: BTreeMap::new(),
                    param_modulations: BTreeMap::new(),
                    hp_position: x,
                    row,
                    bypassed: false,
                    dsp_state: None,
                    alias: None,
                    subpatch_path: None,
                });

            let json = serde_json::to_string_pretty(&serial)?;
            std::fs::write(&patch, json)?;
            println!(
                "{} Added {} to patch (Stable ID: {})",
                "✓".green().bold(),
                module_id,
                max_id + 1
            );
        }
        Commands::Connect {
            patch,
            from_id,
            from_port,
            to_id,
            to_port,
        } => {
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
        Commands::SetParam {
            patch,
            id,
            name,
            value,
        } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;

            if let Some(m) = serial.modules.iter_mut().find(|m| m.stable_id == id) {
                m.params.insert(name.clone(), value);
                let json = serde_json::to_string_pretty(&serial)?;
                std::fs::write(&patch, json)?;
                println!("{} Updated {} -> {:.4}", "✓".green().bold(), name, value);
            } else {
                println!(
                    "{} Module with stable_id {} not found",
                    "✗".red().bold(),
                    id
                );
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
                    ports: m
                        .ports
                        .iter()
                        .map(|p| ExportPort {
                            name: p.name.to_string(),
                            direction: format!("{:?}", p.direction),
                            signal_type: format!("{:?}", p.signal_type),
                        })
                        .collect(),
                    params: m
                        .params
                        .iter()
                        .map(|p| ExportParam {
                            name: p.name.to_string(),
                            min: p.min,
                            max: p.max,
                            default: p.default,
                        })
                        .collect(),
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
                        md.push_str(&format!(
                            "- `{}`: {} ({})\n",
                            p.name, p.direction, p.signal_type
                        ));
                    }
                    md.push_str("\n### Parameters\n");
                    for p in &m.params {
                        md.push_str(&format!(
                            "- `{}`: [{:.2} to {:.2}], Default: {:.2}\n",
                            p.name, p.min, p.max, p.default
                        ));
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

            let mut runner = dirtyrack_modules::runner::RackRunner::new(
                44100.0,
                dirtyrack_modules::signal::SeedScope::Global(serial.project_seed),
            );
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

                    let ins = desc
                        .ports
                        .iter()
                        .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input)
                        .count();
                    let outs = desc
                        .ports
                        .iter()
                        .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output)
                        .count();
                    port_counts.push((ins, outs));
                }
            }

            let mut snapshot = dirtyrack_modules::runner::GraphSnapshot {
                order: (0..nodes.len()).collect(),
                connections: Vec::new(),
                port_counts,
                node_ids,
                node_type_ids,
                modulations: vec![Vec::new(); n],
                forward_edges: Vec::new(),
                back_edges: Vec::new(),
            };

            runner.apply_snapshot(&mut snapshot, nodes);
            runner.process_sample(&snapshot, &initial_params);

            if let Some(sid) = id {
                if let Some(idx) = serial.modules.iter().position(|m| m.stable_id == sid) {
                    let m = &serial.modules[idx];
                    println!(
                        "{} Module Snapshot: {} (ID: {})",
                        "📸".yellow().bold(),
                        m.id,
                        sid
                    );
                    if let Some(node) = runner.active_nodes.get(idx) {
                        println!("   Internal State: {:?}", node.extract_state());
                    }
                }
            } else {
                println!(
                    "{} Patch Snapshot Captured ({} modules)",
                    "📸".yellow().bold(),
                    n
                );
            }
        }
        Commands::Rm { patch, id } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;

            let sid = if let Ok(n) = id.parse::<u64>() {
                Some(n)
            } else {
                serial.aliases.get(&id).copied()
            };

            if let Some(target_sid) = sid {
                serial.modules.retain(|m| m.stable_id != target_sid);
                serial
                    .cables
                    .retain(|c| c.from_stable_id != target_sid && c.to_stable_id != target_sid);
                let mut alias_to_rm = None;
                for (a, &s) in &serial.aliases {
                    if s == target_sid {
                        alias_to_rm = Some(a.clone());
                        break;
                    }
                }
                if let Some(a) = alias_to_rm {
                    serial.aliases.remove(&a);
                }

                let json = serde_json::to_string_pretty(&serial)?;
                std::fs::write(&patch, json)?;
                println!("{} Removed module {}", "✓".green().bold(), id);
            } else {
                println!("{} Module not found: {}", "✗".red().bold(), id);
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
                println!("{} Aliased {} -> {}", "✓".green().bold(), id, name);
            } else {
                println!("{} Module not found: {}", "✗".red().bold(), id);
            }
        }
        Commands::Shell { patch } => {
            println!(
                "{} Entering Interactive Patching Shell",
                "🐚".magenta().bold()
            );
            let mut rl = DefaultEditor::new()?;

            // Hierarchy State
            let mut path_stack = vec![patch.clone()];
            let mut name_stack = vec![patch.file_name().unwrap().to_string_lossy().to_string()];

            loop {
                // Construct prompt based on hierarchy
                let prompt_path = name_stack.join("/");
                let prompt = format!("{}> ", prompt_path.cyan());

                let readline = rl.readline(&prompt);
                match readline {
                    Ok(line) => {
                        let line_trimmed = line.trim();
                        if line_trimmed.is_empty() {
                            continue;
                        }
                        let _ = rl.add_history_entry(line_trimmed);

                        let parts: Vec<&str> = line_trimmed.split_whitespace().collect();
                        let current_patch = path_stack.last().unwrap();

                        match parts[0] {
                            "exit" | "quit" => break,
                            "help" => {
                                println!("Navigation:");
                                println!("  ls          List modules in current patch");
                                println!("  cd <name>   Enter subpatch (CompositeModule)");
                                println!("  cd ..       Return to parent patch");
                                println!("  pwd         Show current path");
                                println!("Patching:");
                                println!("  inspect     Show details of current patch");
                                println!("  add <id>    Add module (e.g. add dirty_vco)");
                                println!("  rm <id>     Remove module");
                                println!("  connect <from> <port> <to> <port>");
                                println!("  set <id> <param> <val>");
                                println!("System:");
                                println!("  play        Real-time audio monitoring");
                                println!("  exit        Leave shell");
                            }
                            "pwd" => {
                                println!("{}", name_stack.join("/").cyan());
                            }
                            "ls" => {
                                let json = std::fs::read_to_string(current_patch)?;
                                let serial: dirtyrack_gui::rack::SerializableRack =
                                    serde_json::from_str(&json)?;
                                for m in &serial.modules {
                                    let type_info = if m.subpatch_path.is_some() {
                                        "[SUB]".yellow()
                                    } else {
                                        "".normal()
                                    };
                                    let alias_info = m
                                        .alias
                                        .as_ref()
                                        .map(|a| format!(" ({})", a.bold()))
                                        .unwrap_or_default();
                                    println!(
                                        "  [{}] {}{}{}",
                                        m.stable_id.to_string().cyan(),
                                        m.id,
                                        alias_info,
                                        type_info
                                    );
                                }
                            }
                            "inspect" => {
                                inner_execute(Commands::Inspect {
                                    patch: current_patch.clone(),
                                })?;
                            }
                            "cd" => {
                                if parts.len() < 2 {
                                    continue;
                                }
                                if parts[1] == ".." {
                                    if path_stack.len() > 1 {
                                        path_stack.pop();
                                        name_stack.pop();
                                    }
                                } else {
                                    let json = std::fs::read_to_string(current_patch)?;
                                    let serial: dirtyrack_gui::rack::SerializableRack =
                                        serde_json::from_str(&json)?;

                                    // Find target module by ID or alias
                                    let target = if let Ok(sid) = parts[1].parse::<u64>() {
                                        serial.modules.iter().find(|m| m.stable_id == sid)
                                    } else {
                                        serial
                                            .modules
                                            .iter()
                                            .find(|m| m.alias.as_deref() == Some(parts[1]))
                                    };

                                    if let Some(m) = target {
                                        if let Some(sub_path_str) = &m.subpatch_path {
                                            // Handle relative paths from the current patch directory
                                            let mut sub_path =
                                                current_patch.parent().unwrap().to_path_buf();
                                            sub_path.push(sub_path_str);

                                            if sub_path.exists() {
                                                path_stack.push(sub_path);
                                                name_stack.push(parts[1].to_string());
                                            } else {
                                                println!(
                                                    "{} Subpatch file not found: {:?}",
                                                    "✗".red(),
                                                    sub_path
                                                );
                                            }
                                        } else {
                                            println!(
                                                "{} Module '{}' is not a subpatch.",
                                                "✗".red(),
                                                parts[1]
                                            );
                                        }
                                    } else {
                                        println!("{} Module not found: {}", "✗".red(), parts[1]);
                                    }
                                }
                            }
                            "add" => {
                                if parts.len() < 2 {
                                    continue;
                                }
                                inner_execute(Commands::AddModule {
                                    patch: current_patch.clone(),
                                    module_id: parts[1].to_string(),
                                    x: 0.0,
                                    row: 0,
                                })?;
                            }
                            "rm" => {
                                if parts.len() < 2 {
                                    continue;
                                }
                                inner_execute(Commands::Rm {
                                    patch: current_patch.clone(),
                                    id: parts[1].to_string(),
                                })?;
                            }
                            "connect" => {
                                if parts.len() < 5 {
                                    continue;
                                }
                                let f_id = parts[1].parse::<u64>().ok();
                                let t_id = parts[3].parse::<u64>().ok();
                                if let (Some(fid), Some(tid)) = (f_id, t_id) {
                                    inner_execute(Commands::Connect {
                                        patch: current_patch.clone(),
                                        from_id: fid,
                                        from_port: parts[2].to_string(),
                                        to_id: tid,
                                        to_port: parts[4].to_string(),
                                    })?;
                                }
                            }
                            "set" => {
                                if parts.len() < 4 {
                                    continue;
                                }
                                if let (Ok(id), Ok(val)) =
                                    (parts[1].parse::<u64>(), parts[3].parse::<f32>())
                                {
                                    inner_execute(Commands::SetParam {
                                        patch: current_patch.clone(),
                                        id,
                                        name: parts[2].to_string(),
                                        value: val,
                                    })?;
                                }
                            }
                            "play" => {
                                inner_execute(Commands::Play {
                                    patch: current_patch.clone(),
                                    duration: None,
                                })?;
                            }
                            _ => println!(
                                "Unknown command: {}. Type 'help' for available commands.",
                                parts[0]
                            ),
                        }
                    }
                    Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
                    Err(err) => {
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
        }
        Commands::Batch {
            patch: _,
            script: _,
        } => {
            println!(
                "{} Batch processing not yet fully implemented in this refactor.",
                "🚧".yellow()
            );
        }
        Commands::Play { patch, duration } => {
            let json = std::fs::read_to_string(&patch)?;
            let serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;

            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .expect("No output device found");
            let config = device
                .default_output_config()
                .expect("No default output config");
            let sample_rate = config.sample_rate().0 as f32;
            let channels = config.channels() as usize;

            let registry = dirtyrack_modules::registry::ModuleRegistry::new();
            let n = serial.modules.len();
            let mut nodes = Vec::with_capacity(n);
            let mut node_type_ids = Vec::with_capacity(n);
            let mut node_ids = Vec::with_capacity(n);
            let mut initial_params = Vec::with_capacity(n);
            let mut port_counts = Vec::with_capacity(n);
            let mut connections = Vec::new();

            for m in &serial.modules {
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

                    let ins = desc
                        .ports
                        .iter()
                        .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Input)
                        .count();
                    let outs = desc
                        .ports
                        .iter()
                        .filter(|p| p.direction == dirtyrack_modules::signal::PortDirection::Output)
                        .count();
                    port_counts.push((ins, outs));
                }
            }

            let mut stable_to_idx = BTreeMap::new();
            for (i, &sid) in node_ids.iter().enumerate() {
                stable_to_idx.insert(sid, i);
            }

            for cable in &serial.cables {
                if let (Some(&fi), Some(&ti)) = (
                    stable_to_idx.get(&cable.from_stable_id),
                    stable_to_idx.get(&cable.to_stable_id),
                ) {
                    let from_desc = registry.find(&node_type_ids[fi]);
                    let to_desc = registry.find(&node_type_ids[ti]);
                    if let (Some(fd), Some(td)) = (from_desc, to_desc) {
                        let from_port = fd
                            .ports
                            .iter()
                            .filter(|p| {
                                p.direction == dirtyrack_modules::signal::PortDirection::Output
                            })
                            .position(|p| p.name == cable.from_port)
                            .unwrap_or(0);
                        let to_port = td
                            .ports
                            .iter()
                            .filter(|p| {
                                p.direction == dirtyrack_modules::signal::PortDirection::Input
                            })
                            .position(|p| p.name == cable.to_port)
                            .unwrap_or(0);
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

            let mut runner = dirtyrack_modules::runner::RackRunner::new(
                sample_rate,
                dirtyrack_modules::signal::SeedScope::Global(serial.project_seed),
            );
            runner.ctx.aging = serial.aging;
            runner.apply_snapshot(&mut snapshot, nodes);

            let output_node_idx = node_ids.iter().position(|&id| id == 100);

            println!("\n{} Playing... (Press CTRL-C to stop)", "▶".green().bold());

            let stream = device
                .build_output_stream(
                    &config.into(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        for frame in data.chunks_mut(channels) {
                            runner.process_sample(&snapshot, &initial_params);
                            let (l, r) = if let Some(idx) = output_node_idx {
                                let out_buf = &runner.output_buffers[idx];
                                (out_buf[0], out_buf[16])
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
                    None,
                )
                .expect("Failed to build audio stream");

            stream.play().expect("Failed to start audio stream");

            if let Some(d) = duration {
                std::thread::sleep(std::time::Duration::from_secs_f32(d));
            } else {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        Commands::Bundle { patch, ids, name } => {
            let json = std::fs::read_to_string(&patch)?;
            let mut serial: dirtyrack_gui::rack::SerializableRack = serde_json::from_str(&json)?;
            let _registry = dirtyrack_modules::registry::ModuleRegistry::new();

            // Resolve IDs/Aliases to Stable IDs
            let mut target_stable_ids = Vec::new();
            for id_str in ids {
                if let Ok(num) = id_str.parse::<u64>() {
                    target_stable_ids.push(num);
                } else if let Some(&num) = serial.aliases.get(&id_str) {
                    target_stable_ids.push(num);
                } else {
                    println!("{} Unknown module: {}", "✗".red().bold(), id_str);
                    return Ok(());
                }
            }

            println!(
                "{} Bundling {} modules into subpatch: {}.json",
                "📦".yellow().bold(),
                target_stable_ids.len(),
                name
            );

            let mut sub_modules = Vec::new();
            let mut sub_aliases = BTreeMap::new();

            // Extract modules
            for &sid in &target_stable_ids {
                if let Some(pos) = serial.modules.iter().position(|m| m.stable_id == sid) {
                    let m = serial.modules.remove(pos);
                    if let Some(alias) = &m.alias {
                        sub_aliases.insert(alias.clone(), m.stable_id);
                        serial.aliases.remove(alias);
                    }
                    sub_modules.push(m);
                }
            }

            // Extract cables (internal vs boundary)
            let mut sub_cables = Vec::new();
            let mut incoming_cables = Vec::new();
            let mut outgoing_cables = Vec::new();

            let mut i = 0;
            while i < serial.cables.len() {
                let from_internal = target_stable_ids.contains(&serial.cables[i].from_stable_id);
                let to_internal = target_stable_ids.contains(&serial.cables[i].to_stable_id);

                if from_internal && to_internal {
                    sub_cables.push(serial.cables.remove(i));
                } else if !from_internal && to_internal {
                    incoming_cables.push(serial.cables.remove(i));
                } else if from_internal && !to_internal {
                    outgoing_cables.push(serial.cables.remove(i));
                } else {
                    i += 1;
                }
            }

            // --- Bridge: Incoming Signals (Parent -> Subpatch) ---
            let mut subpatch_in_sid =
                sub_modules.iter().map(|m| m.stable_id).max().unwrap_or(0) + 1000;
            let composite_sid = serial
                .modules
                .iter()
                .map(|m| m.stable_id)
                .max()
                .unwrap_or(0)
                + 1;

            let num_in = incoming_cables.len();
            for (idx, mut cable) in incoming_cables.into_iter().enumerate() {
                let port_name = format!("IN_{}", idx);

                sub_modules.push(dirtyrack_gui::rack::SerializableModule {
                    id: "subpatch_in".to_string(),
                    stable_id: subpatch_in_sid,
                    params: BTreeMap::new(),
                    param_modulations: BTreeMap::new(),
                    hp_position: -10.0,
                    row: 0,
                    bypassed: false,
                    dsp_state: None,
                    alias: Some(port_name.clone()),
                    subpatch_path: None,
                });

                sub_cables.push(dirtyrack_gui::rack::SerializableCable {
                    from_stable_id: subpatch_in_sid,
                    from_port: "OUT".to_string(),
                    to_stable_id: cable.to_stable_id,
                    to_port: cable.to_port.clone(),
                    color: cable.color,
                    channels: cable.channels,
                });

                cable.to_stable_id = composite_sid;
                cable.to_port = port_name;
                serial.cables.push(cable);

                subpatch_in_sid += 1;
            }

            // --- Bridge: Outgoing Signals (Subpatch -> Parent) ---
            let mut subpatch_out_sid = subpatch_in_sid + 1000;
            let num_out = outgoing_cables.len();
            for (idx, mut cable) in outgoing_cables.into_iter().enumerate() {
                let port_name = format!("OUT_{}", idx);

                sub_modules.push(dirtyrack_gui::rack::SerializableModule {
                    id: "subpatch_out".to_string(),
                    stable_id: subpatch_out_sid,
                    params: BTreeMap::new(),
                    param_modulations: BTreeMap::new(),
                    hp_position: 100.0,
                    row: 0,
                    bypassed: false,
                    dsp_state: None,
                    alias: Some(port_name.clone()),
                    subpatch_path: None,
                });

                sub_cables.push(dirtyrack_gui::rack::SerializableCable {
                    from_stable_id: cable.from_stable_id,
                    from_port: cable.from_port.clone(),
                    to_stable_id: subpatch_out_sid,
                    to_port: "IN".to_string(),
                    color: cable.color,
                    channels: cable.channels,
                });

                cable.from_stable_id = composite_sid;
                cable.from_port = port_name;
                serial.cables.push(cable);

                subpatch_out_sid += 1;
            }

            let sub_rack = dirtyrack_gui::rack::SerializableRack {
                version: serial.version.clone(),
                engine_hash: serial.engine_hash.clone(),
                modules: sub_modules,
                cables: sub_cables,
                project_seed: serial.project_seed,
                aging: serial.aging,
                cable_opacity: serial.cable_opacity,
                cable_tension: serial.cable_tension,
                causality_log: Vec::new(),
                snapshots: BTreeMap::new(),
                aliases: sub_aliases,
            };

            let sub_json = serde_json::to_string_pretty(&sub_rack)?;
            std::fs::write(format!("{}.json", name), sub_json)?;

            serial
                .modules
                .push(dirtyrack_gui::rack::SerializableModule {
                    id: "composite".to_string(),
                    stable_id: composite_sid,
                    params: BTreeMap::new(),
                    param_modulations: BTreeMap::new(),
                    hp_position: 0.0,
                    row: 0,
                    bypassed: false,
                    dsp_state: None,
                    alias: Some(name.clone()),
                    subpatch_path: Some(format!("{}.json", name)),
                });
            serial.aliases.insert(name.clone(), composite_sid);

            let updated_json = serde_json::to_string_pretty(&serial)?;
            std::fs::write(&patch, updated_json)?;

            println!(
                "{} Bundle complete. Subpatch saved as {}.json",
                "✓".green().bold(),
                name
            );
            println!(
                "   - Boundary bridges created: IN: {}, OUT: {}",
                num_in, num_out
            );
        }
    }

    Ok(())
}
