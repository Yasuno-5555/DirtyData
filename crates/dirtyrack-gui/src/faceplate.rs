//! Faceplate Drawing — モジュールフェースプレート描画
//!
//! ノブ・ジャック・スイッチ・LEDをegui上でカスタム描画。

use crate::rack::{CableAction, RackState};
use dirtyrack_modules::signal::{
    IntentBoundary, IntentClass, IntentMetadata, ParamKind,
    PortDirection, SignalType,
};
use egui::{vec2, Color32, Pos2, Rect, Stroke, Ui, Vec2};

/// 信号タイプ別のポート色
fn port_color(sig: SignalType) -> Color32 {
    match sig {
        SignalType::Audio => Color32::from_rgb(220, 50, 50),
        SignalType::VoltPerOct => Color32::from_rgb(50, 150, 255),
        SignalType::UniCV | SignalType::BiCV => Color32::from_rgb(100, 200, 255),
        SignalType::Gate => Color32::from_rgb(255, 220, 50),
        SignalType::Trigger => Color32::from_rgb(255, 180, 50),
        SignalType::Clock => Color32::from_rgb(50, 255, 150),
    }
}

use dirtyrack_modules::registry::ModuleDescriptor;
/// Pre-collected module layout data to avoid borrow conflicts
use std::sync::Arc;

struct ModuleLayout {
    screen_rect: Rect,
    descriptor: Arc<ModuleDescriptor>,
}

