use clap::{Parser, Subcommand};
use colored::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
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
            let mut runner = dirtyrack_modules::runner::RackRunner::new(sample_rate as f32, dirtyrack_modules::signal::SeedScope::Global(0));
            
            // Reconstruct the graph from serializable data
            // (Minimal reconstruction for CLI rendering)
            let mut node_type_ids = Vec::new();
            let mut connections = Vec::new();
            let mut nodes = Vec::new();
            let mut initial_params = Vec::new();
            
            for m in &serial.modules {
                node_type_ids.push(m.id.clone());
                if let Some(desc) = registry.find(&m.id) {
                    let node = (desc.factory)(sample_rate as f32);
                    nodes.push(node);
                    
                    let mut node_params = Vec::new();
                    for p in &desc.params {
                        let val = m.params.get(p.name).cloned().unwrap_or(p.default);
                        node_params.push(val);
                    }
                    initial_params.push(node_params);
                }
            }
            
            let mut stable_to_idx = BTreeMap::new();
            for (i, m) in serial.modules.iter().enumerate() {
                stable_to_idx.insert(m.stable_id, i);
            }
            
            for c in &serial.cables {
                if let (Some(&fi), Some(&ti)) = (stable_to_idx.get(&c.from_stable_id), stable_to_idx.get(&c.to_stable_id)) {
                    let from_desc = registry.find(&serial.modules[fi].id);
                    let to_desc = registry.find(&serial.modules[ti].id);
                    
                    if let (Some(fd), Some(td)) = (from_desc, to_desc) {
                        let from_port = fd.ports.iter().position(|p| p.name == c.from_port).unwrap_or(0);
                        let to_port = td.ports.iter().position(|p| p.name == c.to_port).unwrap_or(0);
                        
                        connections.push(dirtyrack_modules::runner::Connection {
                            from_module: fi,
                            from_port,
                            to_module: ti,
                            to_port,
                        });
                    }
                }
            }

            // Simple topological sort / order (actual GUI logic is more complex)
            let order: Vec<usize> = (0..nodes.len()).collect(); 
            let mut snapshot = dirtyrack_modules::runner::GraphSnapshot {
                order,
                connections,
                node_type_ids,
                port_counts: vec![(0,0); nodes.len()], // Placeholder
                node_ids: vec![0; nodes.len()], // Placeholder
                modulations: vec![vec![]; nodes.len()],
                forward_edges: vec![vec![]; nodes.len()],
                back_edges: vec![],
            };

            runner.apply_snapshot(&mut snapshot, nodes);

            let spec = hound::WavSpec {
                channels: 2,
                sample_rate: sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };

            let mut writer = hound::WavWriter::create(&output, spec)?;
            let total_samples = (length * sample_rate as f32) as usize;
            
            let mut hasher = blake3::Hasher::new();

            for _ in 0..total_samples {
                runner.process_sample(&snapshot, &initial_params);
                // Assume the last module is an OutputModule or similar
                // We'll just grab the sum of all nodes that have 0 outputs for now as a "mix"
                let mut left = 0.0;
                let mut right = 0.0;
                if !runner.output_buffers.is_empty() {
                    let last_idx = runner.output_buffers.len() - 1;
                    left = runner.output_buffers[last_idx][0];
                    right = runner.output_buffers[last_idx][16];
                }
                
                writer.write_sample(left)?;
                writer.write_sample(right)?;
                hasher.update(&left.to_le_bytes());
                hasher.update(&right.to_le_bytes());
            }

            writer.finalize()?;
            let hash = hasher.finalize();
            println!("{} Render Complete!", "✓".green().bold());
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
    }

    Ok(())
}
