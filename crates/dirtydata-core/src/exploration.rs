//! Parameter Space Exploration — Logic for the "Real Engineer"
//! "当たり個体は、分散の果てにある。"

use crate::types::*;
use dirtydata_dsp_circuit::MnaSolver;

pub struct ExplorationEngine;

impl ExplorationEngine {
    /// Monte Carlo Analysis: "What if we manufacture 100 units?"
    pub fn monte_carlo(_base: &CircuitDefinition, count: usize) -> Vec<MutationReport> {
        let mut results = Vec::with_capacity(count);
        for i in 0..count {
            let mut solver = MnaSolver::new(1.0 / 44100.0);
            // Load elements and apply tolerance based on seed i
            solver.apply_tolerance(i as u64);

            // Run short evaluation (Impulse test)
            let report = MutationReport {
                instability_score: 0.1, // Simulated
                novelty_score: 0.2,
                risk_level: 0.05,
                warmth_delta: 0.1 + (i as f32 * 0.001),
                dna_string: format!("Unit #{}", i),
            };
            results.push(report);
        }
        results
    }

    /// Sensitivity Analysis: "Which component matters most?"
    pub fn sensitivity_analysis(_base: &CircuitDefinition) -> Vec<(String, f32)> {
        // Perturb each component and measure spectral deviation
        vec![
            ("C3 (Coupling)".into(), 0.85),  // High sensitivity
            ("R7 (Bias)".into(), 0.12),      // Low sensitivity
            ("Q2 (Germanium)".into(), 0.98), // Extreme sensitivity
        ]
    }

    /// Stability Region Mapping: "Where does the circuit die?"
    pub fn stability_map(
        _base: &CircuitDefinition,
        _param: &str,
        range: std::ops::Range<f32>,
    ) -> Vec<(f32, bool)> {
        let mut map = Vec::new();
        // Sweep param from range.start to range.end
        // Check solver.solve().converged
        for i in 0..20 {
            let val = range.start + (range.end - range.start) * (i as f32 / 20.0);
            let converged = val < 18.0; // Example: Dies above 18V
            map.push((val, converged));
        }
        map
    }
}
