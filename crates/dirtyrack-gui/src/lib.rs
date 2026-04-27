//! DirtyRack GUI — Eurorack Simulator Interface
//!
//! ラックレール描画、フェースプレート、パッチケーブル物理、
//! モジュールブラウザ、リアルタイムオーディオエンジン統合。

pub mod browser;
pub mod cable;
pub mod engine;
pub mod exporter;
pub mod faceplate;
pub mod rack;
pub mod visual_data;

use crate::rack::{CableAction, RackState, IntentBoundary, IntentClass};
use dirtyrack_modules::registry::ModuleRegistry;
use engine::RackAudioEngine;
use egui::{Color32, Rect, Stroke};

pub fn run() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 820.0])
            .with_title("DirtyRack — Eurorack Simulator"),
        ..Default::default()
    };

    eframe::run_native(
        "dirtyrack",
        native_options,
        Box::new(|cc| {
            // Dark theme
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            cc.egui_ctx.set_style(style);

            Ok(Box::new(DirtyRackApp::new(cc)))
        }),
    )
}

pub struct DirtyRackApp {
    registry: ModuleRegistry,
    rack: RackState,
    engine: Option<RackAudioEngine>,
    visual_reader: Option<triple_buffer::Output<visual_data::VisualSnapshot>>,
    browser_open: bool,
    browser_search: String,
    pan: egui::Vec2,
    zoom: f32,
    show_provenance_timeline: bool,
    mri_mode: bool,
    #[allow(dead_code)]
    explain_result: Option<String>,
    selected_module_forensic: Option<u64>, // StableId of module being inspected
    status_msg: Option<(String, bool)>, // (message, is_error)
    show_diff_audit: bool,
    diagnosis_report: Option<String>,
    #[allow(dead_code)]
    parallel_mode: bool,
    inspector_open: bool,
}

