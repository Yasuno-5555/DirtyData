//! GUI Commander — コマンドラインベースのラック操作
//!
//! CLI のロジックを GUI の RackState にブリッジ。
//! 「Summoner (召喚)」バーからの入力を処理する。

use crate::rack::{Cable, RackState};
use dirtyrack_modules::registry::ModuleRegistry;
use egui::{Color32, Vec2};
use std::sync::Arc;

pub struct Commander {
    pub last_result: Option<Result<String, String>>,
    pub input_buffer: String,
}

impl Commander {
    pub fn new() -> Self {
        Self {
            last_result: None,
            input_buffer: String::new(),
        }
    }

    /// 文字列コマンドを実行し、RackState を直接操作するかアクションを返す
    pub fn execute(
        &mut self,
        input: &str,
        rack: &mut RackState,
        registry: &ModuleRegistry,
        mouse_pos_world: Vec2,
    ) {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let cmd = parts[0];
        let result = match cmd {
            "add" => self.handle_add(&parts[1..], rack, registry, mouse_pos_world),
            "multiply" | "mul" => {
                self.handle_multiply(&parts[1..], rack, registry, mouse_pos_world)
            }
            "connect" | "conn" => self.handle_connect(&parts[1..], rack),
            "set" => self.handle_set(&parts[1..], rack),
            "alias" => self.handle_alias(&parts[1..], rack),
            "rm" | "remove" => self.handle_remove(&parts[1..], rack),
            "clear" => {
                rack.modules.clear();
                rack.cables.clear();
                Ok("Rack cleared".to_string())
            }
            _ => Err(format!("Unknown command: {}", cmd)),
        };

        self.last_result = Some(result);
    }

    fn resolve_module_index(&self, rack: &RackState, id_or_alias: &str) -> Option<usize> {
        if let Ok(sid) = id_or_alias.parse::<u64>() {
            rack.modules.iter().position(|m| m.stable_id == sid)
        } else {
            rack.modules
                .iter()
                .position(|m| m.alias.as_deref() == Some(id_or_alias))
        }
    }

    fn handle_add(
        &self,
        args: &[&str],
        rack: &mut RackState,
        registry: &ModuleRegistry,
        mouse_pos: Vec2,
    ) -> Result<String, String> {
        if args.is_empty() {
            return Err("Usage: add <module_id>".to_string());
        }
        let id = args[0];
        if let Some(desc) = registry.find(id) {
            let desc_arc = Arc::new(desc);
            rack.add_module_at(Arc::clone(&desc_arc), mouse_pos.to_pos2());
            let sid = rack.modules.last().map(|m| m.stable_id).unwrap_or(0);
            Ok(format!("Added {} (Stable ID: {})", desc_arc.name, sid))
        } else {
            Err(format!("Module not found: {}", id))
        }
    }

    fn handle_connect(&self, args: &[&str], rack: &mut RackState) -> Result<String, String> {
        if args.len() < 4 {
            return Err("Usage: connect <from> <port> <to> <port>".to_string());
        }
        let from_idx = self
            .resolve_module_index(rack, args[0])
            .ok_or_else(|| format!("Source not found: {}", args[0]))?;
        let to_idx = self
            .resolve_module_index(rack, args[2])
            .ok_or_else(|| format!("Target not found: {}", args[2]))?;

        let from_port = args[1];
        let to_port = args[3];

        // Validate ports
        let has_from = rack.modules[from_idx]
            .descriptor
            .ports
            .iter()
            .any(|p| p.name == from_port);
        let has_to = rack.modules[to_idx]
            .descriptor
            .ports
            .iter()
            .any(|p| p.name == to_port);

        if !has_from {
            return Err(format!("Port not found on source: {}", from_port));
        }
        if !has_to {
            return Err(format!("Port not found on target: {}", to_port));
        }

        rack.cables.push(Cable {
            from_module: from_idx,
            from_port: from_port.to_string(),
            to_module: to_idx,
            to_port: to_port.to_string(),
            color: Color32::from_rgb(200, 200, 200),
            channels: 1,
        });

        Ok(format!(
            "Connected {}:{} -> {}:{}",
            args[0], from_port, args[2], to_port
        ))
    }

    fn handle_set(&self, args: &[&str], rack: &mut RackState) -> Result<String, String> {
        if args.len() < 3 {
            return Err("Usage: set <id> <param> <val>".to_string());
        }
        let idx = self
            .resolve_module_index(rack, args[0])
            .ok_or_else(|| format!("Module not found: {}", args[0]))?;
        let param_name = args[1];
        let val = args[2]
            .parse::<f32>()
            .map_err(|_| "Invalid value".to_string())?;

        if let Some(m) = rack.modules.get_mut(idx) {
            m.params.insert(param_name.to_string(), val);
            Ok(format!("Set {} : {} -> {:.3}", args[0], param_name, val))
        } else {
            Err("Module not found".to_string())
        }
    }

    fn handle_alias(&self, args: &[&str], rack: &mut RackState) -> Result<String, String> {
        if args.len() < 2 {
            return Err("Usage: alias <id> <name>".to_string());
        }
        let idx = self
            .resolve_module_index(rack, args[0])
            .ok_or_else(|| format!("Module not found: {}", args[0]))?;
        let alias = args[1];

        if let Some(m) = rack.modules.get_mut(idx) {
            m.alias = Some(alias.to_string());
            rack.aliases.insert(alias.to_string(), m.stable_id);
            Ok(format!("Aliased {} as {}", args[0], alias))
        } else {
            Err("Module not found".to_string())
        }
    }

    fn handle_remove(&self, args: &[&str], rack: &mut RackState) -> Result<String, String> {
        if args.is_empty() {
            return Err("Usage: rm <id>".to_string());
        }
        let idx = self
            .resolve_module_index(rack, args[0])
            .ok_or_else(|| format!("Module not found: {}", args[0]))?;
        rack.remove_module(idx);
        Ok(format!("Removed module {}", args[0]))
    }

    fn handle_multiply(
        &self,
        args: &[&str],
        rack: &mut RackState,
        registry: &ModuleRegistry,
        mouse_pos: Vec2,
    ) -> Result<String, String> {
        if args.len() < 2 {
            return Err("Usage: multiply <count> <module_id>".to_string());
        }
        let count = args[0]
            .parse::<usize>()
            .map_err(|_| "Invalid count".to_string())?;
        let id = args[1];

        if let Some(desc) = registry.find(id) {
            let desc_arc = Arc::new(desc);
            let mut world_pos = mouse_pos;
            for _ in 0..count {
                rack.add_module_at(Arc::clone(&desc_arc), world_pos.to_pos2());
                world_pos.x += desc_arc.hp_width as f32 * crate::rack::HP_PIXELS;
            }
            Ok(format!("Multiplied {} x {}", count, id))
        } else {
            Err(format!("Module not found: {}", id))
        }
    }
}
