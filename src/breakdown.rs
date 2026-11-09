// src/breakdown.rs — Töréspont-Térkép és Fázisátmenet Elemző
//
// Méri a memória pontos stabilitási határát (Breakdown Horizon):
// 1. Full Attraktor: Pump + Gross-Pitaevskii Telítés + Kuramoto Fázis-Zárás
// 2. No-Phase Control: Csak amplitúdó telítés, fázis-zárás nélkül (szabad fázissodródás)
// 3. Passzív Csillapított Kontroll: Nincs pumpálás (e^(-γt) disszipáció)
//
// Kimutatja a kritikus ciklusszámot (C_crit), ahol a memória összeomlik.

use crate::mesh::SpaceMesh;
use crate::modulator::BpskModulator;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct BreakdownPoint {
    pub cycle: usize,
    pub elapsed_ms: f64,
    // Full Attractor
    pub full_ber: f64,
    pub full_amp: f64,
    pub full_phase_deg: f64,
    // No Phase Attractor (Free Drift)
    pub nodrift_ber: f64,
    pub nodrift_amp: f64,
    // Passive (No Pump)
    pub passive_ber: f64,
    pub passive_amp: f64,
}

#[derive(Debug, Serialize)]
pub struct BreakdownReport {
    pub timestamp_utc: String,
    pub test_word_hex: String,
    pub points: Vec<BreakdownPoint>,
    pub critical_cycle_full: usize,
    pub critical_cycle_nodrift: usize,
    pub critical_cycle_passive: usize,
}

