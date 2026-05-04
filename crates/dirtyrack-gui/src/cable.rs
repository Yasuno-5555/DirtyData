//! Patch Cable Drawing & Physics
//!
//! 重力シミュレーション付きベジェ曲線ケーブル。

use crate::rack::{CableAction, RackState, ModuleRegistry};
use egui::{vec2, Color32, Painter, Pos2, Stroke, Vec2};


/// ケーブル色パレット（接続順にサイクル）
pub const CABLE_COLORS: &[Color32] = &[
    Color32::from_rgb(220, 60, 60),   // Red
    Color32::from_rgb(60, 140, 255),  // Blue
    Color32::from_rgb(255, 220, 50),  // Yellow
    Color32::from_rgb(60, 220, 120),  // Green
    Color32::from_rgb(200, 100, 255), // Purple
    Color32::from_rgb(255, 140, 60),  // Orange
    Color32::from_rgb(255, 255, 255), // White
];

/// ケーブルアクションを処理（RackStateに委譲）
pub fn handle_cable_action(rack: &mut RackState, registry: &ModuleRegistry, action: CableAction, zoom: f32, pan: Vec2) {
    rack.handle_action(action, registry, zoom, pan);
}

/// すべてのパッチケーブルを描画
pub fn draw_cables(painter: &Painter, rack: &RackState, zoom: f32, pan: Vec2) {
    for cable in &rack.cables {
        let from_pos = rack.port_world_pos(cable.from_module, &cable.from_port);
        let to_pos = rack.port_world_pos(cable.to_module, &cable.to_port);

        if let (Some(from), Some(to)) = (from_pos, to_pos) {
            let screen_from = (from.to_vec2() * zoom + pan).to_pos2();
            let screen_to = (to.to_vec2() * zoom + pan).to_pos2();
            
            // Tier B: Polyphonic Thickness & Activity
            let thickness = (2.0 + (cable.channels as f32 * 0.5)).min(8.0);
            
            // Add subtle pulsing based on signal (if we had access to real-time signal here)
            // For now, use the channel count to drive thickness
            
            let mut color = cable.color;
            color = Color32::from_rgba_unmultiplied(
                color.r(),
                color.g(),
                color.b(),
                (rack.cable_opacity * 255.0) as u8,
            );

            draw_cable_curve(
                painter,
                screen_from,
                screen_to,
                color,
                zoom,
                thickness,
                rack.cable_tension,
            );
        }
    }
}

/// ドラッグ中のケーブルを描画
pub fn draw_dragging_cable(
    painter: &Painter,
    rack: &RackState,
    pointer: Pos2,
    zoom: f32,
    pan: Vec2,
) {
    if let Some(drag) = &rack.dragging_cable {
        let from_pos = rack.port_world_pos(drag.from_module, &drag.from_port);
        if let Some(from) = from_pos {
            let screen_from = (from.to_vec2() * zoom + pan).to_pos2();
            draw_cable_curve(
                painter,
                screen_from,
                pointer,
                Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                zoom,
                3.0,
                rack.cable_tension,
            );
        }
    }
}

/// 1本のケーブルをベジェ曲線＋重力たるみで描画
fn draw_cable_curve(
    painter: &Painter,
    from: Pos2,
    to: Pos2,
    color: Color32,
    zoom: f32,
    thickness: f32,
    tension: f32,
) {
    let distance = (to - from).length();
    let sag = distance * tension * zoom;

    let mid_x = (from.x + to.x) * 0.5;
    let mid_y = from.y.max(to.y) + sag;

    let cp1 = Pos2::new(from.x + (mid_x - from.x) * 0.5, mid_y);
    let cp2 = Pos2::new(to.x - (to.x - mid_x) * 0.5, mid_y);

    let segments = 20;
    let points: Vec<Pos2> = (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let it = 1.0 - t;
            let p = from.to_vec2() * it * it * it
                + cp1.to_vec2() * 3.0 * it * it * t
                + cp2.to_vec2() * 3.0 * it * t * t
                + to.to_vec2() * t * t * t;
            p.to_pos2()
        })
        .collect();

    let shadow_offset = vec2(1.5 * zoom, 2.0 * zoom);
    let shadow_points: Vec<Pos2> = points.iter().map(|p| *p + shadow_offset).collect();
    painter.add(egui::Shape::line(
        shadow_points,
        Stroke::new(
            (thickness + 0.5) * zoom,
            Color32::from_rgba_unmultiplied(0, 0, 0, (color.a() / 2) as u8),
        ),
    ));
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(thickness * zoom, color),
    ));

    // --- Signal Flow Animation ---
    let time = painter.ctx().input(|i| i.time);
    let pulse_count = 3;
    for i in 0..pulse_count {
        let t_offset = (time as f32 * 0.5 + (i as f32 / pulse_count as f32)) % 1.0;
        let p_idx = (t_offset * segments as f32) as usize;
        if let Some(&p) = points.get(p_idx) {
            painter.circle_filled(p, 1.5 * zoom, Color32::from_rgba_unmultiplied(255, 255, 255, 180));
        }
    }
}
