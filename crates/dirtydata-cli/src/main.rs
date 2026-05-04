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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

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
            let workspace = Workspace::open(".")?;
            if graph {
                let dsl = dirtydata_core::dsl::render_dsl(workspace.graph());
                println!("\n{}", dsl);
            } else {
                for (id, patch) in &workspace.graph().lineage.history {
                    println!("  {} [{}] {}", "•".blue(), id, patch.timestamp.0);
                }
            }
        }
        Commands::Patch { file, intent } => {
            println!("{} Applying patch: {:?}", "🩹".bold(), file);
            let mut workspace = Workspace::open(".")?;
            let content = std::fs::read_to_string(&file)?;
            
            // Try parsing as UserPatchFile (high-level actions)
            if let Ok(patch_file) = serde_json::from_str::<dirtydata_core::actions::UserPatchFile>(&content) {
                if let Err(e) = workspace.apply_user_patch(patch_file) {
                    println!("Failed to apply user patch: {:?}", e);
                    std::process::exit(1);
                }
            } else {
                // Try parsing as raw Patch (low-level operations)
                let patch: dirtydata_core::patch::Patch = serde_json::from_str(&content)?;
                workspace.apply_patch(patch)?;
            }
            
            if let Some(i) = intent {
                println!("   Linked intent: {}", i.blue());
            }
            println!("{} Patch applied successfully.", "✓".green());
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
<<<<<<< HEAD
            println!(
                "{} Mutation generated. Apply it with `dirty patch`.",
                "✓".green()
            );
            println!("   Hash: {}", hex::encode(patch.deterministic_hash));
=======
            
            let patch_name = format!("patch_{}.json", hex::encode(&patch.deterministic_hash[..4]));
            let patch_path = std::env::current_dir()?.join(&patch_name);
            std::fs::write(&patch_path, serde_json::to_string_pretty(&patch)?)?;

            println!("{} Mutation generated and saved to {}.", "✓".green(), patch_name);
            println!("   Apply it with `dirty patch {}`.", patch_name);
>>>>>>> fe9c97d (feat: enhance modular synthesis architecture, add circuit simulation modules, and update GUI/SDK)
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
    }

    Ok(())
}
