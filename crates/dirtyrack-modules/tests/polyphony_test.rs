#[cfg(test)]
mod tests {
    use dirtyrack_modules::midi::MidiCvModule;
    use dirtyrack_modules::runner::{Connection, GraphSnapshot, RackRunner};
    use dirtyrack_modules::signal::{RackDspNode, RackProcessContext, SeedScope};
    use dirtyrack_modules::vco::VcoModule;

    #[test]
    fn test_polyphonic_propagation() {
        let sample_rate = 44100.0;
        let mut runner = RackRunner::new(sample_rate, SeedScope::Global(0));

        let mut midi: Box<dyn RackDspNode> = Box::new(MidiCvModule::new(sample_rate));
        let vco: Box<dyn RackDspNode> = Box::new(VcoModule::new(sample_rate));

        // Setup notes (We need a way to trigger notes on the boxed trait or downcast)
        // For simplicity in test, let's cast back or use a helper
        let midi_ptr = midi.as_any_mut().downcast_mut::<MidiCvModule>().unwrap();
        midi_ptr.note_on(60, 60, 100); // C4
        midi_ptr.note_on(64, 64, 100); // E4
        midi_ptr.note_on(67, 67, 100); // G4

        let mut snapshot = GraphSnapshot {
            order: vec![0, 1],
            connections: vec![
                Connection {
                    from_module: 0,
                    from_port: 0,
                    to_module: 1,
                    to_port: 0,
                }, // MIDI 1V/OCT -> VCO 1V/OCT
            ],
            port_counts: vec![(0, 5), (4, 4)], // MIDI(0in, 5out), VCO(4in, 4out)
            node_ids: vec![100, 101],
            node_type_ids: vec!["midi".to_string(), "vco".to_string()],
            forward_edges: vec![],
            back_edges: vec![],
            modulations: vec![vec![], vec![]],
        };

        runner.apply_snapshot(&mut snapshot, vec![midi, vco]);

        let params = vec![
            vec![],                   // MIDI-CV (0 params)
            vec![5.0, 0.0, 0.0, 0.5], // VCO (FREQ, FINE, FM_AMT, PW)
        ];

        // Process a few samples
        for _ in 0..10 {
            runner.process_sample(&snapshot, &params);
        }

        // Check outputs of VCO
        // Channel 0 (C4)
        let out0 = runner.output_buffers[1][0 * 16 + 0]; // SINE ch0
                                                         // Channel 1 (E4)
        let out1 = runner.output_buffers[1][0 * 16 + 1]; // SINE ch1
                                                         // Channel 2 (G4)
        let out2 = runner.output_buffers[1][0 * 16 + 2]; // SINE ch2

        println!("name: \"MOD\", Poly outputs: {}, {}, {}", out0, out1, out2);

        // Verification: Check if frequencies are different
        // In a real test, we'd check the frequency of the generated sine,
        // but here we just check if the inputs were propagated.
        let vco_in0 = runner.input_buffers[1][0 * 16 + 0];
        let vco_in1 = runner.input_buffers[1][0 * 16 + 1];
        let vco_in2 = runner.input_buffers[1][0 * 16 + 2];

        assert_eq!(vco_in0, 0.0); // C4 = 0V
        assert!(vco_in1 > 0.3 && vco_in1 < 0.35); // E4 = (64-60)/12 = 0.333V
        assert!(vco_in2 > 0.5 && vco_in2 < 0.6); // G4 = (67-60)/12 = 0.583V
    }
}
