use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use dirty_exporter::{BuildTarget, Transmuter};
use dirty_mutate::Mutator;
use dirtydata_host::Workspace;
use std::path::{Path, PathBuf};

/// DirtyData: Headless Forensic Audio Workbench
#[derive(Parser)]
#[command(name = "dirty")]
#[command(about = "Headless Forensic Audio Workbench", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new DirtyData project
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Audit the forensic integrity of the project
    Doctor,
    /// View the semantic lineage and intent chain
    Log {
        #[arg(long)]
        graph: bool,
    },
    /// Apply a patch to the forensic record
    Patch {
        file: PathBuf,
        #[arg(long)]
        intent: Option<String>,
    },
    /// Headless batch mutation (Evolutionary Search)
    Mutate {
        node_id: String,
        #[arg(long, default_value = "wild")]
        level: String,
        #[arg(long, default_value_t = 100)]
        epochs: usize,
    },
    /// Transmute IR to a standalone plugin or binary
    Build {
        #[arg(long, default_value = "vst3")]
        target: String,
        #[arg(long)]
        release: bool,
    },
    /// Verify forensic spec adherence
    Verify,
    /// Freeze a node's output to a deterministic asset
    Freeze {
        node_id: String,
        #[arg(long, default_value_t = 10.0)]
        length: f32,
    },
    /// Manage nodes in the graph
    Node {
        #[command(subcommand)]
        subcommand: NodeCommands,
    },
    /// Manage edges (connections) in the graph
    Edge {
        #[command(subcommand)]
        subcommand: EdgeCommands,
    },
    /// Set a configuration parameter on a node
    Set {
        node_id: String,
        key: String,
        value: String,
    },
    /// Record a new human intent
    Intent { description: String },
}

