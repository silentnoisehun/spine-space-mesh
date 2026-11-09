// src/spine.rs — A Dinamikus Spine Busz és Késleltető Művonal (Delay-Line)
//
// A fizikai közeg, amin a hullámok terjednek a csomópontok között.
// Szigorú Zero-I/O feltétel: Az adat kizárólag in-flight (tranzitban) létezik.
// Ha a csatorna szétkapcsol, az in-flight hullámcsomag disszipálódik (hővé válik).

use crate::wave::WavePacket;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpineSlot {
    pub packet: WavePacket,
    pub departure_time: f64,
    pub arrival_time: f64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SpineBus {
    pub name: String,
    pub flight_delay_sec: f64,
    pub attenuation_per_sec: f64,
    pub is_connected: bool,
    in_flight: Vec<SpineSlot>,
}

impl SpineBus {
    pub fn new(name: &str, flight_delay_sec: f64, attenuation_per_sec: f64) -> Self {
        Self {
            name: name.to_string(),
            flight_delay_sec,
            attenuation_per_sec,
            is_connected: true,
            in_flight: Vec::new(),
        }
    }

    /// Hullámcsomag kibocsátása a buszra (in-flight állapotba helyezés).
    pub fn transmit(&mut self, packet: WavePacket, now_sec: f64) {
        if !self.is_connected {
            // Ha a busz le van kapcsolva, a csomag azonnal elveszik a vákuumban
            return;
        }

        let arrival = now_sec + self.flight_delay_sec;
        self.in_flight.push(SpineSlot {
            packet,
            departure_time: now_sec,
            arrival_time: arrival,
        });
    }

    /// Érkező hullámok kinyerése, amelyek elérték a célállomást.
    /// Csak azok jönnek ki, amelyeknek t >= arrival_time.
    pub fn receive(&mut self, now_sec: f64) -> Vec<WavePacket> {
        if !self.is_connected {
            self.in_flight.clear();
            return Vec::new();
        }

        let mut ready = Vec::new();
        let mut still_flying = Vec::new();

        for slot in self.in_flight.drain(..) {
            if now_sec >= slot.arrival_time {
                // A csomag megérkezett a célállomásra
                let transit_time = slot.arrival_time - slot.departure_time;
                let mut arrived_packet = slot.packet;
                // A busz saját közege is csillapít:
                let bus_decay = (-self.attenuation_per_sec * transit_time).exp();
                arrived_packet.amplitude *= bus_decay;
                ready.push(arrived_packet);
            } else {
                still_flying.push(slot);
            }
        }

        self.in_flight = still_flying;
        ready
    }

    /// Az adott pillanatban a térben (in-flight tranzitban) lévő csomagok száma.
    #[inline]
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }

    /// Kapcsolat megszakítása (Negatív kontroll méréshez).
    pub fn disconnect(&mut self) {
        self.is_connected = false;
        self.in_flight.clear();
    }

    /// Kapcsolat helyreállítása.
    #[allow(dead_code)]
    pub fn connect(&mut self) {
        self.is_connected = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spine_delay_and_disconnect() {
        let mut bus = SpineBus::new("test_bus", 0.001, 1.0); // 1 ms delay
        let p = WavePacket::new(1.0, 100.0, 0.0, 0.0, 0.0);

        bus.transmit(p, 0.0);
        assert_eq!(bus.in_flight_count(), 1);

        // 0.5 ms után még repül:
        let r0 = bus.receive(0.0005);
        assert!(r0.is_empty());
        assert_eq!(bus.in_flight_count(), 1);

        // 1.0 ms után megérkezik:
        let r1 = bus.receive(0.0010);
        assert_eq!(r1.len(), 1);
        assert_eq!(bus.in_flight_count(), 0);

        // Lekapcsolt teszt:
        bus.disconnect();
        bus.transmit(p, 0.002);
        assert_eq!(bus.in_flight_count(), 0);
        let r2 = bus.receive(0.0035);
        assert!(r2.is_empty());
    }
}
