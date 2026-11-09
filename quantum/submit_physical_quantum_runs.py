#!/usr/bin/env python3
"""
submit_physical_quantum_runs.py — Mindkét Kísérlet Beküldése Éles Heron QPU-kra

1. Út B: On-Chip 6-qubites Szupravezető Kvantum Késleltető Művonal (ibm_marrakesh)
2. Út A: Elosztott Kvantum Spine Híd (ibm_kingston)
"""

import json
import time
from pathlib import Path
import numpy as np

from qiskit import QuantumCircuit, transpile
from qiskit_ibm_runtime import QiskitRuntimeService, SamplerV2

def get_service():
    token_path = Path(r"C:\Users\mater\Downloads\apikey (2) (2).json")
    with open(token_path) as f:
        token = json.load(f)["apikey"]
    return QiskitRuntimeService(channel="ibm_quantum_platform", token=token)

def build_on_chip_circuits():
    """Út B áramkörök: 1 ciklus, 2 ciklus, negatív kontroll, és kalibrációk."""
    circuits = []
    
    # Circuit 0: 1 ciklus zárt gyűrűben (6 qubit: 0->1->2->3->4->5->0)
    qc1 = QuantumCircuit(6, 1, name="quantum_delay_1_cycle")
    qc1.h(0)
    for i in range(5):
        qc1.swap(i, i + 1)
        qc1.delay(200.0, i + 1, unit="ns")
    qc1.swap(5, 0)
    qc1.delay(200.0, 0, unit="ns")
    qc1.h(0)
    qc1.measure(0, 0)
    circuits.append(qc1)

    # Circuit 1: 2 ciklus zárt gyűrűben
    qc2 = QuantumCircuit(6, 1, name="quantum_delay_2_cycles")
    qc2.h(0)
    for _ in range(2):
        for i in range(5):
            qc2.swap(i, i + 1)
            qc2.delay(200.0, i + 1, unit="ns")
        qc2.swap(5, 0)
        qc2.delay(200.0, 0, unit="ns")
    qc2.h(0)
    qc2.measure(0, 0)
    circuits.append(qc2)

    # Circuit 2: Negatív kontroll (megszakított gyűrű — nincs visszahurkolás 5->0)
    qc_disc = QuantumCircuit(6, 1, name="quantum_delay_control_disc")
    qc_disc.h(0)
    for i in range(5):
        qc_disc.swap(i, i + 1)
        qc_disc.delay(200.0, i + 1, unit="ns")
    # Nincs qc.swap(5, 0)!! A gyűrű nyitott.
    qc_disc.h(0)
    qc_disc.measure(0, 0)
    circuits.append(qc_disc)

    # Circuit 3: Kalibráció |0⟩
    cal0 = QuantumCircuit(6, 1, name="cal_0")
    cal0.measure(0, 0)
    circuits.append(cal0)

    # Circuit 4: Kalibráció |1⟩
    cal1 = QuantumCircuit(6, 1, name="cal_1")
    cal1.x(0)
    cal1.measure(0, 0)
    circuits.append(cal1)

    return circuits

def build_distributed_bridge_circuits():
    """Út A áramkörök: Elosztott Kvantum Híd (Aktív vs Szétkapcsolt)."""
    circuits = []

    # Circuit 0: Aktív híd (Teleportációs Tranzit: Node A -> Node B)
    qc_bridge = QuantumCircuit(3, 3, name="quantum_bridge_active")
    qc_bridge.ry(np.pi / 2, 0) # |+⟩ állapot injektálása
    qc_bridge.h(1)             # EPR megosztás
    qc_bridge.cx(1, 2)
    qc_bridge.cx(0, 1)         # Bell-mérés Node A-n
    qc_bridge.h(0)
    qc_bridge.measure(0, 0)
    qc_bridge.measure(1, 1)
    # Pauli helyreállítás Node B-n (Q2)
    qc_bridge.cx(1, 2)
    qc_bridge.cz(0, 2)
    qc_bridge.h(2)             # X-bázis kiolvasás
    qc_bridge.measure(2, 2)
    circuits.append(qc_bridge)

    # Circuit 1: Negatív kontroll (Szétkapcsolt híd — szindróma elveszett)
    qc_disc = QuantumCircuit(3, 3, name="quantum_bridge_control_disc")
    qc_disc.ry(np.pi / 2, 0)
    qc_disc.h(1)
    qc_disc.cx(1, 2)
    qc_disc.cx(0, 1)
    qc_disc.h(0)
    qc_disc.measure(0, 0)
    qc_disc.measure(1, 1)
    # NINCS Pauli korrekció! A Spine busz szét van kapcsolva.
    qc_disc.h(2)
    qc_disc.measure(2, 2)
    circuits.append(qc_disc)

    return circuits

def main():
    print("=" * 78)
    print("ÉLES HARVERES KVANTUM INDÍTÁS (ibm_marrakesh + ibm_kingston)")
    print("=" * 78)

    svc = get_service()

    # 1. Beküldés ibm_marrakesh-re (Út B: On-Chip Delay Line)
    b_marrakesh = svc.backend("ibm_marrakesh")
    print(f"\n[1/2] Út B áramkörök transzpilálása {b_marrakesh.name}-re...")
    circs_b = build_on_chip_circuits()
    transpiled_b = transpile(circs_b, backend=b_marrakesh, optimization_level=1)
    print(f"  Transzpilálás kész ({len(transpiled_b)} áramkör).")

    sampler_m = SamplerV2(mode=b_marrakesh)
    print("  Beküldés ibm_marrakesh-re (2000 shot)...")
    job_b = sampler_m.run(transpiled_b, shots=2000)
    job_b_id = job_b.job_id()
    print(f"  MARRAKESH JOB ID: {job_b_id} (Státusz: {job_b.status()})")

    # 2. Beküldés ibm_kingston-ra (Út A: Distributed Quantum Bridge)
    b_kingston = svc.backend("ibm_kingston")
    print(f"\n[2/2] Út A áramkörök transzpilálása {b_kingston.name}-re...")
    circs_a = build_distributed_bridge_circuits()
    transpiled_a = transpile(circs_a, backend=b_kingston, optimization_level=1)
    print(f"  Transzpilálás kész ({len(transpiled_a)} áramkör).")

    sampler_k = SamplerV2(mode=b_kingston)
    print("  Beküldés ibm_kingston-ra (2000 shot)...")
    job_a = sampler_k.run(transpiled_a, shots=2000)
    job_a_id = job_a.job_id()
    print(f"  KINGSTON JOB ID:  {job_a_id} (Státusz: {job_a.status()})")

    # Manifest mentése
    manifest = {
        "timestamp_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "path_b_marrakesh_job_id": job_b_id,
        "path_a_kingston_job_id": job_a_id,
        "backend_path_b": b_marrakesh.name,
        "backend_path_a": b_kingston.name,
        "shots": 2000
    }
    manifest_path = Path("results/quantum_hardware_submission.json")
    manifest_path.parent.mkdir(exist_ok=True, parents=True)
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    print("\n" + "=" * 78)
    print(f"Job adatok mentve: {manifest_path}")
    print("Mindkét hardveres futtatás élesben elindult!")
    print("=" * 78)

if __name__ == "__main__":
    main()
