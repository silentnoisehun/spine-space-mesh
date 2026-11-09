#!/usr/bin/env python3
"""
on_chip_quantum_delay_line.py — Út B: Heron Szupravezető Kvantum Késleltető Művonal

Egy fizikai 12-qubites zárt gyűrűn (IBM Heron heavy-hex unit cell) keringtet
egy tetszőleges kvantumállapotot (|ψ⟩ = |+⟩) SWAP-láncon keresztül.
Szigorú Zero-I/O feltétel: A Q0 qubit NEM tárolja az állapotot tranzit közben.
Az állapot kizárólag a szupravezető mikrohullámú térben utazik.
"""

import json
import math
import sys
from pathlib import Path
import numpy as np

from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator
from qiskit.quantum_info import state_fidelity, Statevector

# Fizikai 12-qubites zárt gyűrű a Heron chipen:
HERON_12_RING = [153, 152, 151, 138, 131, 132, 133, 134, 135, 139, 155, 154]

def build_quantum_delay_line_circuit(num_qubits: int = 6, cycles: int = 1, delay_dt: float = 100.0, disconnected: bool = False) -> QuantumCircuit:
    """
    Létrehoz egy N-qubites zárt gyűrűt, ahol a Q0 állapotát SWAP kapukkal
    végigkeringtetjük a Q0 -> Q1 -> ... -> Q_{N-1} -> Q0 útvonalon `cycles` alkalommal.
    """
    qc = QuantumCircuit(num_qubits, 1)

    # 1. Állapot-injektálás Q0-ba: |+⟩ = (|0⟩ + |1⟩)/sqrt(2)
    qc.h(0)

    # 2. Keringtetés a zárt gyűrű mentén
    for c in range(cycles):
        for i in range(num_qubits - 1):
            qc.swap(i, i + 1)
            if delay_dt > 0:
                qc.delay(delay_dt, i + 1, unit='ns')

        # Visszazárás Q_{N-1} -> Q0 (kivéve ha le van kapcsolva mint negatív kontroll)
        if not disconnected:
            qc.swap(num_qubits - 1, 0)
            if delay_dt > 0:
                qc.delay(delay_dt, 0, unit='ns')

    # 3. Kiolvasás Q0-ból X-bázisban (Hadamard + Z-mérés): |+⟩ esetén 100% '0' eredmény
    qc.h(0)
    qc.measure(0, 0)

    return qc

def run_simulation_benchmark():
    print("=" * 78)
    print("ÚT B: ON-CHIP KVANTUM KÉSLELTETŐ MŰVONAL BENCHMARK (Aer + Heron Layout)")
    print("=" * 78)

    sim = AerSimulator()
    num_qubits = 6 # 6-qubites zárt szub-gyűrű
    cycles_to_test = [1, 2, 3, 5, 10]
    shots = 4096

    results = []

    print("\n[1/2] Kvantum Keringtetés Zárt Gyűrűben (Q0 -> Q1..Q5 -> Q0)...")
    for cyc in cycles_to_test:
        qc = build_quantum_delay_line_circuit(num_qubits=num_qubits, cycles=cyc, delay_dt=500.0, disconnected=False)
        job = sim.run(qc, shots=shots)
        counts = job.result().get_counts()

        zeros = counts.get('0', 0)
        ones = counts.get('1', 0)
        p_plus = zeros / shots # Mivel H után mértük, '0' = |+⟩, '1' = |-⟩
        # p_plus = X-bázis túlélési valószínűség (nem teljes állapothűség — ahhoz több bázis kellene)

        print(f"  Ciklus #{cyc:2} ({cyc * num_qubits:2} SWAP ugras): P(|+>) = {p_plus:.4f} | counts: {counts}")
        results.append({
            "cycle": cyc,
            "total_swaps": cyc * num_qubits,
            "p_plus": p_plus,
            "p_plus": p_plus,  # X-basis survival probability
            "status": "PASS" if fidelity > 0.99 else "DEGRADED"
        })

    print("\n[2/2] Negatív Kontroll (Megszakított Kvantum Gyűrű — Szétkapcsolt Busz)...")
    qc_disc = build_quantum_delay_line_circuit(num_qubits=num_qubits, cycles=1, delay_dt=500.0, disconnected=True)
    job_disc = sim.run(qc_disc, shots=shots)
    counts_disc = job_disc.result().get_counts()
    p_disc = counts_disc.get('0', 0) / shots

    # Ha a gyűrű nyitott, a Q0-ban nem a keringtetett állapot van, hanem az alapállapot |0⟩,
    # ami a H után 50% '0' és 50% '1' keveréket ad!
    print(f"  Szétkapcsolt gyűrű kimenete: P('0') = {p_disc:.4f} (Elvárt: 0.5000 klasszikus keverék)")
    print(f"  Negatív Kontroll Eredmény: {'PASSED (Zéró állapot-visszaérkezés)' if abs(p_disc - 0.5) < 0.05 else 'FAILED'}")

    report = {
        "timestamp_utc": "2026-11-09T15:45:00Z",
        "num_qubits": num_qubits,
        "ring_results": results,
        "negative_control_p": p_disc,
        "negative_control_passed": abs(p_disc - 0.5) < 0.05,
        "verdict": "SIMULATOR DESIGN VALIDATION (Aer, ideal) — physical hardware result in results/physical_quantum_hardware_marrakesh.json"
    }

    out_file = Path(__file__).resolve().parent.parent / "results" / "quantum_on_chip_report.json"
    out_file.parent.mkdir(exist_ok=True, parents=True)
    with open(out_file, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\nJelentés mentve: {out_file}")

if __name__ == "__main__":
    run_simulation_benchmark()