/// モジュール1つのフェースプレートを描画
pub fn draw_module(
    ui: &mut Ui,
    rack: &mut RackState,
    _registry: &crate::rack::ModuleRegistry,
    module_idx: usize,
    zoom: f32,
    pan: Vec2,
    mri_mode: bool,
    visual_snapshot: &crate::visual_data::VisualSnapshot,
) -> Option<CableAction> {
    let mut cable_action = None;
    // Pre-collect all static data to avoid borrow conflicts
    let layout = {
        let module = &rack.modules[module_idx];
        let world_rect = module.world_rect();
        ModuleLayout {
            screen_rect: Rect::from_min_size(
                (world_rect.min.to_vec2() * zoom + pan).to_pos2(),
                world_rect.size() * zoom,
            ),
            descriptor: Arc::clone(&module.descriptor),
        }
    };

    let screen_rect = layout.screen_rect;

    // --- Faceplate Interaction ---
    let handle_height = 30.0 * zoom;
    let handle_rect = Rect::from_min_max(
        screen_rect.left_top(),
        screen_rect.left_top() + vec2(screen_rect.width(), handle_height),
    );

    let face_id = ui.make_persistent_id(("face_panel", module_idx));
    let face_resp = ui.interact(screen_rect, face_id, egui::Sense::click_and_drag());
    
    if face_resp.drag_started() {
        if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
            cable_action = Some(CableAction::StartModuleDrag {
                module_idx,
                press_pos: pos,
            });
        }
    }
    if face_resp.dragged() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            cable_action = Some(CableAction::MoveModule {
                module_idx,
                pointer_pos: pos,
            });
        }
    }
    if face_resp.drag_stopped() {
        cable_action = Some(CableAction::CancelDrag);
    }
    
    if face_resp.clicked() {
        let stable_id = rack.modules[module_idx].stable_id;
        let additive = ui.input(|i| i.modifiers.command || i.modifiers.shift);
        cable_action = Some(CableAction::SelectModule { stable_id, additive });
    }

    // Visual Handle Background
    ui.painter().rect_filled(
        handle_rect,
        4.0 * zoom,
        Color32::from_rgba_unmultiplied(255, 255, 255, 10),
    );
    ui.painter().line_segment(
        [handle_rect.left_bottom(), handle_rect.right_bottom()],
        Stroke::new(1.0 * zoom, Color32::from_gray(60)),
    );

    // --- Right-click Context Menu ---
    face_resp.context_menu(|ui| {
        if ui.button("🔍 Forensic Inspector").clicked() {
            cable_action = Some(CableAction::InspectForensics {
                stable_id: rack.modules[module_idx].stable_id,
            });
            ui.close_menu();
        }
        ui.separator();
        if ui.button("🔄 Initialize").clicked() {
            cable_action = Some(CableAction::ResetModule { module_idx });
            ui.close_menu();
        }
        if ui.button("🎲 Randomize").clicked() {
            cable_action = Some(CableAction::RandomizeParams { module_idx });
            ui.close_menu();
        }
        ui.separator();
        if ui.button("🚫 Bypass").clicked() {
            cable_action = Some(CableAction::ToggleBypass { module_idx });
            ui.close_menu();
        }
        if ui.button("🗑 Remove").clicked() {
            cable_action = Some(CableAction::RemoveModule { module_idx });
            ui.close_menu();
        }
    });

    // --- Faceplate Background Graphics ---
    {
        let painter = ui.painter();
        let visuals = &layout.descriptor.visuals;
        let bg = Color32::from_rgb(
            visuals.background_color[0],
            visuals.background_color[1],
            visuals.background_color[2],
        );
        let text_color = Color32::from_rgb(
            visuals.text_color[0],
            visuals.text_color[1],
            visuals.text_color[2],
        );
        let accent = Color32::from_rgb(
            visuals.accent_color[0],
            visuals.accent_color[1],
            visuals.accent_color[2],
        );

        painter.rect_filled(screen_rect, 4.0 * zoom, bg);
        painter.rect_stroke(
            screen_rect,
            4.0 * zoom,
            Stroke::new(1.0 * zoom, Color32::from_gray(80)),
        );

        let is_selected = rack.selection.contains(&rack.modules[module_idx].stable_id);
        let module_rect = layout.screen_rect;
        let visual_state = visual_snapshot.modules.get(&rack.modules[module_idx].stable_id);

        // Highlight if selected
        if is_selected {
            painter.rect_stroke(
                module_rect,
                0.0,
                Stroke::new(3.0 * zoom, Color32::from_rgb(255, 215, 0)), // Gold
            );
        }

        // --- Tier SSS: Patch MRI Overlay ---
        if mri_mode {
            if let Some(visual) = visual_state {
            if let Some(f) = &visual.forensic {
                let stats = &f.stats;
                
                // Clipping Glow (Red)
                if stats.clipping_count > 0 {
                    let intensity = (stats.clipping_count as f32 / 1000.0).min(1.0);
                    painter.rect_filled(
                        module_rect,
                        0.0,
                        Color32::from_rgba_unmultiplied(255, 0, 0, (intensity * 100.0) as u8),
                    );
                }

                // Energy Density (Orange Heat)
                if stats.energy_delta > 0.1 {
                    let intensity = (stats.energy_delta / 50.0).min(0.8);
                    painter.rect_filled(
                        module_rect,
                        0.0,
                        Color32::from_rgba_unmultiplied(255, 100, 0, (intensity * 80.0) as u8),
                    );
                }

                // DC Drift (Purple Aura)
                if stats.dc_offset.abs() > 0.1 {
                    let intensity = (stats.dc_offset.abs() / 2.0).min(1.0);
                    painter.rect_stroke(
                        module_rect.expand(4.0 * zoom),
                        4.0 * zoom,
                        Stroke::new(2.0 * zoom, Color32::from_rgba_unmultiplied(200, 0, 255, (intensity * 150.0) as u8)),
                    );
                }
            }
        }
    }

        // Left Accent Strip (Brand indicator)
        let strip_rect = Rect::from_min_max(
            screen_rect.left_top(),
            screen_rect.left_top() + vec2(4.0 * zoom, screen_rect.height()),
        );
        painter.rect_filled(strip_rect, 0.0, accent);

        // Module Name
        painter.text(
            screen_rect.center_top() + vec2(0.0, 18.0 * zoom),
            egui::Align2::CENTER_TOP,
            &layout.descriptor.name,
            egui::FontId::proportional(13.0 * zoom),
            text_color,
        );

        // Manufacturer
        painter.text(
            screen_rect.center_top() + vec2(0.0, 6.0 * zoom),
            egui::Align2::CENTER_TOP,
            &layout.descriptor.manufacturer,
            egui::FontId::proportional(8.0 * zoom),
            text_color.gamma_multiply(0.5),
        );
    }

    // --- Remove Button ---
    let remove_btn_size = 12.0 * zoom;
    let remove_btn_rect = Rect::from_min_size(
        Pos2::new(
            screen_rect.right() - remove_btn_size - 4.0 * zoom,
            screen_rect.top() + 4.0 * zoom,
        ),
        vec2(remove_btn_size, remove_btn_size),
    );
    let remove_id = ui.make_persistent_id(("remove", module_idx));
    let remove_resp = ui.interact(remove_btn_rect, remove_id, egui::Sense::click());

    ui.painter().circle_filled(
        remove_btn_rect.center(),
        remove_btn_size * 0.5,
        if remove_resp.hovered() {
            Color32::from_rgb(200, 50, 50)
        } else {
            Color32::from_rgb(100, 40, 40)
        },
    );
    ui.painter().text(
        remove_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        egui::FontId::proportional(10.0 * zoom),
        Color32::WHITE,
    );

    if remove_resp.clicked() {
        cable_action = Some(CableAction::RemoveModule { module_idx });
    }

    // --- Parameters (Knobs/Switches) ---
    for param_desc in &layout.descriptor.params {
        let center = Pos2::new(
            screen_rect.left() + param_desc.position[0] * screen_rect.width(),
            screen_rect.top() + param_desc.position[1] * screen_rect.height(),
        );

        match param_desc.kind {
            ParamKind::Knob => {
                if let Some(action) = draw_knob(
                    ui,
                    rack,
                    module_idx,
                    param_desc.name,
                    center,
                    zoom,
                    param_desc.min,
                    param_desc.max,
                    param_desc.default,
                ) {
                    cable_action = Some(action);
                }
            }
            ParamKind::Switch { positions } => {
                draw_switch(ui, center, zoom, positions);
            }
            ParamKind::Button => {
                draw_button(ui.painter(), center, zoom);
            }
            ParamKind::Slider => {
                draw_slider_widget(
                    ui,
                    rack,
                    module_idx,
                    param_desc.name,
                    center,
                    zoom,
                    param_desc.min,
                    param_desc.max,
                    param_desc.default,
                );
            }
        }

        // Param label
        ui.painter().text(
            center + vec2(0.0, 16.0 * zoom),
            egui::Align2::CENTER_TOP,
            param_desc.name,
            egui::FontId::proportional(8.0 * zoom),
            Color32::from_gray(140),
        );
    }

    // --- Ports (Jacks) ---
    for port_desc in &layout.descriptor.ports {
        let center = Pos2::new(
            screen_rect.left() + port_desc.position[0] * screen_rect.width(),
            screen_rect.top() + port_desc.position[1] * screen_rect.height(),
        );

        let color = port_color(port_desc.signal_type);
        let is_output = port_desc.direction == PortDirection::Output;
        let jack_radius = 8.0 * zoom;

        {
            let painter = ui.painter();
            painter.circle_filled(center, jack_radius + 2.0 * zoom, Color32::from_gray(20));

            // LED Glow based on voltage
            let stable_id = rack.modules[module_idx].stable_id;
            if let Some(vs) = visual_snapshot.modules.get(&stable_id) {
                let voltage = if is_output {
                    let idx = layout
                        .descriptor
                        .ports
                        .iter()
                        .filter(|p| p.direction == PortDirection::Output)
                        .position(|p| p.name == port_desc.name)
                        .unwrap_or(0);
                    vs.outputs.get(idx).copied().unwrap_or(0.0)
                } else {
                    let idx = layout
                        .descriptor
                        .ports
                        .iter()
                        .filter(|p| p.direction == PortDirection::Input)
                        .position(|p| p.name == port_desc.name)
                        .unwrap_or(0);
                    vs.inputs.get(idx).copied().unwrap_or(0.0)
                };

                let intensity = (voltage.abs() * 0.2).min(1.0);
                if intensity > 0.05 {
                    painter.circle_filled(
                        center,
                        jack_radius * 0.6,
                        color.gamma_multiply(intensity),
                    );
                }
            }

            painter.circle_stroke(center, jack_radius, Stroke::new(2.0 * zoom, color));
            painter.circle_stroke(
                center,
                jack_radius - 2.0 * zoom,
                Stroke::new(1.0 * zoom, Color32::from_gray(60)),
            );
            painter.text(
                center + vec2(0.0, -12.0 * zoom),
                egui::Align2::CENTER_BOTTOM,
                &port_desc.name,
                egui::FontId::proportional(7.0 * zoom),
                color,
            );
        }

        // Interaction
        let port_rect = Rect::from_center_size(center, vec2(jack_radius * 2.5, jack_radius * 2.5));
        let port_id = ui.make_persistent_id((module_idx, &port_desc.name));
        let response = ui.interact(port_rect, port_id, egui::Sense::click_and_drag());

        // --- Right-click Menu for Module ---
        response.context_menu(|ui| {
            if ui.button("Bypass").clicked() {
                cable_action = Some(CableAction::ToggleBypass { module_idx });
                ui.close_menu();
            }
            if ui.button("Randomize").clicked() {
                cable_action = Some(CableAction::RandomizeParams { module_idx });
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Remove Module").clicked() {
                cable_action = Some(CableAction::RemoveModule { module_idx });
                ui.close_menu();
            }
        });

        if response.hovered() {
            ui.painter().circle_stroke(
                center,
                jack_radius + 4.0 * zoom,
                Stroke::new(1.5 * zoom, Color32::WHITE),
            );
        }

        if response.clicked_by(egui::PointerButton::Secondary) {
            cable_action = Some(CableAction::DisconnectPort {
                module_idx,
                port_name: port_desc.name.to_string(),
            });
        }

        if response.drag_started() {
            cable_action = Some(CableAction::StartDrag {
                module_idx,
                port_name: port_desc.name.to_string(),
                is_output,
            });
        }

        // ここが重要：ドラッグが終了したとき、どのポートの上であっても、
        // 最後にドラッグしていたポート（開始元）が drag_stopped を検知する。
        // その瞬間のポインタ位置を送信して、lib.rs 側でヒットテストを行う。
        if response.drag_stopped() && rack.dragging_cable.is_some() {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                cable_action = Some(CableAction::EndDrag { pointer_pos: pos });
            }
        }
    }

    // --- Visual Displays (Scope, etc.) ---
    let stable_id = rack.modules[module_idx].stable_id;
    if let Some(vs) = visual_snapshot.modules.get(&stable_id) {
        if !vs.scope_data.is_empty() {
            let display_rect = Rect::from_center_size(
                screen_rect.center() + vec2(0.0, -40.0 * zoom),
                vec2(screen_rect.width() * 0.9, 120.0 * zoom),
            );

            let painter = ui.painter();
            painter.rect_filled(display_rect, 2.0 * zoom, Color32::from_rgb(10, 20, 10));
            painter.rect_stroke(
                display_rect,
                2.0 * zoom,
                Stroke::new(1.0 * zoom, Color32::from_rgb(40, 60, 40)),
            );

            // Draw Waveform
            let points: Vec<Pos2> = vs
                .scope_data
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let x = display_rect.left()
                        + (i as f32 / vs.scope_data.len() as f32) * display_rect.width();
                    let y = display_rect.center().y - v * 5.0 * zoom; // 5V scaling
                    Pos2::new(x, y.clamp(display_rect.top(), display_rect.bottom()))
                })
                .collect();

            if points.len() > 1 {
                painter.add(egui::Shape::line(
                    points,
                    Stroke::new(1.5 * zoom, Color32::from_rgb(100, 255, 100)),
                ));
            }
        }
    }

    cable_action
}