#[derive(Subcommand)]
enum NodeCommands {
    /// Add a new node
    Add {
        kind: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// Remove a node
    Rm { id: String },
    /// List all nodes
    Ls,
}

#[derive(Subcommand)]
enum EdgeCommands {
    /// Connect two ports
    Add {
        src_node: String,
        src_port: String,
        dst_node: String,
        dst_port: String,
    },
    /// Remove a connection
    Rm { id: String },
    /// List all connections
    Ls,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            println!("{} Initializing forensic record at {:?}", "🏛".bold(), path);
            let workspace = Workspace::open(&path)?;
            workspace.save()?;
            println!("{} Project initialized successfully.", "✓".green());
        }
        Commands::Doctor => {
            println!("{} Auditing forensic integrity...", "🩺".bold());
            let workspace = Workspace::open(".")?;
            let report = workspace.audit()?;

            println!("\n{}", "--- Forensic Audit Report ---".dimmed());

            let status = if report.is_healthy() {
                "HEALTHY".green()
            } else {
                "CORRUPTED".red()
            };
            println!("Status: {}", status);

            let check = |label: &str, valid: bool| {
                let icon = if valid { "✓".green() } else { "✗".red() };
                println!(
                    "  {} {:<20} [{}]",
                    icon,
                    label,
                    if valid { "OK".green() } else { "FAIL".red() }
                );
            };

            check("Root Hash", report.root_hash_valid);
            check("Manifest Signature", report.manifest_valid);
            check("Lineage Integrity", report.lineage_intact);
            check("CAS Completeness", report.cas_complete);
            check("Determinism", report.determinism_verified);

            if !report.issues.is_empty() {
                println!("\n{}", "Issues Found:".yellow().bold());
                for issue in &report.issues {
                    println!("  {} {}", "!".yellow(), issue);
                }
            }

            println!("{}\n", "-----------------------------".dimmed());

            if report.is_healthy() {
                println!(
                    "{} All forensic checks passed. You can trust this workspace.",
                    "🛡️".green()
                );
            } else {
                println!(
                    "{} Workspace integrity compromised. Repair required.",
                    "⚠️".red()
                );
                std::process::exit(1);
            }
        }
        Commands::Log { graph } => {
            println!("{} Displaying Merkle DAG lineage...", "📜".bold());
            if graph {
                println!("(ASCII Graph Visualization would go here)");
            }
        }
        Commands::Patch { file, intent } => {
            println!("{} Applying patch: {:?}", "🩹".bold(), file);
            if let Some(i) = intent {
                println!("   Linked intent: {}", i.blue());
            }
        }
        Commands::Mutate {
            node_id,
            level,
            epochs,
        } => {
            println!(
                "{} Mutating node {} (level: {}, epochs: {})",
                "🧬".bold(),
                node_id,
                level,
                epochs
            );
            let workspace = Workspace::open(".")?;
            let mutation_level = match level.as_str() {
                "safe" => 0.01,
                "wild" => 0.1,
                "radioactive" => 0.5,
                _ => 0.1,
            };
            let patch = Mutator::evolve(workspace.graph(), &node_id, epochs, mutation_level)?;
            println!(
                "{} Mutation generated. Apply it with `dirty patch`.",
                "✓".green()
            );
            println!("   Hash: {}", hex::encode(patch.deterministic_hash));
        }
        Commands::Build { target, release } => {
            let mode = if release { "release" } else { "debug" };
            println!("{} Building target {} in {} mode", "🏗".bold(), target, mode);
            let workspace = Workspace::open(".")?;
            let target_enum = match target.as_str() {
                "vst3" => BuildTarget::Vst3,
                "clap" => BuildTarget::Clap,
                "standalone" => BuildTarget::Standalone,
                _ => BuildTarget::Vst3,
            };
            let output = Transmuter::transmute(workspace.graph(), target_enum, Path::new("build"))?;
            println!(
                "{} Transmutation complete. Project generated at: {:?}",
                "✓".green(),
                output
            );
        }
        Commands::Verify => {
            println!("{} Verifying forensic spec compliance...", "🔎".bold());
        }
        Commands::Freeze { node_id, length } => {
            println!("{} Freezing node {} ({}s)", "❄️".bold(), node_id, length);
        }
        Commands::Node { subcommand } => {
            let mut workspace = Workspace::open(".")?;
            match subcommand {
                NodeCommands::Add { kind, name } => {
                    let name = name.unwrap_or_else(|| kind.clone());
                    let node = match kind.as_str() {
                        "source" => dirtydata_core::ir::Node::new_source(&name),
                        "sink" => dirtydata_core::ir::Node::new_sink(&name),
                        "processor" => dirtydata_core::ir::Node::new_processor(&name),
                        "subgraph" => dirtydata_core::ir::Node::new_subgraph(&name),
                        _ => {
                            println!("{} Unknown node kind: {}", "✗".red(), kind);
                            return Ok(());
                        }
                    };
                    let id = node.id;
                    let patch = dirtydata_core::patch::Patch::from_operations(vec![
                        dirtydata_core::patch::Operation::AddNode(node),
                    ]);
                    workspace.apply_patch(patch)?;
                    println!(
                        "{} Added {} node: {} ({})",
                        "✓".green(),
                        kind,
                        name,
                        id.to_string().cyan()
                    );
                }
                NodeCommands::Rm { id } => {
                    let stable_id: dirtydata_core::types::StableId = id
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid ID format"))?;
                    let patch = dirtydata_core::patch::Patch::from_operations(vec![
                        dirtydata_core::patch::Operation::RemoveNode(stable_id),
                    ]);
                    workspace.apply_patch(patch)?;
                    println!("{} Removed node: {}", "✓".green(), id.cyan());
                }
                NodeCommands::Ls => {
                    println!("\n{} {:<26} {:<12} {}", " ".dimmed(), "ID", "KIND", "NAME");
                    for (id, node) in &workspace.graph().topology.nodes {
                        let kind_str = format!("{:?}", node.kind);
                        let name = node
                            .config
                            .get("name")
                            .and_then(|v| v.as_string())
                            .cloned()
                            .unwrap_or_default();
                        println!(
                            "  {} {:<26} {:<12} {}",
                            "•".blue(),
                            id.to_string().cyan(),
                            kind_str.yellow(),
                            name
                        );
                    }
                    println!();
                }
            }
        }
        Commands::Edge { subcommand } => {
            let mut workspace = Workspace::open(".")?;
            match subcommand {
                EdgeCommands::Add {
                    src_node,
                    src_port,
                    dst_node,
                    dst_port,
                } => {
                    let src_id: dirtydata_core::types::StableId = src_node
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid source ID"))?;
                    let dst_id: dirtydata_core::types::StableId = dst_node
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid destination ID"))?;

                    let edge = dirtydata_core::ir::Edge::new(
                        dirtydata_core::types::PortRef {
                            node_id: src_id,
                            port_name: src_port.clone(),
                        },
                        dirtydata_core::types::PortRef {
                            node_id: dst_id,
                            port_name: dst_port.clone(),
                        },
                    );
                    let id = edge.id;
                    let patch = dirtydata_core::patch::Patch::from_operations(vec![
                        dirtydata_core::patch::Operation::AddEdge(edge),
                    ]);
                    workspace.apply_patch(patch)?;
                    println!(
                        "{} Connected {}:{} -> {}:{} ({})",
                        "✓".green(),
                        src_node.cyan(),
                        src_port,
                        dst_node.cyan(),
                        dst_port,
                        id.to_string().dimmed()
                    );
                }
                EdgeCommands::Rm { id } => {
                    let stable_id: dirtydata_core::types::StableId = id
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid ID format"))?;
                    let patch = dirtydata_core::patch::Patch::from_operations(vec![
                        dirtydata_core::patch::Operation::RemoveEdge(stable_id),
                    ]);
                    workspace.apply_patch(patch)?;
                    println!("{} Removed edge: {}", "✓".green(), id.cyan());
                }
                EdgeCommands::Ls => {
                    println!(
                        "\n{} {:<26} {} -> {}",
                        " ".dimmed(),
                        "ID",
                        "SOURCE",
                        "TARGET"
                    );
                    for (id, edge) in &workspace.graph().topology.edges {
                        println!(
                            "  {} {:<26} {}:{} -> {}:{}",
                            "→".blue(),
                            id.to_string().dimmed(),
                            edge.source.node_id.to_string().cyan(),
                            edge.source.port_name,
                            edge.target.node_id.to_string().cyan(),
                            edge.target.port_name
                        );
                    }
                    println!();
                }
            }
        }
        Commands::Set {
            node_id,
            key,
            value,
        } => {
            let mut workspace = Workspace::open(".")?;
            let stable_id: dirtydata_core::types::StableId = node_id
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid ID format"))?;

            let mut delta = std::collections::BTreeMap::new();
            delta.insert(
                key.clone(),
                dirtydata_core::types::ConfigChange {
                    old: None,
                    new: Some(parse_config_value(&value)),
                },
            );

            let patch = dirtydata_core::patch::Patch::from_operations(vec![
                dirtydata_core::patch::Operation::ModifyConfig {
                    node_id: stable_id,
                    delta,
                },
            ]);
            workspace.apply_patch(patch)?;
            println!(
                "{} Set {} = {} for node {}",
                "✓".green(),
                key.yellow(),
                value.magenta(),
                node_id.cyan()
            );
        }
        Commands::Intent { description } => {
            let mut workspace = Workspace::open(".")?;
            let id = workspace
                .intent_state_mut()
                .add(description.clone(), vec![]);
            workspace.save()?;
            println!(
                "{} Recorded intent: {} ({})",
                "💡".yellow(),
                description,
                id.to_string().dimmed()
            );
        }
    }

    Ok(())
}

fn parse_config_value(s: &str) -> dirtydata_core::types::ConfigValue {
    if let Ok(f) = s.parse::<f64>() {
        dirtydata_core::types::ConfigValue::Float(f)
    } else if let Ok(i) = s.parse::<i64>() {
        dirtydata_core::types::ConfigValue::Int(i)
    } else if let Ok(b) = s.parse::<bool>() {
        dirtydata_core::types::ConfigValue::Bool(b)
    } else {
        dirtydata_core::types::ConfigValue::String(s.to_string())
    }
}
