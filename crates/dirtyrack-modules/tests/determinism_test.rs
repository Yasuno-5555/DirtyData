#[cfg(test)]
mod tests {
    use blake3::Hasher;
    use dirtyrack_modules::runner::{Connection, GraphSnapshot, RackRunner};
    use dirtyrack_modules::signal::{RackDspNode, SeedScope};
    use dirtyrack_modules::vcf::VcfModule;
    use dirtyrack_modules::vco::VcoModule;

    #[test]
    fn test_cross_platform_determinism() {
        let sample_rate = 44100.0;
        let mut runner = RackRunner::new(sample_rate, SeedScope::Global(0xDEADBEEF));

        let vco = Box::new(VcoModule::new(sample_rate));
        let vcf = Box::new(VcfModule::new(sample_rate));

        let snapshot = GraphSnapshot {
            order: vec![0, 1],
            connections: vec![
                Connection {
                    from_module: 0,
                    from_port: 0,
                    to_module: 1,
                    to_port: 0,
                }, // VCO SINE -> VCF IN
            ],
            port_counts: vec![(4, 4), (4, 4)],
            node_ids: vec![1, 2],
        };

        runner.apply_snapshot(snapshot.clone(), vec![vco, vcf]);

        // Params:
        // VCO: FREQ=0.0, FINE=0.0, FM=0.0, PW=0.5
        // VCF: CUTOFF=5.0, RES=0.5, DRIVE=0.0, TYPE=0.0
        let params = vec![vec![0.0, 0.0, 0.0, 0.5], vec![5.0, 0.5, 0.0, 0.0]];

        let mut hasher = Hasher::new();

        // Process 1 second (44100 samples)
        for _ in 0..44100 {
            runner.process_sample(&snapshot, &params);

            // Hash the outputs of VCF (module 1, port 0)
            let out = runner.output_buffers[1][0];
            hasher.update(&out.to_le_bytes());
        }

        let hash = hasher.finalize();
        let hash_hex = hash.to_hex();

        println!("Determinism Hash (44.1k samples): {}", hash_hex);

        // This is the "Golden Hash" that must match across Mac, Linux, and Windows.
        let expected_hash = "f15e9a0a538011fdf2d2fe1b5637b1922be2ed15b2771f52876c5a344e2a5e9f";
        assert_eq!(hash_hex.as_str(), expected_hash);
    }
}
