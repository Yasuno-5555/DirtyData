
use nih_plug::prelude::*;
use dirtydata_core::ir::Graph;
use dirtydata_runtime::AudioEngine;

struct DirtyPlugin {
    params: Arc<DirtyParams>,
    engine: Option<AudioEngine>,
}

#[derive(Params)]
struct DirtyParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for DirtyPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(DirtyParams {
                gain: FloatParam::new("Gain", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 }),
            }),
            engine: None,
        }
    }
}

impl Plugin for DirtyPlugin {
    const NAME: &'static str = "DirtyData Transmuted";
    const VENDOR: &'static str = "DirtyData Forensic";
    const URL: &'static str = "https://github.com/Yasuno-5555/DirtyData";
    const EMAIL: &'static str = "forensic@example.com";
    const VERSION: &'static str = "0.1.0";

    fn process(&mut self, buffer: &mut Buffer, _context: &mut ProcessContext) -> ProcessStatus {
        // Execute the JIT engine here...
        ProcessStatus::Normal
    }
}
