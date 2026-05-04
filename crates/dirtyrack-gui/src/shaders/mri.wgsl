//! MRI WGSL Shader — 医療グレードのパッチ可視化
//! 
//! # 憲法遵守
//! - Gehennaエンジンからの鑑識データを色に変換。
//! - Clipping -> Red Pulse
//! - Energy -> Orange Heat
//! - DC Offset -> Purple Aura

struct ForensicStats {
    peak: f32,
    clipping: f32,
    dc: f32,
    energy: f32,
}

@group(0) @binding(0) var<storage, read> stats_buffer: array<ForensicStats>;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) module_idx: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) module_idx: u32,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.uv = model.uv;
    out.module_idx = model.module_idx;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let s = stats_buffer[in.module_idx];
    
    // Base Color (Empty)
    var final_color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    
    // 1. Clipping (Red Glow)
    let clip_intensity = min(s.clipping / 1000.0, 1.0);
    if (clip_intensity > 0.01) {
        let dist = length(in.uv - vec2<f32>(0.5, 0.5));
        let glow = exp(-dist * 4.0) * clip_intensity;
        final_color += vec4<f32>(1.0, 0.0, 0.0, glow);
    }
    
    // 2. Energy (Orange Heat)
    let energy_intensity = min(s.energy / 50.0, 1.0);
    final_color += vec4<f32>(1.0, 0.5, 0.0, energy_intensity * 0.2);
    
    // 3. DC Drift (Purple Magnetic Field)
    let dc_intensity = min(abs(s.dc) / 2.0, 1.0);
    if (dc_intensity > 0.1) {
        let pulse = 0.5 + 0.5 * sin(in.uv.x * 20.0 + dc_intensity * 10.0);
        final_color += vec4<f32>(0.8, 0.0, 1.0, pulse * dc_intensity * 0.3);
    }
    
    return final_color;
}
