//! Circuit Editor — egui-based visual circuit designer
//!
//! 抵抗、コンデンサ、真空管等を配置し、ノード間を配線。
//! MNAソルバー用の構成データを出力。

use dirtydata_dsp_circuit::{CircuitElement, Material, NodeId};
use dirtyrack_modules::circuit::CircuitDefinition;
use egui::{Color32, Pos2, Stroke, Vec2};

pub struct CircuitEditor {
    pub open: bool,
    pub target_module_stable_id: Option<u64>,
    pub definition: CircuitDefinition,
    pub pan: Vec2,
    pub zoom: f32,
    pub selected_element: Option<usize>,
    pub dragging_node: Option<usize>,
    pub hover_pos: Option<Pos2>,
}

impl CircuitEditor {
    pub fn new() -> Self {
        Self {
            open: false,
            target_module_stable_id: None,
            definition: CircuitDefinition {
                elements: Vec::new(),
                num_nodes: 1, // Node 0 is Ground
                input_mappings: Vec::new(),
                output_mappings: Vec::new(),
            },
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected_element: None,
            dragging_node: None,
            hover_pos: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<CircuitDefinition> {
        let mut result = None;
        if !self.open {
            return None;
        }

        let mut open = self.open;
        let mut apply_clicked = false;

        egui::Window::new("🛠 Circuit Sandbox Editor")
            .open(&mut open)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Components");
                        if ui.button("➕ Resistor").clicked() {
                            self.add_element(CircuitElement::Resistor {
                                a: NodeId(0),
                                b: NodeId(0),
                                value: 1000.0,
                                tolerance: 0.01,
                                material: Material::MetalFilm,
                            });
                        }
                        if ui.button("➕ Capacitor").clicked() {
                            self.add_element(CircuitElement::Capacitor {
                                a: NodeId(0),
                                b: NodeId(0),
                                value: 1e-7,
                                tolerance: 0.1,
                                state_v: 0.0,
                                material: Material::Ceramic,
                            });
                        }
                        if ui.button("➕ Diode").clicked() {
                            self.add_element(CircuitElement::Diode {
                                a: NodeId(0),
                                k: NodeId(0),
                                material: Material::Silicon,
                                is: 1e-12,
                            });
                        }
                        if ui.button("➕ Tube (Triode)").clicked() {
                            self.add_element(CircuitElement::Triode {
                                g: NodeId(0),
                                k: NodeId(0),
                                p: NodeId(0),
                                mu: 100.0,
                                kg1: 1060.0,
                                kp: 600.0,
                                kvb: 300.0,
                                ex: 1.4,
                            });
                        }
                        ui.separator();
                        if ui.button("💾 Apply to Module").clicked() {
                            apply_clicked = true;
                        }
                    });

                    ui.separator();

                    // --- Canvas Area ---
                    let (rect, response) =
                        ui.allocate_at_least(ui.available_size(), egui::Sense::click_and_drag());
                    self.hover_pos = response.hover_pos();

                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 25, 20));

                    // Draw Grid
                    let grid_size = 20.0 * self.zoom;
                    let mut x = rect.left() + self.pan.x % grid_size;
                    while x < rect.right() {
                        painter.line_segment(
                            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                            Stroke::new(1.0, Color32::from_gray(30)),
                        );
                        x += grid_size;
                    }
                    let mut y = rect.top() + self.pan.y % grid_size;
                    while y < rect.bottom() {
                        painter.line_segment(
                            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                            Stroke::new(1.0, Color32::from_gray(30)),
                        );
                        y += grid_size;
                    }

                    // --- Interaction ---
                    if response.dragged_by(egui::PointerButton::Middle) {
                        self.pan += response.drag_delta();
                    }

                    // Draw elements (dummy for now)
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Circuit Canvas - WIP\n(Schematic capture logic to be implemented)",
                        egui::FontId::proportional(20.0),
                        Color32::GRAY,
                    );
                });
            });

        self.open = open;
        if apply_clicked {
            result = Some(self.definition.clone());
        }
        result
    }

    fn add_element(&mut self, el: CircuitElement) {
        self.definition.elements.push(el);
    }
}