fn draw_knob(
    ui: &mut Ui,
    rack: &mut RackState,
    module_idx: usize,
    name: &str,
    center: Pos2,
    zoom: f32,
    min: f32,
    max: f32,
    default: f32,
) -> Option<CableAction> {
    let radius = 12.0 * zoom;

    let current = rack.modules[module_idx]
        .params
        .get(name)
        .copied()
        .unwrap_or(default);
    let mut action = None;

    // Interaction
    let knob_rect = Rect::from_center_size(center, vec2(radius * 2.5, radius * 2.5));
    let knob_id = ui.make_persistent_id(("knob", module_idx, name));
    let response = ui.interact(knob_rect, knob_id, egui::Sense::drag().union(egui::Sense::click()));

    // --- Right-click Mapping Menu ---
    response.context_menu(|ui| {
        ui.label(egui::RichText::new(format!("Map {}", name)).strong());
        ui.separator();
        
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for (m_idx, m) in rack.modules.iter().enumerate() {
                ui.menu_button(format!("{}: {}", m_idx, m.descriptor.name), |ui| {
                    for (p_idx, p) in m.descriptor.ports.iter().filter(|p| p.direction == PortDirection::Output).enumerate() {
                        if ui.button(&*p.name).clicked() {
                            action = Some(CableAction::AddModMapping {
                                target_module_idx: module_idx,
                                param_name: name.to_string(),
                                src_stable_id: m.stable_id,
                                src_port_idx: p_idx,
                            });
                            ui.close_menu();
                        }
                    }
                });
            }
        });

        if !rack.modules[module_idx].param_modulations.get(name).map(|v| v.is_empty()).unwrap_or(true) {
            ui.separator();
            if ui.button("🗑 Clear Mappings").clicked() {
                action = Some(CableAction::ClearModMappings {
                    module_idx,
                    param_name: name.to_string(),
                });
                ui.close_menu();
            }
        }
    });

    if response.drag_started() {
        action = Some(CableAction::ParamUpdate {
            module_idx,
            name: name.to_string(),
            value: current,
            intent: IntentBoundary::Begin,
        });
    } else if response.dragged() {
        let delta = -response.drag_delta().y * 0.005;
        let v = (current + delta * (max - min)).clamp(min, max);
        action = Some(CableAction::ParamUpdate {
            module_idx,
            name: name.to_string(),
            value: v,
            intent: IntentBoundary::Intermediate,
        });
    } else if response.drag_stopped() {
        let meta = IntentMetadata {
            note: "Param changed".to_string(),
            confidence_score: 1.0,
            hypothesis_tag: None,
        };
        action = Some(CableAction::ParamUpdate {
            module_idx,
            name: name.to_string(),
            value: current,
            intent: IntentBoundary::Commit(IntentClass::Edit, Some(meta)),
        });
    }

    // Tooltip for value
    response.on_hover_ui(|ui| {
        ui.horizontal(|ui| {
            ui.label(format!("{}:", name));
            let unit = rack.modules[module_idx].descriptor.params.iter()
                .find(|p| p.name == name)
                .map(|p| p.unit)
                .unwrap_or("");
            
            // Check for snapshot diff
            let stable_id = rack.modules[module_idx].stable_id;
            let mut diff_str = String::new();
            if let Some(snap_a) = rack.snapshots.get("A") {
                if let Some(mod_snap) = snap_a.get(&stable_id) {
                    if let Some(&old_val) = mod_snap.get(name) {
                        if (old_val - current).abs() > 0.0001 {
                            diff_str = format!(" (Δ {:.3})", current - old_val);
                        }
                    }
                }
            }

            ui.label(egui::RichText::new(format!("{:.3} {}{}", current, unit, diff_str)).strong());
        });
    });

    // Draw knob
    {
        let painter = ui.painter();
        
        // --- Divergence Glow ---
        let stable_id = rack.modules[module_idx].stable_id;
        if let Some(snap_a) = rack.snapshots.get("A") {
            if let Some(mod_snap) = snap_a.get(&stable_id) {
                if let Some(&old_val) = mod_snap.get(name) {
                    if (old_val - current).abs() > 0.001 {
                        // Parameter has drifted! Draw an orange glow
                        painter.circle_stroke(center, radius + 2.0 * zoom, Stroke::new(2.0 * zoom, Color32::from_rgb(255, 165, 0)));
                    }
                }
            }
        }

        painter.circle_filled(center, radius, Color32::from_rgb(50, 50, 55));
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.5 * zoom, Color32::from_gray(80)),
        );

        let display_val = match &action {
            Some(CableAction::ParamUpdate { value, .. }) => *value,
            _ => current,
        };
        let normalized = ((display_val - min) / (max - min)).clamp(0.0, 1.0);
        let angle = -std::f32::consts::PI * 0.75 + normalized * std::f32::consts::PI * 1.5;
        let indicator_end = center + Vec2::new(angle.cos(), -angle.sin()) * radius * 0.7;
        painter.line_segment(
            [center, indicator_end],
            Stroke::new(2.0 * zoom, Color32::WHITE),
        );

        let arc_radius = radius + 3.0 * zoom;
        for i in 0..30 {
            let t = i as f32 / 30.0;
            let a = -std::f32::consts::PI * 0.75 + t * std::f32::consts::PI * 1.5;
            let p = center + Vec2::new(a.cos(), -a.sin()) * arc_radius;
            let c = if t <= normalized {
                Color32::from_rgb(100, 200, 255)
            } else {
                Color32::from_gray(40)
            };
            painter.circle_filled(p, 1.0 * zoom, c);
        }
    }

    action
}