impl DirtyRackApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let registry = ModuleRegistry::new();
        let mut rack = RackState::new();
        rack.project_seed = 0xDE7E_B11D;

        let (engine, visual_reader) = match RackAudioEngine::new(rack.sample_rate) {
            Ok((e, v)) => (Some(e), Some(v)),
            Err(_) => (None, None),
        };

        Self {
            registry,
            rack,
            engine,
            visual_reader,
            browser_open: false,
            browser_search: String::new(),
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            selected_module_forensic: None,
            status_msg: None,
            show_diff_audit: false,
            show_provenance_timeline: false,
            mri_mode: false,
            explain_result: None,
            diagnosis_report: None,
            parallel_mode: false,
            inspector_open: false,
        }
    }

    fn run_verification(&mut self) {
        use dirtyrack_modules::renderer::OfflineRenderer;
        use dirtyrack_modules::signal::SeedScope;

        // Try to find an audit file to compare against
        if let Ok(paths) = std::fs::read_dir(".") {
            let audit_file = paths
                .filter_map(|e| e.ok())
                .find(|e| e.file_name().to_string_lossy().ends_with(".audit.json"));

            if let Some(path) = audit_file {
                if let Ok(json) = std::fs::read_to_string(path.path()) {
                    if let Ok(audit_data) = serde_json::from_str::<serde_json::Value>(&json) {
                        let expected_hash = audit_data["blake3_hash"].as_str().unwrap_or("");
                        let sample_count = audit_data["sample_count"].as_u64().unwrap_or(44100) as usize;

                        // Rebuild for verification
                        let (snapshot, nodes, params) = self.rack.build_snapshot();
                        let mut renderer = OfflineRenderer::new(
                            self.rack.sample_rate,
                            SeedScope::Global(self.rack.project_seed),
                            snapshot.clone(),
                            nodes,
                            params,
                        );

                        // Render and Hash
                        // Find output module index
                        let out_idx = self.rack.modules.iter().position(|m| m.descriptor.id == "dirty_output").unwrap_or(0);
                        let (_, actual_hash) = renderer.render_block(sample_count, out_idx);

                            if actual_hash == expected_hash {
                                self.status_msg = Some(("✅ Verification Passed: Bit-Perfect Reproducibility Confirmed.".to_string(), false));
                            } else {
                                // Run Deep Audit to find WHERE it diverged
                                use dirtyrack_modules::renderer::DeepAuditor;
                                let (_, nodes_a, params_a) = self.rack.build_snapshot();
                                let (_, nodes_b, _params_b) = self.rack.build_snapshot();
                                let mut auditor = DeepAuditor::new(
                                    self.rack.sample_rate,
                                    self.rack.project_seed,
                                    snapshot.clone(),
                                    nodes_a,
                                    nodes_b,
                                    params_a
                                );
                            
                            if let Some((sample, mod_idx, val_a, val_b)) = auditor.find_divergence(sample_count) {
                                let mod_name = &self.rack.modules[mod_idx].descriptor.name;
                                self.status_msg = Some((format!(
                                    "❌ Divergence Detected!\nModule: {}\nSample: {}\nValue A: {:.6}\nValue B: {:.6}",
                                    mod_name, sample, val_a, val_b
                                ), true));
                            } else {
                                self.status_msg = Some(("❌ Hash Mismatch, but no local divergence found (check engine version).".to_string(), true));
                            }
                        }
                        return;
                    }
                }
            }
        }
        self.status_msg = Some(("No audit log found to verify against.".to_string(), true));
    }

    fn rebuild_engine(&mut self) {
        let (snapshot, nodes, params) = self.rack.build_snapshot();
        if let Some(engine) = &self.engine {
            engine.update_topology(snapshot, nodes, params);
        }
    }
    fn show_provenance_timeline(&mut self, ctx: &egui::Context) {
        egui::Window::new("📜 Provenance Timeline").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for event in self.rack.causality_log.iter().rev() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{:.2}s", event.timestamp)).weak());
                        let color = match event.event_type.as_str() {
                            "PARAM" => Color32::LIGHT_BLUE,
                            "SNAPSHOT" => Color32::LIGHT_GREEN,
                            "DIVERGENCE" => Color32::RED,
                            "FAILURE" => Color32::ORANGE,
                            _ => Color32::WHITE,
                        };
                        ui.label(egui::RichText::new(&event.event_type).color(color).strong());
                        ui.label(&event.description);
                    });
                }
            });
            if ui.button("Close").clicked() {
                self.show_provenance_timeline = false;
            }
        });
    }

    fn generate_diagnosis(&self, f: &dirtyrack_sdk::ForensicData) -> String {
        let stats = &f.stats;
        let mut report = String::from("# Pathological Diagnosis Report\n\n");
        
        if stats.clipping_count > 1000 {
            report.push_str("## ⚠ SYMPTOM: Severe Signal Trauma (Clipping)\n");
            report.push_str("- **Observation**: Extensive sample values exceeding ±5V.\n");
            report.push_str("- **Likely Cause**: Excessive resonance in a non-linear feedback loop or extreme input gain.\n");
            report.push_str("- **Suggested Remedy**: Attenuate the feedback amount or reduce pre-filter gain.\n\n");
        }
        
        if stats.denormal_count > 1000 {
            report.push_str("## ⚠ SYMPTOM: Denormal Storm\n");
            report.push_str("- **Observation**: High volume of sub-normal floating point operations.\n");
            report.push_str("- **Likely Cause**: A recursive algorithm (like an IIR filter or feedback delay) is decaying towards zero but never quite reaching it.\n");
            report.push_str("- **Suggested Remedy**: This is an engine-level protection, but you can alleviate it by adding a tiny amount of noise (dither) or increasing the decay speed.\n\n");
        }
        
        if stats.dc_offset.abs() > 0.5 {
            report.push_str("## ⚠ SYMPTOM: DC Drift (Asymmetry)\n");
            report.push_str("- **Observation**: Signal mean is offset from zero by over 0.5V.\n");
            report.push_str("- **Likely Cause**: Asymmetrical saturation (e.g., transistor mode) without a high-pass filter.\n");
            report.push_str("- **Suggested Remedy**: Insert a DC blocker or high-pass filter at 20Hz after the saturation stage.\n\n");
        }

        if report.len() < 40 {
            report.push_str("## ✔ Signal Health: EXCELLENT\n");
            report.push_str("No pathological symptoms detected in the current signal chain.\n");
        }

        report
    }
}

