// src/mesh.rs — Hálózati Topológia és Keringtető Motor
//
// Összekapcsolja a csomópontokat és a Spine buszokat topológiákba:
// 1. Gyűrű (Ring): N csomópont zárt hurokban.
// 2. Kétcsomópontos Vonal / Rezonátor (Line): A és B oda-vissza verődéssel.
// 3. Negatív Kontroll: A Spine csatornák fizikailag szétkapcsolva.

use crate::node::MeshNode;
use crate::spine::SpineBus;
use crate::wave::WavePacket;

pub enum TopologyType {
    Ring,
    Line,
}

pub struct StepResult {
    pub node0_arrived: Vec<WavePacket>,
}

#[allow(dead_code)]
pub struct SpaceMesh {
    pub topology: TopologyType,
    pub nodes: Vec<MeshNode>,
    pub buses: Vec<SpineBus>,
}

impl SpaceMesh {
    /// Gyűrű topológia létrehozása (pl. N csomópont, köztük N Spine busz).
    pub fn new_ring(num_nodes: usize, delay_per_hop: f64, gain_per_node: f64, bus_gamma: f64) -> Self {
        let mut nodes = Vec::new();
        let mut buses = Vec::new();

        for i in 0..num_nodes {
            nodes.push(MeshNode::new(
                i,
                &format!("Node_{i}"),
                gain_per_node,
                0.012, // saturation_g: Gross-Pitaevskii attraktor telítés
                0.050, // phase_restoring_beta: Kuramoto fázis-visszahúzó erő
                1.000, // target_amplitude
            ));

            let next = (i + 1) % num_nodes;
            buses.push(SpineBus::new(
                &format!("Spine_{i}_to_{next}"),
                delay_per_hop,
                bus_gamma,
            ));
        }

        Self {
            topology: TopologyType::Ring,
            nodes,
            buses,
        }
    }

    /// Kétcsomópontos rezonátor (A <-> B).
    pub fn new_line(delay_hop: f64, gain: f64, bus_gamma: f64) -> Self {
        let nodes = vec![
            MeshNode::new(0, "Node_A", gain, 0.012, 0.050, 1.000),
            MeshNode::new(1, "Node_B", gain, 0.012, 0.050, 1.000),
        ];

        let buses = vec![
            SpineBus::new("Spine_A_to_B", delay_hop, bus_gamma),
            SpineBus::new("Spine_B_to_A", delay_hop, bus_gamma),
        ];

        Self {
            topology: TopologyType::Line,
            nodes,
            buses,
        }
    }

    /// Hullámcsomag injektálása adott csomópontra (a kiinduló Spine-ra helyezi).
    pub fn inject(&mut self, node_idx: usize, packet: WavePacket, now_sec: f64) {
        if node_idx < self.buses.len() {
            self.buses[node_idx].transmit(packet, now_sec);
        }
    }

    /// Egy diszkrét idő-lépés lefuttatása: a buszok átadják a csomagokat a csomópontoknak,
    /// a csomópontok erősítik és fázis-illesztik, majd továbbadják a következő buszra.
    pub fn step(&mut self, now_sec: f64) -> StepResult {
        let n = self.nodes.len();
        let mut node0_packets = Vec::new();

        match self.topology {
            TopologyType::Ring => {
                // Minden i. busz a (i+1)%n csomópontba érkezik
                for i in 0..n {
                    let next_node = (i + 1) % n;
                    let arrived = self.buses[i].receive(now_sec);
                    if !arrived.is_empty() {
                        if next_node == 0 {
                            node0_packets.extend(arrived.clone());
                        }
                        let forwarded = self.nodes[next_node].process_and_forward(arrived, now_sec);
                        for pkt in forwarded {
                            self.buses[next_node].transmit(pkt, now_sec);
                        }
                    }
                }
            }
            TopologyType::Line => {
                // Bus 0: A -> B
                let to_b = self.buses[0].receive(now_sec);
                if !to_b.is_empty() {
                    let from_b = self.nodes[1].process_and_forward(to_b, now_sec);
                    for pkt in from_b {
                        self.buses[1].transmit(pkt, now_sec);
                    }
                }

                // Bus 1: B -> A
                let to_a = self.buses[1].receive(now_sec);
                if !to_a.is_empty() {
                    node0_packets.extend(to_a.clone());
                    let from_a = self.nodes[0].process_and_forward(to_a, now_sec);
                    for pkt in from_a {
                        self.buses[0].transmit(pkt, now_sec);
                    }
                }
            }
        }

        StepResult {
            node0_arrived: node0_packets,
        }
    }

    /// Teljes in-flight csomagszám a teljes hálózatban.
    #[allow(dead_code)]
    pub fn total_in_flight(&self) -> usize {
        self.buses.iter().map(|b| b.in_flight_count()).sum()
    }

    /// Minden busz lekapcsolása (Negatív kontroll mérés).
    pub fn disconnect_all(&mut self) {
        for b in &mut self.buses {
            b.disconnect();
        }
    }

    /// Minden busz visszakapcsolása.
    #[allow(dead_code)]
    pub fn connect_all(&mut self) {
        for b in &mut self.buses {
            b.connect();
        }
    }
}
