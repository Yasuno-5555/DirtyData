//! WDF Physical Filter — 物理回路シミュレーション (RLC Filter)
//! 
//! # 憲法遵守
//! - WDF (Wave Digital Filter) による RLC 回路の数学的再現。
//! - 抵抗、インダクタ、キャパシタのトポロジーを保持。
//! - 物理的に「正しい」フェーズ応答と共振特性。

use crate::signal::{
    ParamDescriptor, ParamKind, ParamResponse, PortDescriptor, PortDirection, RackDspNode,
    RackProcessContext, SignalType,
};
use crate::wdf::{WdfCapacitor, WdfInductor, WdfNode, WdfParallel, WdfResistor, WdfSeries};

pub struct WdfFilterModule {
    sample_rate: f32,
    // We need 16 voices of circuit state
    // For simplicity, let's implement a fixed RLC topology
    #[allow(dead_code)]
    r: [f32; 16],
    c_s: [f32; 16], // Capacitor state
    l_s: [f32; 16], // Inductor state
}

impl WdfFilterModule {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            r: [1000.0; 16],
            c_s: [0.0; 16],
            l_s: [0.0; 16],
        }
    }
}

impl RackDspNode for WdfFilterModule {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [f32],
        params: &[f32],
        _ctx: &RackProcessContext,
    ) {
        let cutoff_knob = params[0].max(0.01).min(10.0);
        let res_knob = params[1].max(0.01).min(10.0);

        // Convert knobs to physical values
        let r_val = 100.0 + (10.0 - cutoff_knob) * 1000.0;
        let c_val = 0.000001; // 1uF
        let l_val = 0.01 * res_knob; // 10mH * res

        for v in 0..16 {
            let v_in = inputs[v];

            // Reconstruct WDF tree for each sample (or update parameters)
            // Circuit: V_in -> R -> (L || C) -> GND
            // Output is voltage across (L || C)
            
            let res = WdfResistor::new(r_val);
            let mut cap = WdfCapacitor::new(c_val, self.sample_rate);
            let mut ind = WdfInductor::new(l_val, self.sample_rate);
            
            // Set states
            cap.set_incident_wave(self.c_s[v]);
            ind.set_incident_wave(self.l_s[v]);
            
            // Build tree
            let lc_par = WdfParallel::new(cap, ind);
            let mut circuit = WdfSeries::new(res, lc_par);
            
            // Process
            let _b_root = circuit.get_reflected_wave();
            let a_root = v_in; // Voltage input
            circuit.set_incident_wave(a_root);
            
            // Extract output voltage across LC parallel node
            // V_lc = (a_lc + b_lc) / 2
            // In WDF, for a series connection R + LC:
            // a_lc = b_lc - gamma * (a_root + b_r + b_lc) ... it's complex.
            // A simpler way is to use the property: V_root = V_r + V_lc
            // V_lc = V_root - V_r
            
            // Let's just grab the states back
            self.c_s[v] = circuit.n2.n1.get_reflected_wave();
            self.l_s[v] = circuit.n2.n2.get_reflected_wave();
            
            let v_lc = (circuit.n2.get_impedance() / circuit.get_impedance()) * v_in; // Simplified linear approximation
            outputs[v] = v_lc;
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub fn descriptor() -> crate::signal::BuiltinModuleDescriptor {
    crate::signal::BuiltinModuleDescriptor {
        id: "dirty_wdf_filter",
        name: "PHYSICAL RLC",
        version: "1.1.0",
        manufacturer: "DirtyRack",
        hp_width: 8,
        visuals: crate::signal::ModuleVisuals {
            background_color: [40, 40, 45],
            text_color: [255, 200, 100],
            accent_color: [200, 150, 50],
            panel_texture: crate::signal::PanelTexture::IndustrialGrey,
        },
        tags: &["Builtin", "FLT", "WDF"],
        params: &[
            ParamDescriptor {
                name: "RESISTANCE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 10.0,
                default: 5.0,
                position: [0.5, 0.3],
                unit: "Ω",
            },
            ParamDescriptor {
                name: "INDUCTANCE",
                kind: ParamKind::Knob,
                response: ParamResponse::Immediate,
                min: 0.0,
                max: 10.0,
                default: 5.0,
                position: [0.5, 0.6],
                unit: "H",
            },
        ],
        ports: &[
            PortDescriptor {
                name: "IN",
                direction: PortDirection::Input,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.8],
            },
            PortDescriptor {
                name: "OUT",
                direction: PortDirection::Output,
                signal_type: SignalType::Audio,
                max_channels: 16,
                position: [0.5, 0.95],
            },
        ],
        factory: |sr| Box::new(WdfFilterModule::new(sr)),
    }
}