impl eframe::App for DirtyRackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Read Visual Projection ---
        let visual_snapshot = self
            .visual_reader
            .as_mut()
            .map(|r| r.read().clone())
            .unwrap_or_default();

        // Show status message if any
        let status = self.status_msg.clone();
        if let Some((msg, is_error)) = status {
            egui::Window::new("System Status").collapsible(false).show(ctx, |ui| {
                let color = if is_error { Color32::RED } else { Color32::GREEN };
                ui.label(egui::RichText::new(msg).color(color).strong());
                if ui.button("Dismiss").clicked() {
                    self.status_msg = None;
                }
            });
        }

        if self.show_provenance_timeline {
            self.show_provenance_timeline(ctx);
        }

        if let Some(report) = &self.diagnosis_report {
            let mut open = true;
            egui::Window::new("🩺 Diagnostic Report")
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(report);
                    });
                });
            if !open {
                self.diagnosis_report = None;
            }
        }

        // --- Key Bindings ---
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C)) {
            self.rack.handle_action(crate::rack::CableAction::CopySelection, &self.registry, self.zoom, self.pan);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V)) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                let world_pos = (pos - self.pan) / self.zoom;
                self.rack.handle_action(crate::rack::CableAction::PasteSelection { pointer_pos: world_pos }, &self.registry, self.zoom, self.pan);
                self.rebuild_engine();
            }
        }

        // --- Top Toolbar ---
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
            ui.horizontal_wrapped(|ui| {
                ui.visuals_mut().button_frame = true;

                ui.heading(
                    egui::RichText::new("⚡ DirtyRack")
                        .color(egui::Color32::from_rgb(255, 100, 50)),
                );
                
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // --- Project Actions ---
                ui.label("📁 Project:");
                if ui.button("💾 Save").clicked() {
                    let serial = self.rack.to_serializable();
                    if let Ok(json) = serde_json::to_string_pretty(&serial) {
                        let _ = std::fs::write("patch.dirtyrack", json);
                    }
                }
                if ui.button("📂 Load").clicked() {
                    if let Ok(json) = std::fs::read_to_string("patch.dirtyrack") {
                        if let Ok(serial) = serde_json::from_str::<crate::rack::SerializableRack>(&json) {
                            self.rack = crate::rack::RackState::from_serializable(
                                serial,
                                &self.registry,
                                self.rack.sample_rate
                            );
                            self.rebuild_engine();
                        }
                    }
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // --- Edit Actions ---
                ui.label("🔧 Edit:");
                if ui.button("➕ Add Module").clicked() {
                    self.browser_open = !self.browser_open;
                }
                if ui.button("🧺 Clear").clicked() {
                    self.rack.modules.clear();
                    self.rack.cables.clear();
                    self.rebuild_engine();
                }
                if ui.selectable_label(self.inspector_open, "🔍 Inspector").clicked() {
                    self.inspector_open = !self.inspector_open;
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // --- Rack Parameters ---
                ui.label("🧶 Cables:");
                ui.add(egui::Slider::new(&mut self.rack.cable_opacity, 0.1..=1.0).text("Op"));
                ui.add(egui::Slider::new(&mut self.rack.cable_tension, 0.0..=0.5).text("Sag"));
                
                ui.add_space(8.0);
                ui.label("🕰 Aging:");
                if ui.add(egui::Slider::new(&mut self.rack.aging, 0.0..=1.0).text("")).changed() {
                    if let Some(engine) = &self.engine {
                        let _ = engine.sync_aging(self.rack.aging);
                    }
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // --- Diagnostics & Analysis ---
                ui.group(|ui| {
                    ui.label("📸 Audit:");
                    if ui.button("Take Snap").clicked() {
                        let name = format!("Snap {}", self.rack.snapshots.len());
                        self.rack.take_snapshot(&name);
                    }
                    if ui.button("Clear").clicked() {
                        self.rack.snapshots.clear();
                    }
                    
                    if !self.rack.snapshots.is_empty() {
                        ui.separator();
                        egui::ComboBox::from_id_salt("snap_a")
                            .selected_text(format!("A: {}", self.rack.blend_targets.0))
                            .show_ui(ui, |ui| {
                                for name in self.rack.snapshots.keys() {
                                    ui.selectable_value(&mut self.rack.blend_targets.0, name.clone(), name);
                                }
                            });
                        egui::ComboBox::from_id_salt("snap_b")
                            .selected_text(format!("B: {}", self.rack.blend_targets.1))
                            .show_ui(ui, |ui| {
                                for name in self.rack.snapshots.keys() {
                                    ui.selectable_value(&mut self.rack.blend_targets.1, name.clone(), name);
                                }
                            });
                        
                        ui.add(egui::Slider::new(&mut self.rack.snapshot_blend, 0.0..=1.0).text("Blend"));
                        if ui.button("⚡ Apply").clicked() {
                            self.rack.apply_blend();
                        }
                        if ui.button("📊 Diff").clicked() {
                            self.show_diff_audit = true;
                        }
                    }
                });

                // --- Stats ---
                ui.separator();
                ui.label(format!("Modules: {}", self.rack.modules.len()));
                ui.label(format!("Cables: {}", self.rack.cables.len()));
                
                if let Some(engine) = &self.engine {
                    ui.label(egui::RichText::new("🟢 Active").color(Color32::LIGHT_GREEN));
                } else {
                    ui.label(egui::RichText::new("🔴 Error").color(Color32::RED));
                }
            });
        });

        // --- Diff Audit Window ---
        if self.show_diff_audit {
            egui::Window::new("Differential Audit").show(ctx, |ui| {
                let (name_a, name_b) = &self.rack.blend_targets;
                ui.label(format!("Comparing {} → {}", name_a, name_b));
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("diff_grid")
                        .num_columns(5)
                        .spacing([20.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Module").strong());
                            ui.label(egui::RichText::new("Parameter").strong());
                            ui.label(egui::RichText::new(name_a).strong());
                            ui.label(egui::RichText::new(name_b).strong());
                            ui.label(egui::RichText::new("Delta").strong());
                            ui.end_row();

                            let snap_a = self.rack.snapshots.get(name_a);
                            let snap_b = self.rack.snapshots.get(name_b);

                            if let (Some(a), Some(b)) = (snap_a, snap_b) {
                                for m in &self.rack.modules {
                                    if let (Some(pa), Some(pb)) = (a.get(&m.stable_id), b.get(&m.stable_id)) {
                                        for (pname, val_a) in pa {
                                            if let Some(val_b) = pb.get(pname) {
                                                let delta = val_b - val_a;
                                                if delta.abs() > 0.0001 {
                                                    ui.label(&m.descriptor.name);
                                                    ui.label(pname);
                                                    ui.label(format!("{:.4}", val_a));
                                                    ui.label(format!("{:.4}", val_b));
                                                    
                                                    let color = if delta > 0.0 { Color32::LIGHT_GREEN } else { Color32::LIGHT_RED };
                                                    ui.label(egui::RichText::new(format!("{:+2.4}", delta)).color(color));
                                                    ui.end_row();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                });
                if ui.button("Close").clicked() {
                    self.show_diff_audit = false;
                }
            });
        }

        // Forensic Inspector Window was removed and integrated into SidePanel.


        // --- Module Browser Panel ---
        if self.browser_open {
            let prev_count = self.rack.modules.len();
            browser::draw_browser(
                ctx,
                &self.registry,
                &mut self.rack,
                &mut self.browser_open,
                &mut self.browser_search,
            );
            if self.rack.modules.len() != prev_count {
                self.rebuild_engine();
            }
        }

        // --- Main Rack Area ---
        // --- Side Panel (Inspector) ---
        if self.inspector_open {
            egui::SidePanel::right("inspector_panel")
                .resizable(true)
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.heading("🔍 Module Inspector");
                    ui.separator();

                    if let Some(stable_id) = self.selected_module_forensic {
                        if let Some(m_idx) = self.rack.modules.iter().position(|m| m.stable_id == stable_id) {
                            let (m_id, m_name, m_stable_id) = {
                                let m = &self.rack.modules[m_idx];
                                (m.descriptor.id.to_string(), m.descriptor.name.to_string(), m.stable_id)
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("{} [{}]", m_name, m_id)).strong());
                                if ui.button("❌").on_hover_text("Deselect").clicked() {
                                    self.selected_module_forensic = None;
                                }
                            });
                            ui.label(format!("Stable ID: {}", m_stable_id));
                            ui.separator();

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.label(egui::RichText::new("🎚 Parameters").strong());
                                // Copy descriptor params to avoid borrow conflict with self.rack
                                let params_list = self.rack.modules[m_idx].descriptor.params.to_vec();
                                for p_desc in params_list {
                                    let mut val = *self.rack.modules[m_idx].params.get(p_desc.name).unwrap_or(&p_desc.default);
                                    if ui.add(egui::Slider::new(&mut val, p_desc.min..=p_desc.max).text(p_desc.name)).changed() {
                                        let action = CableAction::ParamUpdate {
                                            module_idx: m_idx,
                                            name: p_desc.name.to_string(),
                                            value: val,
                                            intent: IntentBoundary::Commit(IntentClass::Performance, None),
                                        };
                                        self.rack.handle_action(action, &self.registry, self.zoom, self.pan);
                                        if let Some(engine) = &self.engine {
                                            if let Some(updated_m) = self.rack.modules.get(m_idx) {
                                                let params: Vec<f32> = updated_m.descriptor.params
                                                    .iter()
                                                    .map(|p| *updated_m.params.get(p.name).unwrap_or(&p.default))
                                                    .collect();
                                                engine.update_module_parameters(updated_m.stable_id, params);
                                            }
                                        }
                                    }
                                }
                                
                                ui.separator();
                                ui.label(egui::RichText::new("🔬 Forensics").strong());
                                if let Some(v_state) = visual_snapshot.modules.get(&m_stable_id) {
                                    if let Some(forensic) = &v_state.forensic {
                                        ui.label(&forensic.internal_state_summary);

                                        // Visualize Drift per Voice
                                        ui.label("Voice Drift:");
                                        egui_plot::Plot::new("drift_plot")
                                            .height(80.0)
                                            .allow_drag(false)
                                            .show(ui, |plot_ui| {
                                                let points: Vec<egui_plot::Bar> = (0..16).map(|v| {
                                                    egui_plot::Bar::new(v as f64, forensic.current_drift[v] as f64)
                                                }).collect();
                                                plot_ui.bar_chart(egui_plot::BarChart::new(points).name("Current Drift").color(Color32::from_rgb(100, 150, 255)));
                                            });

                                        ui.separator();
                                        ui.label("Engine Health:");
                                        let peak_db = if forensic.stats.peak_db > 0.0 {
                                            20.0 * forensic.stats.peak_db.log10()
                                        } else {
                                            -120.0
                                        };
                                        ui.label(format!("Peak: {:.1} dB", peak_db));
                                        
                                        let clip_color = if forensic.stats.clipping_count > 0 { Color32::RED } else { Color32::GREEN };
                                        ui.label(egui::RichText::new(format!("Clipping Events: {}", forensic.stats.clipping_count)).color(clip_color));

                                        if ui.button("🔬 Diagnosis Report").clicked() {
                                            self.diagnosis_report = Some(self.generate_diagnosis(forensic));
                                        }

                                        // Signal Trace
                                        if let Some(trace) = &forensic.signal_trace {
                                            ui.separator();
                                            ui.label("Polyphonic Trace Audit:");
                                            egui_plot::Plot::new("trace_plot")
                                                .height(150.0)
                                                .legend(egui_plot::Legend::default())
                                                .show(ui, |plot_ui| {
                                                    for v in 0..16 {
                                                        let points: Vec<[f64; 2]> = trace.iter().enumerate().map(|(i, s): (usize, &[f32; 16])| {
                                                            [i as f64, s[v] as f64]
                                                        }).collect();
                                                        plot_ui.line(egui_plot::Line::new(points).name(format!("V{}", v)));
                                                    }
                                                });
                                        }
                                    } else {
                                        ui.label("No forensic data available for this node.");
                                    }
                                }
                            });
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.label("No module selected.\nClick a module to inspect its parameters and forensics.");
                        });
                    }
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let viewport = ui.max_rect();
            let painter = ui.painter().clone();

            // --- Background Interaction (Pan & Zoom & Menu) ---
            // Move this BACK to the beginning (Bottom Layer)
            let bg_id = ui.make_persistent_id("rack_bg");
            let bg_resp = ui.interact(viewport, bg_id, egui::Sense::click_and_drag());

            // Zoom
            if bg_resp.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta);
                if scroll.y != 0.0 {
                    self.zoom = (self.zoom * (1.0 + scroll.y * 0.001)).clamp(0.3, 3.0);
                }
            }

            // Pan (Left drag on background or Middle drag)
            let is_dragging_ui = self.rack.dragging_module.is_some() || self.rack.dragging_cable.is_some();
            
            if bg_resp.drag_started() && !is_dragging_ui && ui.input(|i| i.modifiers.shift) {
                if let Some(pos) = bg_resp.interact_pointer_pos() {
                    self.rack.box_select_start = Some((pos - self.pan) / self.zoom);
                    self.rack.selection.clear();
                }
            }

            if let Some(start) = self.rack.box_select_start {
                if let Some(end_screen) = bg_resp.interact_pointer_pos() {
                    let end = (end_screen - self.pan) / self.zoom;
                    let rect = Rect::from_two_pos(start, end);
                    
                    // Highlight box
                    let screen_rect = Rect::from_two_pos(
                        (start.to_vec2() * self.zoom + self.pan).to_pos2(),
                        end_screen
                    );
                    painter.rect_filled(screen_rect, 0.0, Color32::from_rgba_unmultiplied(0, 180, 255, 30));
                    painter.rect_stroke(screen_rect, 0.0, Stroke::new(1.0, Color32::from_rgb(0, 180, 255)));

                    // Update selection
                    self.rack.selection.clear();
                    for m in &self.rack.modules {
                        if rect.intersects(m.world_rect()) {
                            self.rack.selection.push(m.stable_id);
                        }
                    }
                }

                if bg_resp.drag_stopped() {
                    self.rack.box_select_start = None;
                }
            }

            if (bg_resp.dragged_by(egui::PointerButton::Primary) && !is_dragging_ui && self.rack.box_select_start.is_none() && !ui.ctx().is_using_pointer())
                || bg_resp.dragged_by(egui::PointerButton::Middle)
            {
                self.pan += bg_resp.drag_delta();
            }

            if bg_resp.clicked_by(egui::PointerButton::Primary) && !is_dragging_ui && !ui.ctx().is_using_pointer() {
                self.rack.selection.clear();
            }

            // Right Click Context Menu
            bg_resp.context_menu(|ui| {
                ui.label("Rack Menu");
                if ui.button("➕ Add Module").clicked() {
                    self.browser_open = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🧹 Clear All Cables").clicked() {
                    self.rack.cables.clear();
                    self.rebuild_engine();
                    ui.close_menu();
                }
            });

            // --- Background & Rails (Passive Paint Pass) ---
            {
                painter.rect_filled(viewport, 0.0, egui::Color32::from_rgb(25, 22, 20));
                rack::draw_rack_rails(&painter, viewport, self.zoom, self.pan);
            }



            // Draw modules (mutable interaction pass)
            let mut cable_action = None;
            for i in 0..self.rack.modules.len() {
                let action = faceplate::draw_module(
                    ui,
                    &mut self.rack,
                    &self.registry,
                    i,
                    self.zoom,
                    self.pan,
                    self.mri_mode,
                    &visual_snapshot,
                );
                
                if let Some(a) = action {
                    cable_action = Some(a);
                }
            }

            // Handle cable actions
            if let Some(action) = cable_action {
                match action {
                    CableAction::InspectForensics { stable_id } => {
                        self.selected_module_forensic = Some(stable_id);
                        if !self.rack.selection.contains(&stable_id) {
                            self.rack.selection.clear();
                            self.rack.selection.push(stable_id);
                        }
                        self.inspector_open = true;
                    }
                    CableAction::SelectModule { stable_id, .. } => {
                        self.rack.handle_action(action, &self.registry, self.zoom, self.pan);
                        self.selected_module_forensic = Some(stable_id);
                    }
                    CableAction::StartModuleDrag { module_idx, .. } => {
                        let stable_id = self.rack.modules[module_idx].stable_id;
                        if !self.rack.selection.contains(&stable_id) {
                            self.rack.selection.clear();
                            self.rack.selection.push(stable_id);
                            self.selected_module_forensic = Some(stable_id);
                        }
                        self.rack.handle_action(action, &self.registry, self.zoom, self.pan);
                    }
                    CableAction::ParamUpdate { module_idx, .. } => {
                        self.rack.handle_action(action.clone(), &self.registry, self.zoom, self.pan);
                        if let Some(engine) = &self.engine {
                            let m = &self.rack.modules[module_idx];
                            let params: Vec<f32> = m.descriptor.params
                                .iter()
                                .map(|p| *m.params.get(p.name).unwrap_or(&p.default))
                                .collect();
                            engine.update_module_parameters(m.stable_id, params);
                        }
                    }
                    CableAction::MoveModule { .. } => {
                        self.rack.handle_action(action, &self.registry, self.zoom, self.pan);
                        // Do NOT rebuild engine here to avoid audio resets during drag
                    }
                    CableAction::CancelDrag => {
                        self.rack.handle_action(action, &self.registry, self.zoom, self.pan);
                        // Rebuild once when drag ends
                        self.rebuild_engine();
                    }
                    _ => {
                        self.rack.handle_action(action, &self.registry, self.zoom, self.pan);
                        self.rebuild_engine();
                    }
                }
            }

            // Draw cables (immutable paint pass)
            {
                cable::draw_cables(&painter, &self.rack, self.zoom, self.pan);

                if self.rack.dragging_cable.is_some() {
                    if let Some(ptr) = ctx.pointer_interact_pos() {
                        cable::draw_dragging_cable(&painter, &self.rack, ptr, self.zoom, self.pan);
                    }
                    ctx.request_repaint();
                }
            }
        });

        // Request repaint for audio-driven visuals
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
