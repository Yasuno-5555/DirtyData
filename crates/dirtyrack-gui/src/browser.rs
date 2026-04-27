//! Module Browser — モジュール追加UI

use crate::rack::RackState;
use dirtyrack_modules::registry::ModuleRegistry;
use egui::Color32;

/// モジュールブラウザを描画
pub fn draw_browser(
    ctx: &egui::Context,
    registry: &ModuleRegistry,
    rack: &mut RackState,
    browser_open: &mut bool,
    search_query: &mut String,
) {
    egui::SidePanel::left("module_browser")
        .resizable(true)
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading(
                egui::RichText::new("Module Browser").color(Color32::from_rgb(255, 200, 100)),
            );
            ui.separator();

            // Search bar
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(search_query);
                if ui.button("Clear").clicked() {
                    search_query.clear();
                }
            });
            ui.separator();

            // Category buttons
            ui.horizontal_wrapped(|ui| {
                let categories = [
                    "OSC", "FLT", "AMP", "ENV", "LFO", "SEQ", "MIX", "CLK", "UTL", "FX",
                ];
                for label in categories {
                    let is_selected = search_query.to_uppercase() == label;
                    if ui.selectable_label(is_selected, label).clicked() {
                        if is_selected {
                            search_query.clear();
                        } else {
                            *search_query = label.to_string();
                        }
                    }
                }
            });
            ui.separator();

            // Module list
            egui::ScrollArea::vertical().show(ui, |ui| {
                let modules = if search_query.is_empty() {
                    registry.all()
                } else {
                    registry.search(search_query)
                };

                for descriptor in &modules {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Module icon
                            let icon = match descriptor.tags.first().map(|s| s.as_str()) {
                                Some("OSC" | "VCO") => "〰",
                                Some("FLT" | "VCF") => "▽",
                                Some("VCA") => "▲",
                                Some("Envelope") => "⌒",
                                Some("LFO") => "∿",
                                Some("Sequencer") => "⊞",
                                Some("Mixer") => "Σ",
                                Some("Clock") => "⏱",
                                Some("Noise") => "⚡",
                                Some("Scope") => "📊",
                                Some("Output") => "🔊",
                                _ => "◆",
                            };
                            ui.label(egui::RichText::new(icon).size(18.0));

                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&descriptor.name)
                                        .strong()
                                        .color(Color32::WHITE),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}HP • {} ports",
                                        descriptor.hp_width,
                                        descriptor.ports.len()
                                    ))
                                    .small()
                                    .color(Color32::from_gray(140)),
                                );
                            });
                        });

                        if ui.button("+ Add to Rack").clicked() {
                            rack.add_module(std::sync::Arc::clone(descriptor));
                        }
                    });
                    ui.add_space(2.0);
                }
            });

            ui.separator();
            if ui.button("Close Browser").clicked() {
                *browser_open = false;
            }
        });
}
