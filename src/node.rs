// src/node.rs — Hajtott-Disszipatív Gross-Pitaevskii Attraktor Csomópont
//
// A csomópont NEM passzív tároló, hanem aktív vonzócentrum (Attraktor):
// 1. Lineáris pumpálás: kompenzálja a Spine csillapítását (γ).
// 2. Nemlineáris telítés (Kerr / Gross-Pitaevskii g|ψ|²): megakadályozza a robbanást,
//    és beállítja a stabil egyensúlyi amplitúdót: A* = sqrt((Pump - γ) / g).
// 3. Két-állapotú Fázis-Attraktor (Kuramoto / Adler szinkronizáció):
//    dφ/dt = -β * sin(2φ) -> Két stabil rögzítési pont: φ = 0 (bit 0) és φ = π (bit 1).
//
// Kibillentés (perturbáció) esetén a nemlineáris mező visszahúzza az állapotot!

use crate::wave::WavePacket;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MeshNode {
    pub id: usize,
    pub name: String,
    pub pump_gain: f64,          // Erősítési faktor (Gain > 1.0)
    pub saturation_g: f64,       // Nemlineáris Kerr-állandó (g|ψ|²)
    pub phase_restoring_beta: f64,// Fázis-visszahúzó erő (Kuramoto-csatolás)
    pub target_amplitude: f64,   // Attraktor cél-amplitúdó (A*)
    pub noise_floor: f64,
}

impl MeshNode {
    pub fn new(
        id: usize,
        name: &str,
        pump_gain: f64,
        saturation_g: f64,
        phase_restoring_beta: f64,
        target_amplitude: f64,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            pump_gain,
            saturation_g,
            phase_restoring_beta,
            target_amplitude,
            noise_floor: 1e-4,
        }
    }

    /// Beérkező hullámok átalakítása az attraktor mezőn keresztül.
    /// Szigorú Zero-I/O: Az átmenet azonnali, nincs tárolt belső állapot.
    pub fn process_and_forward(&self, incoming: Vec<WavePacket>, now_sec: f64) -> Vec<WavePacket> {
        let mut outgoing = Vec::new();

        for in_pkt in incoming {
            let in_amp = in_pkt.decayed_amplitude(now_sec);
            if in_amp < self.noise_floor {
                // A jel beleveszett a zajba, nem tudja megragadni az attraktor
                continue;
            }

            // 1. Gross-Pitaevskii Amplitúdó Attraktor (Hajtott-disszipatív telítés):
            // A_next = (Pump * A_in) / sqrt(1 + g * A_in^2)
            let pumped = self.pump_gain * in_amp;
            let stabilized_amp = pumped / (1.0 + self.saturation_g * in_amp * in_amp).sqrt();

            // 2. Két-állapotú Fázis Attraktor (Kuramoto kettős potenciálvölgy):
            // dφ = -β * sin(2φ)
            // Ha φ közel van 0-hoz: dφ visszahúzza 0-ra.
            // Ha φ közel van π-hez: dφ visszahúzza π-re.
            let phase_normalized = (in_pkt.phase % (2.0 * std::f64::consts::PI) + 2.0 * std::f64::consts::PI) % (2.0 * std::f64::consts::PI);
            let restoring_force = -self.phase_restoring_beta * (2.0 * phase_normalized).sin();
            let stabilized_phase = phase_normalized + restoring_force;

            let forwarded = WavePacket::new(
                stabilized_amp,
                in_pkt.frequency,
                stabilized_phase,
                0.01, // minimális belső csillapítás
                now_sec,
            );
            outgoing.push(forwarded);
        }

        outgoing
    }
}
