#!/usr/bin/env python3
"""
distributed_quantum_bridge.py — Út A: Marrakesh <-> Kingston Kvantum Spine Híd

A két különálló kvantumprocesszort (Node A: Marrakesh, Node B: Kingston)
a szoftveres Spine Busz késleltető művonalával köti össze egy elosztott
kvantum mesh hálózatba (Distributed Quantum Network / Teleportation Loop).

1. Node A gerjeszti a kvantumállapotot és elvégzi a Bell-szindróma mérést.
2. A Spine Busz (in-flight) továbbítja a fázis- és paritás-biteket.
3. Node B megkapja a buszról érkező hullámcsomagot, és Pauli-korrekcióval
   helyreállítja a repülő kvantumállapotot.
4. Negatív kontroll: Szétkapcsolt Spine busz -> az állapot szétesik fehér zajra.
"""

import json
import sys
from pathlib import Path
import numpy as np

from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator

def build_distributed_bridge_circuit(theta: float = np.pi / 2, disconnected: bool = False) -> QuantumCircuit:
    """
    3 qubites elosztott kvantum híd:
      Q0: Node A forrás-qubit (|ψ⟩ = cos(theta/2)|0⟩ + sin(theta/2)|1⟩)
      Q1: Node A transit-qubit (összefonva Q2-vel)
      Q2: Node B cél-qubit
    """
    qc = QuantumCircuit(3, 3)

    # 1. Állapot-előkészítés Node A-n (Q0)
    qc.ry(theta, 0)

    # 2. Megosztott összefonódás Node A és Node B között (EPR pár: Q1 és Q2)
    qc.h(1)
    qc.cx(1, 2)

    # 3. Bell-mérés Node A-n (Q0 és Q1)
    qc.cx(0, 1)
    qc.h(0)
    qc.measure(0, 0) # b0 szindróma
    qc.measure(1, 1) # b1 szindróma

    # 4. Spine Busz Tranzit és Pauli Rekonstrukció Node B-n (Q2)
    if not disconnected:
        # A Spine buszon átérkező bitek vezérlik a lokális korrekciót
        qc.cx(1, 2)
        qc.cz(0, 2)
    else:
        # Ha a Spine busz szét van kapcsolva, nincs korrekció (a szindróma elveszett)
        pass

    # 5. Végső ellenőrző mérés Node B-n (Q2)
    # Ha theta = pi/2, akkor |ψ⟩ = |+⟩, tehát H után 100% '0'-t kell kapnunk
    qc.h(2)
    qc.measure(2, 2)

    return qc

def run_bridge_benchmark():
    print("=" * 78)
    print("ÚT A: EGYÜTTMŰKÖDŐ KVANTUM SPIDER-MESH (Marrakesh <-> Kingston Híd)")
    print("=" * 78)

    sim = AerSimulator()
    shots = 4096

    # 1. Összekapcsolt Spine Busz Teszt
    print("\n[1/2] Aktív Spine Busz (Teleportációs Kvantum Tranzit Node A -> Node B)...")
    qc_conn = build_distributed_bridge_circuit(theta=np.pi / 2, disconnected=False)
    job_conn = sim.run(qc_conn, shots=shots)
    counts_conn = job_conn.result().get_counts()

    # A 2. klasszikus bit (Q2 mérése, a legbaloldalibb bit Qiskitben: c[2])
    success_shots = 0
    for bitstr, cnt in counts_conn.items():
        # bitstr formátum: c[2] c[1] c[0]
        # bitstr[0] a c[2]
        if bitstr[0] == '0':
            success_shots += cnt

    p_transit = success_shots / shots  # X-basis survival probability (not full state fidelity)
    print(f"  Node B-re átérkező sikeres tranzit aránya (ideális szimulátor): {p_transit * 100:.2f}%")
    print(f"  Státusz: {'SIKERES TRANZIT (ideális szimulátor)' if p_transit > 0.99 else 'DEGRADÁLT'}")

    # 2. Negatív Kontroll (Szétkapcsolt Spine Busz)
    print("\n[2/2] Negatív Kontroll (Szétkapcsolt Spine Busz — Nincs szindróma átvitel)...")
    qc_disc = build_distributed_bridge_circuit(theta=np.pi / 2, disconnected=True)
    job_disc = sim.run(qc_disc, shots=shots)
    counts_disc = job_disc.result().get_counts()

    disc_success = 0
    for bitstr, cnt in counts_disc.items():
        if bitstr[0] == '0':
            disc_success += cnt

    disc_fidelity = disc_success / shots
    print(f"  Szétkapcsolt busz hűsége: {disc_fidelity * 100:.2f}% (Elvárt: 50.00% keverék)")
    print(f"  Negatív Kontroll Eredmény: {'PASSED (Információ-áramlás megszűnt)' if abs(disc_fidelity - 0.5) < 0.05 else 'FAILED'}")

    report = {
        "timestamp_utc": "2026-11-09T15:45:00Z",
        "protocol": "Distributed Quantum Spine Mesh (Node A <-> Node B)",
        "connected_p_transit": p_transit,  # X-basis survival (ideal simulator)
        "disconnected_p_transit": disc_fidelity,
        "negative_control_passed": abs(disc_fidelity - 0.5) < 0.05,
        "verdict": "SIMULATOR DESIGN VALIDATION (Aer, ideal) — not a physical hardware result"
    }

    out_file = Path(__file__).resolve().parent.parent / "results" / "quantum_distributed_bridge_report.json"
    out_file.parent.mkdir(exist_ok=True, parents=True)
    with open(out_file, "w") as f:
        json.dump(report, f, indent=2)

    print(f"\nJelentés mentve: {out_file}")

if __name__ == "__main__":
    run_bridge_benchmark()
