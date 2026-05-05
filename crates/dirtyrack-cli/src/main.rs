#![allow(clippy::all, unused, dead_code)]

use clap::{Parser, Subcommand};
use colored::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::BTreeMap;
use std::path::PathBuf;

mod inner_execute;
use inner_execute::inner_execute;

#[derive(Parser)]
#[command(name = "dirtyrack")]
#[command(about = "DirtyRack Forensic Eurorack Simulator CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
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

    /// Bundle a selection of modules into a subpatch
    Bundle {
        /// Path to the parent patch file
        patch: PathBuf,
        /// List of module IDs or aliases to include
        #[arg(required = true)]
        ids: Vec<String>,
        /// Name of the subpatch file
        #[arg(short, long)]
        name: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    inner_execute(cli.command)
}
