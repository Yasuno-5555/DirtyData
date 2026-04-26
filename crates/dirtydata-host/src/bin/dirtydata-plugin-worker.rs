<<<<<<< Updated upstream
use libloading::{Library, Symbol};
use std::io::{Read, Write};
=======
use std::io::{Read, Write};
use libloading::{Library, Symbol};
>>>>>>> Stashed changes
use std::path::{Path, PathBuf};

// VST3 Factory entry point signature
type GetPluginFactory = unsafe extern "system" fn() -> *mut std::ffi::c_void;

struct Vst3Host {
    _lib: Library,
    factory: *mut std::ffi::c_void, // Placeholder for IPluginFactory
}

impl Vst3Host {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
<<<<<<< Updated upstream

=======
        
>>>>>>> Stashed changes
        // Resolve actual dynamic library path inside the .vst3 bundle
        let dylib_path = if path.is_dir() {
            #[cfg(target_os = "macos")]
            let mut p = path.join("Contents/MacOS");
            #[cfg(target_os = "windows")]
            let mut p = path.join("Contents/x86_64-win");
            #[cfg(target_os = "linux")]
            let mut p = path.join("Contents/x86_64-linux");
<<<<<<< Updated upstream

=======
            
>>>>>>> Stashed changes
            // Just guess the filename based on the bundle name
            let bundle_name = path.file_stem().unwrap().to_str().unwrap();
            #[cfg(target_os = "macos")]
            p.push(bundle_name);
            #[cfg(target_os = "windows")]
            p.push(format!("{}.vst3", bundle_name));
            #[cfg(target_os = "linux")]
            p.push(format!("{}.so", bundle_name));
<<<<<<< Updated upstream

=======
            
>>>>>>> Stashed changes
            p
        } else {
            path.to_path_buf()
        };

        if !dylib_path.exists() {
            return Err(format!("Dylib not found at {:?}", dylib_path));
        }

        unsafe {
            let lib = Library::new(&dylib_path).map_err(|e| e.to_string())?;
<<<<<<< Updated upstream

            // VST3 standard entry point
            let get_factory: Symbol<GetPluginFactory> =
                lib.get(b"GetPluginFactory\0").map_err(|e| e.to_string())?;
            let factory = get_factory();

=======
            
            // VST3 standard entry point
            let get_factory: Symbol<GetPluginFactory> = lib.get(b"GetPluginFactory\0").map_err(|e| e.to_string())?;
            let factory = get_factory();
            
>>>>>>> Stashed changes
            if factory.is_null() {
                return Err("GetPluginFactory returned null".to_string());
            }

<<<<<<< Updated upstream
            Ok(Self { _lib: lib, factory })
=======
            Ok(Self {
                _lib: lib,
                factory,
            })
>>>>>>> Stashed changes
        }
    }

    pub fn process(&mut self, in_buf: &[f32], out_buf: &mut [f32]) {
        // TODO: Full VST3 COM IAudioProcessor interaction
        // For now, passthrough as the host architecture is being laid down
        out_buf.copy_from_slice(in_buf);
    }
<<<<<<< Updated upstream

=======
    
>>>>>>> Stashed changes
    pub fn set_parameter(&mut self, _id: u32, _value: f32) {
        // TODO: IEditController interaction
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let plugin_path = args.get(1).map(|s| s.as_str()).unwrap_or("");

    let mut host = match Vst3Host::load(plugin_path) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to load VST3 plugin: {}", e);
            std::process::exit(1);
        }
    };

    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    loop {
        let mut cmd_buf = [0u8; 1];
        if stdin.read_exact(&mut cmd_buf).is_err() {
            break;
        }

        match cmd_buf[0] {
<<<<<<< Updated upstream
            0 => {
                // Process
=======
            0 => { // Process
>>>>>>> Stashed changes
                let mut size_buf = [0u8; 4];
                stdin.read_exact(&mut size_buf).unwrap();
                let size = u32::from_le_bytes(size_buf) as usize;

                let mut in_bytes = vec![0u8; size * 4];
                stdin.read_exact(&mut in_bytes).unwrap();

                let mut float_buf = vec![0.0f32; size];
                bytemuck::cast_slice_mut(&mut float_buf).copy_from_slice(&in_bytes);

                // Call actual VST3 processing (passthrough for now)
                let mut out_buf = vec![0.0f32; size];
                host.process(&float_buf, &mut out_buf);

                let out_bytes = bytemuck::cast_slice(&out_buf);
                stdout.write_all(out_bytes).unwrap();
                stdout.flush().unwrap();
            }
<<<<<<< Updated upstream
            1 => {
                // SetParameter
=======
            1 => { // SetParameter
>>>>>>> Stashed changes
                let mut id_buf = [0u8; 4];
                let mut val_buf = [0u8; 4];
                stdin.read_exact(&mut id_buf).unwrap();
                stdin.read_exact(&mut val_buf).unwrap();
<<<<<<< Updated upstream

=======
                
>>>>>>> Stashed changes
                let param_id = u32::from_le_bytes(id_buf);
                let value = f32::from_le_bytes(val_buf);
                host.set_parameter(param_id, value);
            }
            _ => {}
        }
    }
}
