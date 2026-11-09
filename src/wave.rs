// src/wave.rs — Fizikai Hullámcsomag és Interferencia Modell
//
// ψ(t) = A * exp(-γ * (t - t0)) * cos(2π * f * (t - t0) + φ)
// A Hope Library és a Spine kanonikus hullám-egyenlete.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WavePacket {
    pub amplitude: f64,
    pub frequency: f64,
    pub phase: f64,
    pub gamma: f64,      // Felejtési ráta / csillapítás
    pub t_birth_sec: f64,
}

impl WavePacket {
    pub fn new(amplitude: f64, frequency: f64, phase: f64, gamma: f64, t_birth_sec: f64) -> Self {
        Self {
            amplitude,
            frequency,
            phase,
            gamma,
            t_birth_sec,
        }
    }

    /// Kiszámítja az azonnali térerősséget ψ(t) adott t időpillanatban.
    #[inline]
    pub fn sample(&self, t_sec: f64) -> f64 {
        let dt = t_sec - self.t_birth_sec;
        if dt < 0.0 {
            return 0.0;
        }
        let decay = (-self.gamma * dt).exp();
        let osc = (2.0 * std::f64::consts::PI * self.frequency * dt + self.phase).cos();
        self.amplitude * decay * osc
    }

    /// Két érkező hullámcsomag szuperpozíciója adott t időpillanatban.
    #[allow(dead_code)]
    pub fn superpose(w1: &WavePacket, w2: &WavePacket, t_sec: f64) -> f64 {
        w1.sample(t_sec) + w2.sample(t_sec)
    }

    /// Csillapítás frissítése a közegben eltöltött idő arányában.
    #[inline]
    pub fn decayed_amplitude(&self, t_sec: f64) -> f64 {
        let dt = (t_sec - self.t_birth_sec).max(0.0);
        self.amplitude * (-self.gamma * dt).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_decay() {
        let w = WavePacket::new(1.0, 1000.0, 0.0, 10.0, 0.0);
        assert!((w.sample(0.0) - 1.0).abs() < 1e-6);
        // t = 0.1s -> exp(-10 * 0.1) = exp(-1) approx 0.367879
        let s = w.decayed_amplitude(0.1);
        assert!((s - 0.367879).abs() < 1e-4);
    }
}
