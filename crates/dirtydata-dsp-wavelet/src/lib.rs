//! Wavelet Analysis Civilization

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WaveletType {
    Haar,
    Daubechies2,
    Daubechies4,
    Daubechies6,
    Daubechies8,
    Daubechies10,
    Coiflet1,
    Symlet4,
}

impl WaveletType {
    /// Returns the decomposition filters (scaling, wavelet)
    pub fn filters(&self) -> (&'static [f32], &'static [f32]) {
        match self {
            WaveletType::Haar => {
                const S: f32 = 0.70710678118; // 1/sqrt(2)
                (&[S, S], &[S, -S])
            }
            WaveletType::Daubechies2 => {
                // Same as Haar, just a common alias
                const S: f32 = 0.70710678118;
                (&[S, S], &[S, -S])
            }
            WaveletType::Daubechies4 => {
                const H: &[f32] = &[
                    0.4829629131, 0.8365163037, 0.2241438680, -0.1294095226
                ];
                const G: &[f32] = &[
                    -0.1294095226, -0.2241438680, 0.8365163037, -0.4829629131
                ];
                (H, G)
            }
            WaveletType::Daubechies6 => {
                const H: &[f32] = &[
                    0.3326705529, 0.8068915093, 0.4598775021, -0.1350110200, -0.0854412739, 0.0352262919
                ];
                const G: &[f32] = &[
                    0.0352262919, 0.0854412739, -0.1350110200, -0.4598775021, 0.8068915093, -0.3326705529
                ];
                (H, G)
            }
            WaveletType::Daubechies8 => {
                const H: &[f32] = &[
                    0.2303778133, 0.7148465705, 0.6308807679, -0.0279837694, -0.1870348117, 0.0308413818, 0.0328830116, -0.0105974017
                ];
                const G: &[f32] = &[
                    -0.0105974017, -0.0328830116, 0.0308413818, 0.1870348117, -0.0279837694, -0.6308807679, 0.7148465705, -0.2303778133
                ];
                (H, G)
            }
            WaveletType::Daubechies10 => {
                const H: &[f32] = &[
                    0.1601023979, 0.6038292697, 0.7243025286, 0.1384281459, -0.2422948870, -0.0322448695, 0.0775714938, -0.0062414902, -0.0125807519, 0.0033357252
                ];
                const G: &[f32] = &[
                     0.0033357252, 0.0125807519, -0.0062414902, -0.0775714938, -0.0322448695, 0.2422948870, 0.1384281459, -0.7243025286, 0.6038292697, -0.1601023979
                ];
                (H, G)
            }
            WaveletType::Coiflet1 => {
                const H: &[f32] = &[-0.0156557281, -0.0727326195, 0.3848648469, 0.8525720202, 0.3378976625, -0.0727326195];
                const G: &[f32] = &[-0.0727326195, -0.3378976625, 0.8525720202, -0.3848648469, -0.0727326195, 0.0156557281];
                (H, G)
            }
            WaveletType::Symlet4 => {
                const H: &[f32] = &[-0.0757657148, -0.0296355276, 0.4976186676, 0.8037387518, 0.2978577956, -0.0992195436, -0.0126039673, 0.0322231006];
                const G: &[f32] = &[-0.0322231006, -0.0126039673, 0.0992195436, 0.2978577956, -0.8037387518, 0.4976186676, 0.0296355276, -0.0757657148];
                (H, G)
            }
        }
    }

    /// Returns the reconstruction filters (scaling, wavelet).
    pub fn reconstruction_filters(&self) -> (&'static [f32], &'static [f32]) {
        self.filters()
    }
}

pub struct Dwt {
    wavelet: WaveletType,
}

impl Dwt {
    pub fn new(wavelet: WaveletType) -> Self {
        Self { wavelet }
    }

    /// Performs one level of discrete wavelet transform.
    pub fn decompose(&self, input: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let (h, g) = self.wavelet.filters();
        let n = input.len();
        let mut approx = Vec::with_capacity(n / 2);
        let mut detail = Vec::with_capacity(n / 2);

        for i in (0..n).step_by(2) {
            let mut a = 0.0;
            let mut d = 0.0;
            for j in 0..h.len() {
                let idx = if i + j < n { i + j } else { (i + j) % n };
                a += input[idx] * h[j];
                d += input[idx] * g[j];
            }
            approx.push(a);
            detail.push(d);
        }

        (approx, detail)
    }