fn draw_switch(ui: &mut Ui, center: Pos2, zoom: f32, _positions: u8) {
    let painter = ui.painter();
    let size = vec2(10.0, 16.0) * zoom;
    let rect = Rect::from_center_size(center, size);
    painter.rect_filled(rect, 2.0 * zoom, Color32::from_gray(50));
    painter.rect_stroke(rect, 2.0 * zoom, Stroke::new(1.0, Color32::from_gray(80)));
    let handle = Rect::from_center_size(center + vec2(0.0, -3.0 * zoom), vec2(8.0, 4.0) * zoom);
    painter.rect_filled(handle, 1.0 * zoom, Color32::from_rgb(180, 180, 180));
}


fn draw_button(painter: &egui::Painter, center: Pos2, zoom: f32) {
    painter.circle_filled(center, 6.0 * zoom, Color32::from_rgb(180, 40, 40));
    painter.circle_stroke(
        center,
        6.0 * zoom,
        Stroke::new(1.0 * zoom, Color32::from_gray(100)),
    );
}

fn draw_slider_widget(
    ui: &mut Ui,
    rack: &mut RackState,
    module_idx: usize,
    name: &str,
    center: Pos2,
    zoom: f32,
    min: f32,
    max: f32,
    default: f32,
) {
    let height = 40.0 * zoom;
    let width = 8.0 * zoom;

    let current = rack.modules[module_idx]
        .params
        .get(name)
        .copied()
        .unwrap_or(default);
    let normalized = ((current - min) / (max - min)).clamp(0.0, 1.0);

    let painter = ui.painter();
    let track = Rect::from_center_size(center, vec2(width, height));
    painter.rect_filled(track, 2.0 * zoom, Color32::from_gray(30));

    let handle_y = track.bottom() - normalized * height;
    let handle = Rect::from_center_size(Pos2::new(center.x, handle_y), vec2(12.0, 6.0) * zoom);
    painter.rect_filled(handle, 2.0 * zoom, Color32::WHITE);
}
