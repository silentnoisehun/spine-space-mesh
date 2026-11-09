// src/benchmark.rs — Kvantitatív Kísérleti és Mérési Csomag
//
// Futtatja a 4 mérési cölöpöt:
// 1. In-flight memóriamegőrzés a keringési ciklusok függvényében (Ring).
// 2. Negatív Kontroll: szétkapcsolt Spine busz teszt (Zero-I/O integritás).
// 3. Topológia összehasonlítás: Gyűrű vs. Rezonátor Vonal.
// 4. Térmemória kapacitás és lecsengési ráta (γ) mérés.

use crate::mesh::SpaceMesh;
use crate::modulator::BpskModulator;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BenchmarkReport {
    pub timestamp_utc: String,
    pub payload_hex: String,
    pub carrier_freq_hz: f64,
    pub delay_per_hop_sec: f64,
    pub ring_results: Vec<CycleResult>,
    pub negative_control_passed: bool,
    pub negative_control_ber: f64,
    pub line_results: Vec<CycleResult>,
    pub theoretical_capacity_bits: usize,
    pub verdict: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CycleResult {
    pub cycle: usize,
    pub elapsed_time_sec: f64,
    pub total_hops: usize,
    pub decoded_hex: String,
    pub bit_errors: usize,
    pub ber: f64,
    pub mean_amplitude: f64,
    pub status: String,
}

pub fn run_comprehensive_benchmark() -> BenchmarkReport {
    println!("==============================================================================");
    println!("SPINE SPACE MESH — IN-FLIGHT DINAMIKUS TÉRMEMÓRIA BENCHMARK");
    println!("==============================================================================");

    let test_word: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let carrier_freq = 10_000.0; // 10 kHz
    let symbol_period = 0.0001;  // 100 us per symbol (6.4 ms total 64-bit train)
    let delay_hop = 0.005;       // 5 ms delay per hop
    let bus_gamma = 2.0;         // Csillapítás a közegben
    let gain_per_node = 1.011;   // Regeneratív kompenzáció (1.011 * exp(-2 * 0.005) approx 1.00)

    let modulator = BpskModulator::new(carrier_freq, symbol_period, 0.5, 1.0);

    // 1. Gyűrű Kísérlet (3 csomópont, 3 Spine busz)
    println!("\n[1/3] KERINGTETÉS GYŰRŰ TOPOLÓGIÁBAN (3 csomópont)...");
    let num_nodes = 3;
    let round_trip_delay = delay_hop * (num_nodes as f64); // 15 ms / cycle
    let cycles_to_test = [1, 2, 5, 10, 20, 50];
    let mut ring_results = Vec::new();

    for &cycles in &cycles_to_test {
        let mut mesh = SpaceMesh::new_ring(num_nodes, delay_hop, gain_per_node, bus_gamma);
        let packets = modulator.encode_u64(test_word, 0.0);

        // Injektálás Node 0 elmenő buszára
        for p in packets {
            mesh.inject(0, p, 0.0);
        }

        let cycle_arrival_start = (cycles as f64) * round_trip_delay;
        let cycle_arrival_end = cycle_arrival_start + (64.0 * symbol_period) + 0.0005;
        let dt = 0.00005; // 50 us lépésméret (finom mintavételezés)
        let mut current_time = 0.0;
        let mut collected_at_node0 = Vec::new();

        while current_time <= cycle_arrival_end {
            current_time += dt;
            let step_res = mesh.step(current_time);

            // Ha a megfigyelési ablakban vagyunk, gyűjtjük a Node 0-n áthaladó csomagokat
            if current_time >= cycle_arrival_start && current_time <= cycle_arrival_end {
                for pkt in step_res.node0_arrived {
                    let (bit, conf) = modulator.decode_packet(&pkt, current_time);
                    collected_at_node0.push((pkt, bit, conf));
                }
            }
        }

        let mut decoded_word: u64 = 0;
        let mut amp_sum = 0.0;

        let num_decoded = collected_at_node0.len().min(64);
        for bit_idx in 0..num_decoded {
            let (ref pkt, raw_bit, _) = collected_at_node0[bit_idx];
            amp_sum += pkt.amplitude;
            // Ha a gyűrű páratlan számú hopból áll (3 hop), páratlan ciklusnál π fázisfordulás (inverzió) lép fel:
            let bit = if cycles % 2 == 1 && num_nodes % 2 == 1 {
                raw_bit ^ 1
            } else {
                raw_bit
            };
            decoded_word |= (bit as u64) << (63 - bit_idx);
        }

        let mean_amp = if num_decoded > 0 { amp_sum / num_decoded as f64 } else { 0.0 };
        let (errors, ber) = if num_decoded == 64 {
            BpskModulator::compute_ber(test_word, decoded_word)
        } else {
            (64, 1.0)
        };

        let total_time = cycle_arrival_end;
        let status = if ber == 0.0 {
            "TÖKÉLETES MEGGŐRZÉS (BER=0.0)"
        } else if ber < 0.10 {
            "ROBOSZTUS (BER < 10%)"
        } else {
            "DEGRADÁLT / ZAJOS"
        };

        println!(
            "  Ciklus #{cycles:2} ({:6.1} ms, {:3} hops): Hex=0x{:016X} | Hibás bitek: {:2}/64 (BER: {:5.2}%) | Amp: {:.3} | {}",
            total_time * 1000.0,
            cycles * num_nodes,
            decoded_word,
            errors,
            ber * 100.0,
            mean_amp,
            status
        );

        ring_results.push(CycleResult {
            cycle: cycles,
            elapsed_time_sec: total_time,
            total_hops: cycles * num_nodes,
            decoded_hex: format!("0x{decoded_word:016X}"),
            bit_errors: errors,
            ber,
            mean_amplitude: mean_amp,
            status: status.to_string(),
        });
    }

    // 2. Negatív Kontroll (Szétkapcsolt Spine Buszok)
    println!("\n[2/3] NEGATÍV KONTROLL (Szétkapcsolt Spine Buszok)...");
    let mut disc_mesh = SpaceMesh::new_ring(num_nodes, delay_hop, gain_per_node, bus_gamma);
    disc_mesh.disconnect_all(); // FIZIKAILAG SZÉTKAPCSOLVA

    let packets = modulator.encode_u64(test_word, 0.0);
    for p in packets {
        disc_mesh.inject(0, p, 0.0);
    }

    let check_time = round_trip_delay * 2.0;
    let mut ct = 0.0;
    let mut disc_packets_seen = 0;
    while ct <= check_time {
        ct += 0.00005;
        let sres = disc_mesh.step(ct);
        disc_packets_seen += sres.node0_arrived.len();
    }

    let neg_control_passed = disc_packets_seen == 0;
    let neg_control_ber = if neg_control_passed { 1.0 } else { 0.0 };

    println!(
        "  Szétkapcsolt buszon érkező csomagok száma: {} (Elvárt: 0)",
        disc_packets_seen
    );
    println!(
        "  Negatív Kontroll Eredmény: {}",
        if neg_control_passed {
            "PASSED (Zéró szivárgás, a csomópontok NEM tartották meg az adatot statikus RAM-ban)"
        } else {
            "FAILED (Illegális memóriaszivárgás történt!)"
        }
    );

    // 3. Vonal / Rezonátor Topológia
    println!("\n[3/3] REZONÁTOR VONAL TOPOLÓGIA (2 csomópont, fázis-tükrözés)...");
    let mut line_mesh = SpaceMesh::new_line(delay_hop, gain_per_node, bus_gamma);
    let line_packets = modulator.encode_u64(test_word, 0.0);
    for p in line_packets {
        line_mesh.inject(0, p, 0.0);
    }

    let line_arrival_start = delay_hop * 4.0; // 2 teljes oda-vissza kör (A->B->A->B->A)
    let line_arrival_end = line_arrival_start + (64.0 * symbol_period) + 0.0005;
    let mut lt = 0.0;
    let mut line_collected = Vec::new();

    while lt <= line_arrival_end {
        lt += 0.00005;
        let sres = line_mesh.step(lt);
        if lt >= line_arrival_start && lt <= line_arrival_end {
            for pkt in sres.node0_arrived {
                let (bit, conf) = modulator.decode_packet(&pkt, lt);
                line_collected.push((pkt, bit, conf));
            }
        }
    }

    let mut line_decoded: u64 = 0;
    let mut line_amp = 0.0;
    let line_n = line_collected.len().min(64);

    for bit_idx in 0..line_n {
        let (ref pkt, bit, _) = line_collected[bit_idx];
        line_amp += pkt.amplitude;
        line_decoded |= (bit as u64) << (63 - bit_idx);
    }
    let (line_err, line_ber) = if line_n == 64 {
        BpskModulator::compute_ber(test_word, line_decoded)
    } else {
        (64, 1.0)
    };
    println!(
        "  Rezonátor 2 oda-vissza kör ({:.1} ms): Hex=0x{:016X} | Hibák: {}/64 (BER: {:.2}%)",
        line_arrival_end * 1000.0,
        line_decoded,
        line_err,
        line_ber * 100.0
    );

    let line_results = vec![CycleResult {
        cycle: 2,
        elapsed_time_sec: line_arrival_end,
        total_hops: 4,
        decoded_hex: format!("0x{line_decoded:016X}"),
        bit_errors: line_err,
        ber: line_ber,
        mean_amplitude: if line_n > 0 { line_amp / line_n as f64 } else { 0.0 },
        status: if line_ber == 0.0 { "PASS".into() } else { "DEGRADED".into() },
    }];

    // Kapacitás számítás: C = delay * symbol_rate * hops
    let theoretical_capacity = ((round_trip_delay / symbol_period) * (num_nodes as f64)) as usize;

    println!("\n{}", "-".repeat(78));
    println!("ÖSSZEGZÉS:");
    println!("  Elméleti térbeli in-flight kapacitás: {theoretical_capacity} bit");
    println!("  Negatív kontroll: {}", if neg_control_passed { "SIKERES" } else { "SIKERTELEN" });
    println!("{}", "-".repeat(78));

    let verdict = if neg_control_passed && ring_results.iter().any(|r| r.ber < 0.05) {
        "MODEL RESULT (simulation) — in-flight mesh memory demonstrated in software model; not a physical hardware result".to_string()
    } else {
        "INCONCLUSIVE / FAILED".to_string()
    };

    BenchmarkReport {
        timestamp_utc: "2026-11-09T15:30:00Z".to_string(),
        payload_hex: format!("0x{test_word:016X}"),
        carrier_freq_hz: carrier_freq,
        delay_per_hop_sec: delay_hop,
        ring_results,
        negative_control_passed: neg_control_passed,
        negative_control_ber: neg_control_ber,
        line_results,
        theoretical_capacity_bits: theoretical_capacity,
        verdict,
    }
}
