//! WDF (Wave Digital Filter) Core — 物理回路シミュレーションの極北
//! 
//! # 憲法遵守
//! - a: 入射波 (Incident wave)
//! - b: 反射波 (Reflected wave)
//! - 電圧 V = (a + b) / 2
//! - 電流 I = (a - b) / (2 * R)

pub trait WdfNode {
    /// ポート・インピーダンス R の取得
    fn get_impedance(&self) -> f32;
    /// 反射波 b の取得
    fn get_reflected_wave(&mut self) -> f32;
    /// 入射波 a の設定
    fn set_incident_wave(&mut self, a: f32);
}

/// 抵抗 (Resistor)
pub struct WdfResistor {
    r: f32,
}
impl WdfResistor {
    pub fn new(r: f32) -> Self { Self { r } }
}
impl WdfNode for WdfResistor {
    fn get_impedance(&self) -> f32 { self.r }
    fn get_reflected_wave(&mut self) -> f32 { 0.0 }
    fn set_incident_wave(&mut self, _a: f32) {}
}

/// キャパシタ (Capacitor)
pub struct WdfCapacitor {
    r: f32,
    s: f32, // State (z^-1)
}
impl WdfCapacitor {
    pub fn new(c: f32, fs: f32) -> Self {
        Self { r: 1.0 / (2.0 * c * fs), s: 0.0 }
    }
}
impl WdfNode for WdfCapacitor {
    fn get_impedance(&self) -> f32 { self.r }
    fn get_reflected_wave(&mut self) -> f32 { self.s }
    fn set_incident_wave(&mut self, a: f32) { self.s = a; }
}

/// インダクタ (Inductor)
pub struct WdfInductor {
    r: f32,
    s: f32,
}
impl WdfInductor {
    pub fn new(l: f32, fs: f32) -> Self {
        Self { r: 2.0 * l * fs, s: 0.0 }
    }
}
impl WdfNode for WdfInductor {
    fn get_impedance(&self) -> f32 { self.r }
    fn get_reflected_wave(&mut self) -> f32 { -self.s }
    fn set_incident_wave(&mut self, a: f32) { self.s = a; }
}

/// 並列アダプタ (Parallel Adapter)
pub struct WdfParallel<N1: WdfNode, N2: WdfNode> {
    pub n1: N1,
    pub n2: N2,
    g: f32, // Impedance
    gamma: f32, // Reflection coefficient
}
impl<N1: WdfNode, N2: WdfNode> WdfParallel<N1, N2> {
    pub fn new(n1: N1, n2: N2) -> Self {
        let g1 = 1.0 / n1.get_impedance();
        let g2 = 1.0 / n2.get_impedance();
        let g = g1 + g2;
        let gamma = (g1 - g2) / g;
        Self { n1, n2, g: 1.0 / g, gamma }
    }
}
impl<N1: WdfNode, N2: WdfNode> WdfNode for WdfParallel<N1, N2> {
    fn get_impedance(&self) -> f32 { self.g }
    fn get_reflected_wave(&mut self) -> f32 {
        let b1 = self.n1.get_reflected_wave();
        let b2 = self.n2.get_reflected_wave();
        b1 + self.gamma * (b2 - b1)
    }
    fn set_incident_wave(&mut self, a: f32) {
        let b1 = self.n1.get_reflected_wave();
        let b2 = self.n2.get_reflected_wave();
        let a2 = b1 + self.gamma * (b2 - b1) - a;
        let a1 = a2 + b2 - b1;
        self.n1.set_incident_wave(a1);
        self.n2.set_incident_wave(a2);
    }
}

/// シリーズアダプタ (Series Adapter)
pub struct WdfSeries<N1: WdfNode, N2: WdfNode> {
    pub n1: N1,
    pub n2: N2,
    r: f32,
    gamma: f32,
}
impl<N1: WdfNode, N2: WdfNode> WdfSeries<N1, N2> {
    pub fn new(n1: N1, n2: N2) -> Self {
        let r1 = n1.get_impedance();
        let r2 = n2.get_impedance();
        let r = r1 + r2;
        let gamma = (r1 - r2) / r;
        Self { n1, n2, r, gamma }
    }
}
impl<N1: WdfNode, N2: WdfNode> WdfNode for WdfSeries<N1, N2> {
    fn get_impedance(&self) -> f32 { self.r }
    fn get_reflected_wave(&mut self) -> f32 {
        -(self.n1.get_reflected_wave() + self.n2.get_reflected_wave())
    }
    fn set_incident_wave(&mut self, a: f32) {
        let b1 = self.n1.get_reflected_wave();
        let b2 = self.n2.get_reflected_wave();
        let a1 = b1 - self.gamma * (a + b1 + b2);
        let a2 = b2 - (1.0 + self.gamma) * (a + b1 + b2);
        self.n1.set_incident_wave(a1);
        self.n2.set_incident_wave(a2);
    }
}