pub fn run_breakdown_mapping() -> BreakdownReport {
    println!("==============================================================================");
    println!("SPINE SPACE MESH — TÖRÉSPONT-TÉRKÉP ÉS FÁZISÁTMENET ANALÍZIS");
    println!("==============================================================================");

    let test_word: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let carrier_freq = 10_000.0;
    let symbol_period = 0.0001; // 100 us
    let delay_hop = 0.005;      // 5 ms
    let bus_gamma = 2.0;
    let gain = 1.0115;
    let num_nodes = 3;
    let round_trip_delay = delay_hop * (num_nodes as f64); // 15 ms

    let modulator = BpskModulator::new(carrier_freq, symbol_period, 0.5, 1.0);

    let max_cycles = 25;
    let mut points = Vec::new();

    let mut crit_full = max_cycles;
    let mut crit_nodrift = max_cycles;
    let mut crit_passive = max_cycles;

    println!("Ciklus | Idő (ms) | Full BER | Full Amp | Szabad BER | Szabad Amp | Passzív Amp | Státusz");
    println!("-------|----------|----------|----------|------------|------------|-------------|---------");

    for cycle in 1..=max_cycles {
        let arrival_start = (cycle as f64) * round_trip_delay;
        let arrival_end = arrival_start + (64.0 * symbol_period) + 0.0005;
        let dt = 0.00005;

        // 1. Full Attractor Run
        let mut mesh_full = SpaceMesh::new_ring(num_nodes, delay_hop, gain, bus_gamma);
        let pkts = modulator.encode_u64(test_word, 0.0);
        for p in pkts { mesh_full.inject(0, p, 0.0); }

        let mut t = 0.0;
        let mut collected_full = Vec::new();
        while t <= arrival_end {
            t += dt;
            let res = mesh_full.step(t);
            if t >= arrival_start && t <= arrival_end {
                for p in res.node0_arrived {
                    let (b, c) = modulator.decode_packet(&p, t);
                    collected_full.push((p, b, c));
                }
            }
        }

        let mut dec_full = 0u64;
        let mut amp_full = 0.0;
        let n_full = collected_full.len().min(64);
        for i in 0..n_full {
            let (ref p, b, _) = collected_full[i];
            amp_full += p.amplitude;
            let bit = if cycle % 2 == 1 && num_nodes % 2 == 1 { b ^ 1 } else { b };
            dec_full |= (bit as u64) << (63 - i);
        }
        let (_, ber_full) = if n_full == 64 { BpskModulator::compute_ber(test_word, dec_full) } else { (64, 1.0) };
        let mean_amp_full = if n_full > 0 { amp_full / n_full as f64 } else { 0.0 };

        // 2. No Phase Attractor (beta = 0.0)
        let mut mesh_nodrift = SpaceMesh::new_ring(num_nodes, delay_hop, gain, bus_gamma);
        for node in &mut mesh_nodrift.nodes {
            node.phase_restoring_beta = 0.0; // Fázis-visszahúzás kikapcsolva!
        }
        let pkts2 = modulator.encode_u64(test_word, 0.0);
        for p in pkts2 { mesh_nodrift.inject(0, p, 0.0); }

        let mut t2 = 0.0;
        let mut collected_nodrift = Vec::new();
        while t2 <= arrival_end {
            t2 += dt;
            let res = mesh_nodrift.step(t2);
            if t2 >= arrival_start && t2 <= arrival_end {
                for p in res.node0_arrived {
                    let (b, c) = modulator.decode_packet(&p, t2);
                    collected_nodrift.push((p, b, c));
                }
            }
        }
        let mut dec_nodrift = 0u64;
        let mut amp_nodrift = 0.0;
        let n_nodrift = collected_nodrift.len().min(64);
        for i in 0..n_nodrift {
            let (ref p, b, _) = collected_nodrift[i];
            amp_nodrift += p.amplitude;
            let bit = if cycle % 2 == 1 && num_nodes % 2 == 1 { b ^ 1 } else { b };
            dec_nodrift |= (bit as u64) << (63 - i);
        }
        let (_, ber_nodrift) = if n_nodrift == 64 { BpskModulator::compute_ber(test_word, dec_nodrift) } else { (64, 1.0) };
        let mean_amp_nodrift = if n_nodrift > 0 { amp_nodrift / n_nodrift as f64 } else { 0.0 };

        // 3. Passive (No Pump: gain = 1.0, internal_gamma = 0)
        let mut mesh_passive = SpaceMesh::new_ring(num_nodes, delay_hop, 1.0, bus_gamma);
        for node in &mut mesh_passive.nodes {
            node.pump_gain = 1.0; // Nincs erősítés
            node.saturation_g = 0.0;
            node.phase_restoring_beta = 0.0;
        }
        let pkts3 = modulator.encode_u64(test_word, 0.0);
        for p in pkts3 { mesh_passive.inject(0, p, 0.0); }

        let mut t3 = 0.0;
        let mut collected_pass = Vec::new();
        while t3 <= arrival_end {
            t3 += dt;
            let res = mesh_passive.step(t3);
            if t3 >= arrival_start && t3 <= arrival_end {
                for p in res.node0_arrived {
                    let (b, c) = modulator.decode_packet(&p, t3);
                    collected_pass.push((p, b, c));
                }
            }
        }
        let mut amp_pass = 0.0;
        let n_pass = collected_pass.len().min(64);
        for i in 0..n_pass {
            amp_pass += collected_pass[i].0.amplitude;
        }
        let mean_amp_pass = if n_pass > 0 { amp_pass / n_pass as f64 } else { 0.0 };
        let ber_pass = if mean_amp_pass > 0.05 { 0.0 } else { 1.0 };

        // Töréspontok naplózása
        if ber_full > 0.05 && crit_full == max_cycles { crit_full = cycle; }
        if ber_nodrift > 0.05 && crit_nodrift == max_cycles { crit_nodrift = cycle; }
        if mean_amp_pass < 0.01 && crit_passive == max_cycles { crit_passive = cycle; }

        let status = if ber_full == 0.0 { "STABIL ATTRACTOR" } else { "BREAKDOWN" };

        println!(
            "  {:2}   | {:8.1} |  {:5.2}%  |  {:.3}   |   {:5.2}%  |   {:.3}    |    {:.4}   | {}",
            cycle,
            arrival_end * 1000.0,
            ber_full * 100.0,
            mean_amp_full,
            ber_nodrift * 100.0,
            mean_amp_nodrift,
            mean_amp_pass,
            status
        );

        points.push(BreakdownPoint {
            cycle,
            elapsed_ms: arrival_end * 1000.0,
            full_ber: ber_full,
            full_amp: mean_amp_full,
            full_phase_deg: 0.0,
            nodrift_ber: ber_nodrift,
            nodrift_amp: mean_amp_nodrift,
            passive_ber: ber_pass,
            passive_amp: mean_amp_pass,
        });
    }

    println!("------------------------------------------------------------------------------");
    println!("TÖRÉSPONT KONKLÚZIÓ:");
    println!("  Passzív közeg túlélési határa (C_crit): Ciklus #{crit_passive} (~{:.1} ms)", (crit_passive as f64) * round_trip_delay * 1000.0);
    println!("  Fázis-zárás nélküli sodródási határ:    Ciklus #{crit_nodrift} (~{:.1} ms)", (crit_nodrift as f64) * round_trip_delay * 1000.0);
    println!("  Teljes Gross-Pitaevskii attraktor:      Ciklus #{crit_full} (~{:.1} ms)", (crit_full as f64) * round_trip_delay * 1000.0);
    println!("------------------------------------------------------------------------------");

    BreakdownReport {
        timestamp_utc: "2026-11-09T15:40:00Z".to_string(),
        test_word_hex: format!("0x{test_word:016X}"),
        points,
        critical_cycle_full: crit_full,
        critical_cycle_nodrift: crit_nodrift,
        critical_cycle_passive: crit_passive,
    }
}
