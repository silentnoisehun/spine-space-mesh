// src/modulator.rs — BPSK Fázis-Modulátor és Koherens Detektor
//
// 64-bites digitális mintázatokat kódol és dekódol hullámcsomagokká:
// bit 0 -> fázis 0.0 rad
// bit 1 -> fázis π rad (180 fok)
//
// A detektor méri a fázis-eltolódást és kiszámítja a Bit Error Rate-et (BER).

use crate::wave::WavePacket;

pub struct BpskModulator {
    pub carrier_freq: f64,
    pub symbol_period_sec: f64,
    pub packet_gamma: f64,
    pub nominal_amplitude: f64,
}

impl BpskModulator {
    pub fn new(carrier_freq: f64, symbol_period_sec: f64, packet_gamma: f64, nominal_amplitude: f64) -> Self {
        Self {
            carrier_freq,
            symbol_period_sec,
            packet_gamma,
            nominal_amplitude,
        }
    }

    /// 64 bites egész szám átalakítása 64 db BPSK hullámcsomaggá.
    pub fn encode_u64(&self, data: u64, start_time_sec: f64) -> Vec<WavePacket> {
        let mut packets = Vec::with_capacity(64);
        for bit_idx in 0..64 {
            let bit = (data >> (63 - bit_idx)) & 1;
            let phase = if bit == 0 { 0.0 } else { std::f64::consts::PI };
            let t_birth = start_time_sec + (bit_idx as f64) * self.symbol_period_sec;

            packets.push(WavePacket::new(
                self.nominal_amplitude,
                self.carrier_freq,
                phase,
                self.packet_gamma,
                t_birth,
            ));
        }
        packets
    }

    /// Visszaérkező hullámcsomag fázisának kinyerése és bitre fordítása.
    pub fn decode_packet(&self, packet: &WavePacket, t_eval_sec: f64) -> (u8, f64) {
        // Mintavételezés
        let s0 = packet.sample(t_eval_sec);
        // Negyed hullámhosszal későbbi minta a kvadratúra (Q) fázishoz:
        let t_quad = t_eval_sec + 1.0 / (4.0 * self.carrier_freq);
        let s_quad = packet.sample(t_quad);

        // Fázis becslése
        let estimated_phase = s_quad.atan2(s0);
        let cos_phase = estimated_phase.cos();

        // Döntési küszöb BPSK-nál:
        let bit = if cos_phase >= 0.0 { 0 } else { 1 };
        let confidence = cos_phase.abs();

        (bit, confidence)
    }

    /// Kiszámítja két 64-bites szó Hamming-távolságát és a Bit Error Rate-et (BER).
    pub fn compute_ber(original: u64, recovered: u64) -> (usize, f64) {
        let xor = original ^ recovered;
        let bit_errors = xor.count_ones() as usize;
        let ber = (bit_errors as f64) / 64.0;
        (bit_errors, ber)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpsk_encode_decode() {
        let mod_bpsk = BpskModulator::new(1000.0, 0.001, 0.0, 1.0);
        let test_word: u64 = 0xA5A5_5A5A_1234_5678;
        let packets = mod_bpsk.encode_u64(test_word, 0.0);

        let mut decoded: u64 = 0;
        for (i, p) in packets.iter().enumerate() {
            let (bit, conf) = mod_bpsk.decode_packet(p, p.t_birth_sec);
            assert!(conf > 0.99);
            decoded |= (bit as u64) << (63 - i);
        }

        let (errors, ber) = BpskModulator::compute_ber(test_word, decoded);
        assert_eq!(errors, 0);
        assert_eq!(ber, 0.0);
    }
}
