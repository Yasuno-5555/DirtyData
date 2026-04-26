use serde::{Deserialize, Serialize};

/// The core trait for DirtyData DSP Plugins.
/// Implement this to create your own custom DSP nodes.
pub trait DspPlugin: Default {
    /// Process one sample of audio.
    /// Returns [left, right] output.
    fn process(&mut self, in_l: f32, in_r: f32) -> [f32; 2];
    
    /// Handle parameter changes from the host.
    fn set_parameter(&mut self, _id: u32, _value: f32) {}
    
    /// Called once when the plugin is loaded.
    fn init(&mut self, _sample_rate: f32) {}
}

#[macro_export]
macro_rules! declare_plugin {
    ($t:ty) => {
        static mut PLUGIN: Option<$t> = None;

        #[no_mangle]
        pub extern "C" fn init(sample_rate: f32) {
            unsafe { 
                let mut p = <$t>::default();
                p.init(sample_rate);
                PLUGIN = Some(p); 
            }
        }

        #[no_mangle]
        pub extern "C" fn process(in_l: f32, in_r: f32) -> i64 {
            let out = unsafe { PLUGIN.as_mut().expect("Plugin not initialized").process(in_l, in_r) };
            // Pack [f32; 2] into i64 for WASM boundary
            unsafe { std::mem::transmute::<[f32; 2], i64>(out) }
        }

        #[no_mangle]
        pub extern "C" fn set_parameter(id: u32, value: f32) {
            unsafe { PLUGIN.as_mut().expect("Plugin not initialized").set_parameter(id, value); }
        }
    };
}
