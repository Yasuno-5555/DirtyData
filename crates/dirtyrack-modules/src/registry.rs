//! Module Registry — Built-in & Third-party Module Catalog

use crate::signal::{
    BuiltinModuleDescriptor, ParamDescriptor, PortDescriptor, ProvenanceZone, RackDspNode,
};
use std::sync::Arc;

/// モジュールのメタ情報（不変データ）
pub struct ModuleDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manufacturer: String,
    pub hp_width: u32,
    pub visuals: crate::signal::ModuleVisuals,
    pub tags: Vec<String>,
    pub ports: Vec<PortDescriptor>,
    pub params: Vec<ParamDescriptor>,
    pub factory: Box<dyn Fn(f32) -> Box<dyn RackDspNode> + Send + Sync>,
    /// 決定論の保証レベル
    pub zone: ProvenanceZone,
}

/// モジュールレジストリ
pub struct ModuleRegistry {
    pub modules: Vec<Arc<ModuleDescriptor>>,
    /// 動的にロードされたライブラリを保持（ドロップ防止）
    #[allow(dead_code)]
    libraries: Vec<libloading::Library>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            modules: Vec::new(),
            libraries: Vec::new(),
        };
        reg.register_builtin();
        reg
    }

    fn register_builtin(&mut self) {
        let builtins: Vec<crate::signal::BuiltinModuleDescriptor> = vec![
            crate::vco::descriptor(),
            crate::vcf::descriptor(),
            crate::vca::descriptor(),
            crate::envelope::descriptor(),
            crate::lfo::descriptor(),
            crate::sequencer::descriptor(),
            crate::mixer::descriptor(),
            crate::clock::descriptor(),
            crate::noise::descriptor(),
            crate::quantizer::descriptor(),
            crate::delay::descriptor(),
            crate::logic::descriptor(),
            crate::sh::descriptor(),
            crate::attenuverter::descriptor(),
            crate::biquad::descriptor(),
            crate::chaos::descriptor(),
            crate::bernoulli::descriptor(),
            crate::midi::descriptor(),
            crate::switch::descriptor(),
            crate::xfade::descriptor(),
            crate::compressor::descriptor(),
            crate::saturation::descriptor(),
            crate::zdf_filter::descriptor(),
            crate::wdf_filter::descriptor(),
            crate::recorder::descriptor(),
            crate::scope::descriptor(),
            crate::reverb::descriptor(),
            crate::macro_ctrl::descriptor(),
            crate::mod_matrix::descriptor(),
            crate::clock_tree::descriptor(),
            crate::drift::descriptor(),
            crate::output::descriptor(),
            crate::scope::descriptor(),
        ];

        for d in builtins {
            let factory_fn = d.factory;
            self.modules.push(Arc::new(ModuleDescriptor {
                id: d.id.to_string(),
                name: d.name.to_string(),
                version: d.version.to_string(),
                manufacturer: d.manufacturer.to_string(),
                hp_width: d.hp_width,
                visuals: d.visuals,
                tags: d.tags.iter().map(|s| s.to_string()).collect(),
                ports: d.ports.to_vec(),
                params: d.params.to_vec(),
                factory: Box::new(move |sr| factory_fn(sr)),
                zone: ProvenanceZone::Safe,
            }));
        }
    }

    pub fn find(&self, id: &str) -> Option<Arc<ModuleDescriptor>> {
        self.modules.iter().find(|m| m.id == id).cloned()
    }

    pub fn filter_by_tag(&self, tag: &str) -> Vec<Arc<ModuleDescriptor>> {
        self.modules
            .iter()
            .filter(|m| m.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    pub fn all(&self) -> Vec<Arc<ModuleDescriptor>> {
        self.modules.clone()
    }

    pub fn search(&self, query: &str) -> Vec<Arc<ModuleDescriptor>> {
        let q = query.to_lowercase();
        self.modules
            .iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&q)
                    || m.id.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .cloned()
            .collect()
    }

    pub unsafe fn load_plugin(&mut self, path: &std::path::Path) -> Result<(), String> {
        let lib = libloading::Library::new(path).map_err(|e| e.to_string())?;
        self.libraries.push(lib);
        Ok(())
    }
}
