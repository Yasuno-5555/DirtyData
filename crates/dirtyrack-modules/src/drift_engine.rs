//! Drift Engine — Deterministic Analog Instability
//!
//! 1/f (ピンクノイズ) 的な低周波の揺らぎを決定論的に生成する。
//! アナログ回路の熱ドリフトや電源ノイズをシミュレートする。

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// 決定論的なドリフト・ジェネレーター
pub struct DriftGenerator {
    /// 低速な揺らぎのためのマルチオクターブ・ノイズ
    octaves: [Octave; 6],
    current_value: f32,
}

struct Octave {
    val1: f32,
    val2: f32,
    phase: u32,
    period: u32,
}

impl DriftGenerator {
    pub fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut octaves = [(); 6].map(|_| Octave {
            val1: rng.gen_range(-1.0..1.0),
            val2: rng.gen_range(-1.0..1.0),
            phase: 0,
            period: 0,
        });

        // 各オクターブの周期をずらす (2^n ではない素数ベースの周期が楽器っぽい)
        let periods = [44100, 23456, 12345, 6789, 3456, 1234];
        for i in 0..6 {
            octaves[i].period = periods[i];
            octaves[i].phase = rng.gen_range(0..periods[i]);
        }

        Self {
            octaves,
            current_value: 0.0,
        }
    }

    /// 次のサンプル値を計算 (決定論的)
    pub fn next(&mut self, rng_source: &mut ChaCha8Rng) -> f32 {
        let mut sum = 0.0;
        let mut weight = 1.0;
        let mut total_weight = 0.0;

        for oct in &mut self.octaves {
            oct.phase += 1;
            if oct.phase >= oct.period {
                oct.phase = 0;
                oct.val1 = oct.val2;
                oct.val2 = rng_source.gen_range(-1.0..1.0);
            }

            // 線形補間 (楽器的な柔らかさのために)
            let t = oct.phase as f32 / oct.period as f32;
            let smooth_t = t * t * (3.0 - 2.0 * t); // Hermite 補間
            let val = oct.val1 + (oct.val2 - oct.val1) * smooth_t;

            sum += val * weight;
            total_weight += weight;
            weight *= 0.5; // 1/f 特性 (高域ほど減衰)
        }

        self.current_value = sum / total_weight;
        self.current_value
    }
}

/// 16ボイス分のドリフトを管理する
pub struct VoiceDriftEngine {
    generators: Vec<DriftGenerator>,
    rng: ChaCha8Rng,
}

impl VoiceDriftEngine {
    pub fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut generators = Vec::with_capacity(16);
        for _ in 0..16 {
            generators.push(DriftGenerator::new(rng.gen()));
        }
        Self { generators, rng }
    }

    pub fn process(&mut self, outputs: &mut [f32; 16]) {
        for i in 0..16 {
            outputs[i] = self.generators[i].next(&mut self.rng);
        }
    }

    pub fn current_drift(&self) -> [f32; 16] {
        let mut out = [0.0; 16];
        for i in 0..16 {
            out[i] = self.generators[i].current_value;
        }
        out
    }
}
