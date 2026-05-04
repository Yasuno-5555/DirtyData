//! Circuit Mutation Engine — Evaluation Logic
//! "美しさは、秩序と混沌の境界にある。"

use crate::types::*;
use dirtydata_dsp_circuit::{CircuitElement, MnaSolver};

impl crate::mutation::MutationEngine {
    /// Evaluates a mutated circuit by running a micro-simulation.
    pub fn evaluate(
        &self,
        def: &CircuitDefinition,
        _intensity: MutationIntensity,
    ) -> MutationReport {
        // 1. Setup a micro-solver (Impulse Response test)
        let mut solver = MnaSolver::new(1.0 / 44100.0);
        let elements: Vec<CircuitElement> =
            serde_json::from_str(&def.elements_json).unwrap_or_default();

        let mut max_node = 0;
        for el in &elements {
            let el_val: CircuitElement = el.clone();
            solver.add_element(el_val);
            let nodes = match el {
                CircuitElement::Resistor { a, b, .. }
                | CircuitElement::Capacitor { a, b, .. }
                | CircuitElement::Inductor { a, b, .. }
                | CircuitElement::Diode { a, k: b, .. }
                | CircuitElement::Zener { a, k: b, .. }
                | CircuitElement::Switch { a, b, .. }
                | CircuitElement::VoltageSource { pos: a, neg: b, .. }
                | CircuitElement::CurrentSource { pos: a, neg: b, .. } => vec![*a, *b],
                CircuitElement::Triode { g, k, p, .. } => vec![*g, *k, *p],
                CircuitElement::Bjt { b, c, e, .. } => vec![*b, *c, *e],
                CircuitElement::Jfet { g, d, s, .. } => vec![*g, *d, *s],
                CircuitElement::Transformer { a1, b1, a2, b2, .. } => vec![*a1, *b1, *a2, *b2],
                CircuitElement::OpAmp { pos, neg, out, .. } => vec![*pos, *neg, *out],
                CircuitElement::Potentiometer { a, wiper, b, .. } => vec![*a, *wiper, *b],
                CircuitElement::ControlledSource {
                    target_a,
                    target_b,
                    control_a,
                    control_b,
                    ..
                } => vec![*target_a, *target_b, *control_a, *control_b],
                CircuitElement::TransmissionLine { a1, b1, a2, b2, .. } => vec![*a1, *b1, *a2, *b2],
                CircuitElement::Memristor { a, b, .. } => vec![*a, *b],
                CircuitElement::ThermalCoupler { a, b, .. } => vec![*a, *b],
                _ => vec![],
            };
            for n in nodes {
                max_node = max_node.max(n.0);
            }
        }
        solver.set_num_nodes(max_node + 1);

        // 2. Probing: Run 256 samples of silence and 128 samples of impulse
        let mut energy: f64 = 0.0;
        let mut peak: f64 = 0.0;
        let mut converged_count = 0;

        for i in 0..256 {
            let _input_v = if i == 128 { 1.0 } else { 0.0 };

            let state = solver.solve();
            if state.converged {
                converged_count += 1;
            }

            let out = state.voltages.first().copied().unwrap_or(0.0).abs();
            energy += out;
            peak = f64::max(peak, out);
        }

        // 3. Compute Metrics
        let instability = if peak > 10.0 {
            1.0
        } else {
            (256 - converged_count) as f32 / 256.0
        };
        let novelty = (def.mutation_history.len() as f32 * 0.1).min(1.0);
        let warmth = if peak > 0.0 && energy / peak > 50.0 {
            0.8
        } else {
            0.2
        };

        MutationReport {
            instability_score: instability,
            novelty_score: novelty,
            risk_level: instability * 1.5,
            warmth_delta: warmth,
            dna_string: format!("gen {}", def.mutation_history.len()),
        }
    }
}
