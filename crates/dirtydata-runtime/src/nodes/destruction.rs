use super::base::*;
use dirtydata_core::types::ConfigSnapshot;
use dirtydata_dsp_destruction::*;

#[derive(Clone)]
pub struct BitCrushNode {
    inner: BitCrush,
}

impl BitCrushNode {
    pub fn new() -> Self {
        Self {
            inner: BitCrush::new(),
        }
    }
}

impl DspNode for BitCrushNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let bits = config.get("bits").and_then(|v| v.as_float()).unwrap_or(8.0) as f32;
        let sr_div = config
            .get("sr_div")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;

        for (i, output) in outputs.iter_mut().enumerate() {
            let input = if i * 2 < inputs.len() {
                inputs[i * 2]
            } else {
                0.0
            };
            let out = self.inner.process(input, bits, sr_div);
            *output = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct WaveShaperNode {
    inner: WaveShaper,
}

impl WaveShaperNode {
    pub fn new() -> Self {
        Self {
            inner: WaveShaper {},
        }
    }
}

impl DspNode for WaveShaperNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        _ctx: &ProcessContext,
    ) {
        let amount = config
            .get("amount")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;

        for (i, output) in outputs.iter_mut().enumerate() {
            let input = if i * 2 < inputs.len() {
                inputs[i * 2]
            } else {
                0.0
            };
            let out = self.inner.process(input, amount);
            *output = [out, out];
        }
    }
}

#[derive(Clone)]
pub struct PllNode {
    inner: Pll,
}

impl PllNode {
    pub fn new() -> Self {
        Self { inner: Pll::new() }
    }
}

impl DspNode for PllNode {
    fn process(
        &mut self,
        inputs: &[f32],
        outputs: &mut [[f32; 2]],
        config: &ConfigSnapshot,
        ctx: &ProcessContext,
    ) {
        let lock_speed = config
            .get("lock_speed")
            .and_then(|v| v.as_float())
            .unwrap_or(0.1) as f32;

        for (i, output) in outputs.iter_mut().enumerate() {
            let input = if i * 2 < inputs.len() {
                inputs[i * 2]
            } else {
                0.0
            };
            let out = self.inner.process(input, lock_speed, ctx.sample_rate);
            *output = [out, out];
        }
    }
}
