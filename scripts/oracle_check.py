#!/usr/bin/env python3
"""
scripts/oracle_check.py — Reference power flow oracle runner using pandapower.

Generates ground-truth bus voltage magnitudes (vm_pu) and phase angles (va_degree / va_rad)
for cross-validating arc's numerical solvers.
"""

import argparse
import datetime
import json
import sys
import pandapower as pp
import pandapower.networks as pn


def build_three_bus_case():
    """
    Constructs the canonical 3-bus test network for arc M2-M4 verification:
    Bus 0: Slack bus (vm = 1.0 pu, va = 0 deg) with generator.
    Bus 1: PQ bus with Load (P = 40 MW, Q = 20 MVar).
    Bus 2: PV bus with Generator (vm = 1.02 pu, P = 50 MW).
    Lines between 0-1, 1-2, 0-2 with specified line impedances.
    """
    net = pp.create_empty_network(f_hz=60.0, sn_mva=100.0)

    # Buses (138 kV base)
    b0 = pp.create_bus(net, vn_kv=138.0, name="Bus 0", index=0)
    b1 = pp.create_bus(net, vn_kv=138.0, name="Bus 1", index=1)
    b2 = pp.create_bus(net, vn_kv=138.0, name="Bus 2", index=2)

    # Slack generator at Bus 0
    pp.create_ext_grid(net, bus=b0, vm_pu=1.0, va_degree=0.0, name="Slack Gen")

    # Load at Bus 1: 40 MW, 20 MVar
    pp.create_load(net, bus=b1, p_mw=40.0, q_mvar=20.0, name="Load 1")

    # Generator at Bus 2: 50 MW, V = 1.02 pu
    pp.create_gen(net, bus=b2, p_mw=50.0, vm_pu=1.02, min_q_mvar=-100.0, max_q_mvar=100.0, name="Gen 2")

    # Lines: 100 MVA base, 138 kV base => Z_base = 138^2 / 100 = 190.44 Ohm
    # Line 0-1: r = 0.02 pu, x = 0.06 pu, b = 0.0 pu (Ohm: r = 3.8088, x = 11.4264)
    # Line 1-2: r = 0.01 pu, x = 0.03 pu, b = 0.0 pu
    # Line 0-2: r = 0.012 pu, x = 0.036 pu, b = 0.0 pu
    z_base = (138.0 ** 2) / 100.0
    pp.create_line_from_parameters(
        net, from_bus=b0, to_bus=b1, length_km=1.0,
        r_ohm_per_km=0.02 * z_base, x_ohm_per_km=0.06 * z_base,
        c_nf_per_km=0.0, max_i_ka=1.0, name="Line 0-1"
    )
    pp.create_line_from_parameters(
        net, from_bus=b1, to_bus=b2, length_km=1.0,
        r_ohm_per_km=0.01 * z_base, x_ohm_per_km=0.03 * z_base,
        c_nf_per_km=0.0, max_i_ka=1.0, name="Line 1-2"
    )
    pp.create_line_from_parameters(
        net, from_bus=b0, to_bus=b2, length_km=1.0,
        r_ohm_per_km=0.012 * z_base, x_ohm_per_km=0.036 * z_base,
        c_nf_per_km=0.0, max_i_ka=1.0, name="Line 0-2"
    )

    return net


def get_network(case_name: str):
    case_lower = case_name.lower().replace("-", "").replace("_", "")
    if case_lower in ["case3", "3bus"]:
        return build_three_bus_case()
    elif case_lower in ["case9", "9bus"]:
        return pn.case9()
    elif case_lower in ["case14", "14bus"]:
        return pn.case14()
    else:
        # Attempt to load as file
        try:
            return pp.from_json(case_name)
        except Exception as err:
            raise ValueError(f"Unknown network case or unable to load file: {case_name} ({err})")


def run_oracle(case_name: str, mode: str = "ac"):
    net = get_network(case_name)
    mode = mode.lower()

    if mode == "dc":
        pp.rundcpp(net, numba=False)
        converged = bool(net.converged)
    elif mode == "ac":
        pp.runpp(net, calculate_voltage_angles=True, init="flat", numba=False)
        converged = bool(net.converged)
    else:
        raise ValueError(f"Unsupported mode: {mode}. Must be 'ac' or 'dc'.")

    bus_results = {}
    import math
    for idx, row in net.res_bus.iterrows():
        bus_idx = int(idx)
        vm = float(row["vm_pu"])
        va_deg = float(row["va_degree"])
        va_rad = math.radians(va_deg)
        bus_results[bus_idx] = {
            "bus_id": bus_idx,
            "vm_pu": round(vm, 10),
            "va_degree": round(va_deg, 10),
            "va_rad": round(va_rad, 10),
            "p_mw": round(float(row.get("p_mw", 0.0)), 8),
            "q_mvar": round(float(row.get("q_mvar", 0.0)), 8),
        }

    output = {
        "metadata": {
            "case": case_name,
            "mode": mode.upper(),
            "converged": converged,
            "pandapower_version": pp.__version__,
            "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        },
        "buses": bus_results,
    }
    return output


def main():
    parser = argparse.ArgumentParser(description="pandapower numerical oracle for arc")
    parser.add_argument("--case", default="case3", help="Case to solve (case3, case9, case14, or JSON path)")
    parser.add_argument("--mode", default="ac", choices=["ac", "dc"], help="Power flow mode (ac or dc)")
    parser.add_argument("--output", default=None, help="Path to write JSON output")
    args = parser.parse_args()

    result = run_oracle(args.case, args.mode)
    json_data = json.dumps(result, indent=2)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(json_data)
        print(f"Wrote oracle results to {args.output}")
    else:
        print(json_data)


if __name__ == "__main__":
    main()
