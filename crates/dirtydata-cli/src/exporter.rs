use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use dirtydata_core::actions::UserPatchFile;
use ulid::Ulid;
use colored::Colorize;

pub fn export_clap(patch_path: PathBuf, output_dir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let patch_content = fs::read_to_string(&patch_path)?;
    let patch_file: UserPatchFile = serde_json::from_str(&patch_content)?;
    let patch_json = serde_json::to_string(&patch_file)?;

    let project_name = patch_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dirtydata_plugin");
    let safe_name = project_name.replace("-", "_");

    let temp_dir = std::env::temp_dir().join(format!("dirtydata_export_{}", Ulid::new()));
    fs::create_dir_all(temp_dir.join("src"))?;

    let workspace_root = std::env::current_dir()?;
    let runtime_path = workspace_root.join("crates/dirtydata-runtime").canonicalize()?;
    let core_path = workspace_root.join("crates/dirtydata-core").canonicalize()?;

    // 1. Generate Cargo.toml
    let cargo_toml = format!(
r#"[package]
name = "{safe_name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nih_plug = {{ git = "https://github.com/robbert-vdh/nih-plug.git", features = ["assert_process_allocs"] }}
dirtydata-runtime = {{ path = {runtime_path:?} }}
dirtydata-core = {{ path = {core_path:?} }}
serde_json = "1"
"#
    );
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

    let template = r###"use nih_plug::prelude::*;
use dirtydata_runtime::{DspRunner, nodes::ProcessContext};
use dirtydata_core::ir::Graph;
use std::sync::Arc;

struct DirtyDataPlugin {
    params: Arc<DirtyDataParams>,
    runner: Option<DspRunner>,
    sample_rate: f32,
    global_sample_index: u64,
}

#[derive(Params)]
struct DirtyDataParams {}

impl Default for DirtyDataPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(DirtyDataParams {}),
            runner: None,
            sample_rate: 44100.0,
            global_sample_index: 0,
        }
    }
}

impl Plugin for DirtyDataPlugin {
    const NAME: &'static str = "PROJECT_NAME";
    const VENDOR: &'static str = "DirtyData";
    const URL: &'static str = "https://github.com/yasuno/DirtyData";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = "0.1.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(&mut self, _audio_io_layout: &AudioIOLayout, buffer_config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        
        let patch_json = r##"PATCH_JSON"##;
        if let Ok(graph) = serde_json::from_str::<Graph>(patch_json) {
            self.runner = Some(DspRunner::new(graph, None, self.sample_rate));
            true
        } else {
            false
        }
    }

    fn process(&mut self, buffer: &mut Buffer, _context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        if let Some(runner) = &mut self.runner {
            for mut channel_samples in buffer.iter_samples() {
                let mut inputs = [0.0f32; 2];
                for i in 0..channel_samples.len() {
                    if i < 2 { inputs[i] = channel_samples.get(i); }
                }

                let ctx = dirtydata_runtime::nodes::ProcessContext {
                    sample_rate: self.sample_rate,
                    global_sample_index: self.global_sample_index,
                    crash_flag: None,
                };

                let out = runner.process_sample(&ctx);
                
                for i in 0..channel_samples.len() {
                    if i < 2 {
                        if let Some(sample) = channel_samples.get_mut(i) {
                            *sample = out[i];
                        }
                    }
                }
                self.global_sample_index += 1;
            }
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for DirtyDataPlugin {
    const CLAP_ID: &'static str = "com.dirtydata.SAFE_NAME";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Exported DirtyData Patch");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

nih_export_clap!(DirtyDataPlugin);
"###;

    let lib_rs = template
        .replace("PROJECT_NAME", project_name)
        .replace("SAFE_NAME", &safe_name)
        .replace("PATCH_JSON", &patch_json);
        
    fs::write(temp_dir.join("src/lib.rs"), lib_rs)?;

    // 3. Build
    println!("{} Scaffolding nih-plug project...", "⚒".blue());
    println!("{} Building CLAP plugin (this may take a while)...", "⚒".blue());

    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&temp_dir)
        .status()?;

    if !status.success() {
        return Err("Cargo build failed".into());
    }

    // 4. Move artifact
    let artifact_name = if cfg!(target_os = "macos") {
        format!("lib{}.dylib", safe_name)
    } else if cfg!(target_os = "windows") {
        format!("{}.dll", safe_name)
    } else {
        format!("lib{}.so", safe_name)
    };

    let build_path = temp_dir.join("target/release").join(artifact_name);
    let final_dir = output_dir.unwrap_or_else(|| workspace_root.clone());
    let final_name = format!("{}.clap", project_name);
    let final_path = final_dir.join(final_name);

    fs::copy(build_path, &final_path)?;
    
    println!("{} Export complete: {}", "✓".green().bold(), final_path.display());

    Ok(())
}