    /// Performs one level of inverse discrete wavelet transform.
    pub fn reconstruct(&self, approx: &[f32], detail: &[f32]) -> Vec<f32> {
        let (h, g) = self.wavelet.reconstruction_filters();
        let n = approx.len() * 2;
        let mut output = vec![0.0; n];

        for i in 0..approx.len() {
            let base_idx = 2 * i;
            for j in 0..h.len() {
                let idx = (base_idx + j) % n;
                output[idx] += approx[i] * h[j] + detail[i] * g[j];
            }
        }

        output
    }
    /// Hard thresholding for wavelet denoising.
    pub fn threshold_hard(data: &mut [f32], threshold: f32) {
        for val in data.iter_mut() {
            if val.abs() < threshold {
                *val = 0.0;
            }
        }
    }

    /// Soft thresholding for wavelet denoising.
    pub fn threshold_soft(data: &mut [f32], threshold: f32) {
        for val in data.iter_mut() {
            let s = val.signum();
            let v = val.abs() - threshold;
            *val = s * v.max(0.0);
        }
    }
}

pub struct MultiLevelDwt {
    dwt: Dwt,
    levels: usize,
}

impl MultiLevelDwt {
    pub fn new(wavelet: WaveletType, levels: usize) -> Self {
        Self { dwt: Dwt::new(wavelet), levels }
    }

    /// Performs multi-level decomposition.
    pub fn decompose(&self, input: &[f32]) -> (Vec<Vec<f32>>, Vec<f32>) {
        let mut current_approx = input.to_vec();
        let mut details = Vec::with_capacity(self.levels);

        for _ in 0..self.levels {
            if current_approx.len() < 2 { break; }
            let (approx, detail) = self.dwt.decompose(&current_approx);
            details.push(detail);
            current_approx = approx;
        }

        (details, current_approx)
    }

    /// Performs multi-level reconstruction.
    pub fn reconstruct(&self, details: &[Vec<f32>], final_approx: &[f32]) -> Vec<f32> {
        let mut current_approx = final_approx.to_vec();

        for detail in details.iter().rev() {
            current_approx = self.dwt.reconstruct(&current_approx, detail);
        }

        current_approx
    }
}

/// Wavelet Packet Decomposition (WPD) Tree.
/// Unlike DWT which only decomposes the approximation, WPD decomposes both approx and detail.
pub struct WaveletPacketTree {
    pub wavelet: WaveletType,
    pub levels: usize,
    pub nodes: Vec<Vec<f32>>, // Flattened tree representation
}

impl WaveletPacketTree {
    pub fn new(wavelet: WaveletType, levels: usize) -> Self {
        Self { wavelet, levels, nodes: Vec::new() }
    }

    pub fn decompose(&mut self, input: &[f32]) {
        let dwt = Dwt::new(self.wavelet);
        let total_nodes = (2usize.pow(self.levels as u32 + 1)) - 1;
        self.nodes = vec![Vec::new(); total_nodes];
        self.nodes[0] = input.to_vec();

        for l in 0..self.levels {
            let start_node = 2usize.pow(l as u32) - 1;
            let end_node = 2usize.pow(l as u32 + 1) - 1;
            
            for i in start_node..end_node {
                if self.nodes[i].len() >= 2 {
                    let (approx, detail) = dwt.decompose(&self.nodes[i]);
                    self.nodes[2 * i + 1] = approx;
                    self.nodes[2 * i + 2] = detail;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haar_roundtrip() {
        let dwt = Dwt::new(WaveletType::Haar);
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let (approx, detail) = dwt.decompose(&input);
        let output = dwt.reconstruct(&approx, &detail);
        
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn test_db4_roundtrip() {
        let dwt = Dwt::new(WaveletType::Daubechies4);
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let (approx, detail) = dwt.decompose(&input);
        let output = dwt.reconstruct(&approx, &detail);
        
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn test_multilevel_roundtrip() {
        let mldwt = MultiLevelDwt::new(WaveletType::Haar, 3);
        let input = vec![1.0; 16];
        let (details, approx) = mldwt.decompose(&input);
        assert_eq!(details.len(), 3);
        let output = mldwt.reconstruct(&details, &approx);
        
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn test_coiflet_roundtrip() {
        let dwt = Dwt::new(WaveletType::Coiflet1);
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let (approx, detail) = dwt.decompose(&input);
        let output = dwt.reconstruct(&approx, &detail);
        
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }
}
