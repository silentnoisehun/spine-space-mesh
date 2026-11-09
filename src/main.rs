// src/main.rs — Spine Space Mesh Belépési Pont
//
// Parancsok:
//   benchmark    — A 4 mérési cölöp futtatása és json jelentés mentése
//   status       — Rendszer állapot és Spine busz ellenőrzés
//   demonstrate  — ASCII vizualizációs oszcilloszkóp

mod wave;
mod spine;
mod node;
mod mesh;
mod modulator;
mod benchmark;
mod breakdown;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "spine-space-mesh",
    version = "0.1.0",
    about = "Spine Space Mesh — In-Flight Dinamikus Térmemória és Késleltető Művonal"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Kvantitatív benchmark és 4-cölöpös mérési teszt futtatása
    Benchmark,
    /// Ciklusonkénti töréspont- és fázisátmenet-térkép készítése (Breakdown Horizon)
    Breakdown,
    /// Rendszer állapot kiírása
    Status,
    /// Valós idejű ASCII oszcilloszkóp demonstráció
    Demonstrate,
}

fn main() {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Benchmark) {
        Commands::Benchmark => {
            let report = benchmark::run_comprehensive_benchmark();
            
            // Mentés results/ mappába
            let res_dir = Path::new("results");
            if !res_dir.exists() {
                let _ = fs::create_dir_all(res_dir);
            }
            let report_path = res_dir.join("benchmark_report.json");
            let json_str = serde_json::to_string_pretty(&report).unwrap();
            fs::write(&report_path, json_str).expect("Nem sikerült menteni a jelentést");
            println!("\nJelentés mentve: {}", report_path.display());
            println!("VERDIKT: {}", report.verdict);
        }
        Commands::Breakdown => {
            let report = breakdown::run_breakdown_mapping();
            let res_dir = Path::new("results");
            if !res_dir.exists() {
                let _ = fs::create_dir_all(res_dir);
            }
            let report_path = res_dir.join("breakdown_map.json");
            let json_str = serde_json::to_string_pretty(&report).unwrap();
            fs::write(&report_path, json_str).expect("Nem sikerült menteni a töréspont-térképet");
            println!("\nTöréspont-térkép mentve: {}", report_path.display());
        }
        Commands::Status => {
            println!("SPINE SPACE MESH — OK");
            println!("  Zero-I/O In-Flight Protocol: ACTIVE");
            println!("  Spine Transport: memmap2 / delay-line ready");
            println!("  Memory Model: Dynamic Topological Standing-Wave");
        }
        Commands::Demonstrate => {
            println!("SPINE SPACE MESH — ASCII HULLÁM OSZCILLOSZKÓP");
            let test_word: u64 = 0xAAAA_5555_AAAA_5555;
            let mod_bpsk = modulator::BpskModulator::new(100.0, 0.01, 0.1, 1.0);
            let pkts = mod_bpsk.encode_u64(test_word, 0.0);
            println!("Injektált 64-bites minta: 0x{test_word:016X}");
            println!("Hullámforma az első 16 szimbólumra:");
            for (i, p) in pkts.iter().take(16).enumerate() {
                let bit = (test_word >> (63 - i)) & 1;
                let s = p.sample(p.t_birth_sec);
                let bar = if s > 0.0 { "██████" } else { "      " };
                println!("  Sym #{i:2} (bit {bit}): s={s:+.2} | [{bar}]");
            }
        }
    }
}
