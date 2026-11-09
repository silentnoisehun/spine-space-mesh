#!/usr/bin/env python3
"""
extract_physical_quantum_results.py — Fizikai Heron Hardveres Mérések Kinyerése és Hitelesítése

Kinyeri a nyers counts adatokat a két éles IBM Heron futásból:
- Path B: dboulglt7fqc7388fs6g (ibm_marrakesh — On-Chip Delay Line)
- Path A: dboulh5t7fqc7388fs70 (ibm_kingston — Distributed Bridge)
"""

import json
import time
from pathlib import Path
from qiskit_ibm_runtime import QiskitRuntimeService

def get_service():
    token_path = Path(r"C:\Users\mater\Downloads\apikey (2) (2).json")
    with open(token_path) as f:
        token = json.load(f)["apikey"]
    return QiskitRuntimeService(channel="ibm_quantum_platform", token=token)

def extract_pub_counts(pub_result):
    data = pub_result.data
    # Keresünk bitstring adatokat (pl. c vagy meas)
    for attr in ("c", "meas", "cr"):
        if hasattr(data, attr):
            val = getattr(data, attr)
            if hasattr(val, "get_counts"):
                return val.get_counts()
    for k in dir(data):
        if not k.startswith("_"):
            val = getattr(data, k)
            if hasattr(val, "get_counts"):
                return val.get_counts()
    return {}

def main():
    print("=" * 78)
    print("ÉLES HARVERES ADATKINYERÉS ÉS HITELESÍTÉS")
    print("=" * 78)

    svc = get_service()

    sub_file = Path("results/quantum_hardware_submission.json")
    if not sub_file.exists():
        print(f"Hiba: {sub_file} nem található!")
        return

    with open(sub_file) as f:
        sub = json.load(f)

    job_b_id = sub["path_b_marrakesh_job_id"]
    job_a_id = sub["path_a_kingston_job_id"]

    print(f"\n[1/2] ibm_marrakesh (Út B) figyelése: {job_b_id}...")
    job_b = svc.job(job_b_id)
    st_b = str(job_b.status()).upper()
    print(f"  Státusz: {st_b}")
    if st_b not in ("DONE", "COMPLETED"):
        print("  Várakozás a Marrakesh futás végére...")
        job_b.wait_for_final_state()

    res_b = job_b.result()
    print(f"  Marrakesh kész! Pubok száma: {len(res_b)}")
    counts_b = [extract_pub_counts(p) for p in res_b]

    # Kiértékelés Út B
    # Áramkörök: [1_cycle, 2_cycles, disconnected_control, cal_0, cal_1]
    shots = 2000
    p0_cal = counts_b[3].get('0', 0) / shots # p(0|0)
    p1_cal = counts_b[4].get('1', 0) / shots # p(1|1)

    p_1cyc = counts_b[0].get('0', 0) / shots
    p_2cyc = counts_b[1].get('0', 0) / shots
    p_disc = counts_b[2].get('0', 0) / shots

    print(f"  Kalibrációs readout: p(0|0)={p0_cal:.4f}, p(1|1)={p1_cal:.4f}")
    print(f"  1 Ciklus Fizikai Hűség: P(|+>) = {p_1cyc:.4f} (counts: {counts_b[0]})")
    print(f"  2 Ciklus Fizikai Hűség: P(|+>) = {p_2cyc:.4f} (counts: {counts_b[1]})")
    print(f"  Negatív Kontroll:       P(|+>) = {p_disc:.4f} (counts: {counts_b[2]})")

    print(f"\n[2/2] ibm_kingston (Út A) figyelése: {job_a_id}...")
    job_a = svc.job(job_a_id)
    st_a = str(job_a.status()).upper()
    print(f"  Státusz: {st_a}")
    if st_a not in ("DONE", "COMPLETED"):
        print("  Várakozás a Kingston futás végére...")
        job_a.wait_for_final_state()

    res_a = job_a.result()
    print(f"  Kingston kész! Pubok száma: {len(res_a)}")
    counts_a = [extract_pub_counts(p) for p in res_a]

    # Kiértékelés Út A: [active_bridge, disconnected_bridge]
    # Q2 mérés bitje c[2], ami a legbaloldalibb karakter a Qiskit bitstringben
    def get_q2_fidelity(c_dict):
        success = sum(cnt for bs, cnt in c_dict.items() if bs[0] == '0')
        return success / sum(c_dict.values())

    fid_active = get_q2_fidelity(counts_a[0])
    fid_disc = get_q2_fidelity(counts_a[1])

    print(f"  Aktív Híd Fizikai Hűség: {fid_active * 100:.2f}% (counts: {counts_a[0]})")
    print(f"  Negatív Kontroll Hűség:   {fid_disc * 100:.2f}% (counts: {counts_a[1]})")

    report = {
        "timestamp_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "marrakesh_job_id": job_b_id,
        "kingston_job_id": job_a_id,
        "path_b_on_chip": {
            "p0_cal": p0_cal,
            "p1_cal": p1_cal,
            "p_1_cycle": p_1cyc,
            "p_2_cycles": p_2cyc,
            "p_disconnected": p_disc,
            "raw_counts": counts_b
        },
        "path_a_distributed": {
            "active_fidelity": fid_active,
            "disconnected_fidelity": fid_disc,
            "raw_counts": counts_a
        }
    }

    out_file = Path("results/physical_quantum_hardware_report.json")
    with open(out_file, "w") as f:
        json.dump(report, f, indent=2)

    print("\n" + "=" * 78)
    print(f"MINDEN FIZIKAI HARDVERES EREDMÉNY MENTVE: {out_file}")
    print("=" * 78)

if __name__ == "__main__":
    main()
