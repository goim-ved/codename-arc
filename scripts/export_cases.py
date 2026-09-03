#!/usr/bin/env python3
"""
scripts/export_cases.py — Exports standard IEEE/MATPOWER cases (case9, case14)
to data/cases/ in both standard MATPOWER .m and arc Grid JSON formats, along with
oracle reference solutions for regression testing.
"""

import json
import os
import pandapower as pp
import pandapower.networks as pn


def ppc_to_matpower_m(ppc, case_name):
    lines = [f"function mpc = {case_name}", f"% MATPOWER case format for {case_name}", ""]
    lines.append(f"mpc.version = '2';")
    lines.append(f"mpc.baseMVA = {ppc['baseMVA']:.1f};")
    lines.append("")

    # Bus matrix
    lines.append("% bus data")
    lines.append("% bus_i type Pd Qd Gs Bs area Vm Va baseKV zone Vmax Vmin")
    lines.append("mpc.bus = [")
    for row in ppc['bus']:
        b_i = int(row[0]) + 1  # 1-based indexing for standard MATPOWER
        b_type = int(row[1])
        pd = row[2]
        qd = row[3]
        gs = row[4]
        bs = row[5]
        area = int(row[6]) if len(row) > 6 else 1
        # Flat start: Vm = 1.0 (or setpoint if PV/slack), Va = 0.0
        vm = 1.0
        if b_type in [2, 3]:
            # find corresponding generator setpoint
            for g_row in ppc['gen']:
                if int(g_row[0]) == int(row[0]):
                    vm = float(g_row[5])
                    break
        va = 0.0
        base_kv = row[9] if len(row) > 9 else 138.0
        zone = int(row[10]) if len(row) > 10 else 1
        vmax = row[11] if len(row) > 11 else 1.1
        vmin = row[12] if len(row) > 12 else 0.9
        lines.append(f"\t{b_i}\t{b_type}\t{pd:.4f}\t{qd:.4f}\t{gs:.4f}\t{bs:.4f}\t{area}\t{vm:.4f}\t{va:.4f}\t{base_kv:.1f}\t{zone}\t{vmax:.4f}\t{vmin:.4f};")
    lines.append("];")
    lines.append("")

    # Gen matrix
    lines.append("% generator data")
    lines.append("% bus Pg Qg Qmax Qmin Vg mBase status Pmax Pmin")
    lines.append("mpc.gen = [")
    for row in ppc['gen']:
        bus = int(row[0]) + 1
        pg = row[1]
        qg = row[2]
        qmax = row[3]
        qmin = row[4]
        vg = row[5]
        mbase = 100.0
        status = int(row[7])
        pmax = row[8]
        pmin = row[9]
        lines.append(f"\t{bus}\t{pg:.4f}\t{qg:.4f}\t{qmax:.4f}\t{qmin:.4f}\t{vg:.4f}\t{mbase:.1f}\t{status}\t{pmax:.4f}\t{pmin:.4f};")
    lines.append("];")
    lines.append("")

    # Branch matrix
    lines.append("% branch data")
    lines.append("% fbus tbus r x b rateA rateB rateC ratio angle status angmin angmax")
    lines.append("mpc.branch = [")
    for row in ppc['branch']:
        fbus = int(row[0]) + 1
        tbus = int(row[1]) + 1
        r = row[2]
        x = row[3]
        b = row[4]
        rateA = row[5]
        rateB = row[6]
        rateC = row[7]
        ratio = row[8]
        angle = row[9]
        status = int(row[10])
        angmin = row[11] if len(row) > 11 else -360.0
        angmax = row[12] if len(row) > 12 else 360.0
        lines.append(f"\t{fbus}\t{tbus}\t{r:.6f}\t{x:.6f}\t{b:.6f}\t{rateA:.1f}\t{rateB:.1f}\t{rateC:.1f}\t{ratio:.4f}\t{angle:.4f}\t{status}\t{angmin:.1f}\t{angmax:.1f};")
    lines.append("];")

    return "\n".join(lines) + "\n"


def ppc_to_arc_json(ppc):
    base_mva = float(ppc['baseMVA'])
    buses = {}
    generators = {}
    loads = {}
    branches = {}
    shunts = {}

    # Map generators first to get voltage setpoints for PV buses
    gen_vg_by_bus = {}
    for idx, row in enumerate(ppc['gen']):
        bus_id = int(row[0])
        vg = float(row[5])
        gen_vg_by_bus[bus_id] = vg

    # Map buses
    for idx, row in enumerate(ppc['bus']):
        bus_id = idx
        bus_type_raw = int(row[1])
        if bus_type_raw == 3:
            btype = "Slack"
            vm_pu = gen_vg_by_bus.get(bus_id, 1.0)
        elif bus_type_raw == 2:
            btype = "PV"
            vm_pu = gen_vg_by_bus.get(bus_id, 1.0)
        else:
            btype = "PQ"
            vm_pu = 1.0

        base_kv = float(row[9]) if float(row[9]) > 0 else 138.0
        va_rad = 0.0
        v_max = float(row[11]) if len(row) > 11 else 1.1
        v_min = float(row[12]) if len(row) > 12 else 0.9

        buses[bus_id] = {
            "id": bus_id,
            "name": f"Bus {bus_id + 1}",
            "bus_type": btype,
            "base_kv": base_kv,
            "vm_pu": vm_pu,
            "va_rad": va_rad,
            "v_min_pu": v_min,
            "v_max_pu": v_max,
        }

        # Loads
        pd = float(row[2])
        qd = float(row[3])
        if abs(pd) > 1e-6 or abs(qd) > 1e-6:
            load_id = len(loads)
            loads[load_id] = {
                "id": load_id,
                "name": f"Load {bus_id + 1}",
                "bus": bus_id,
                "p_mw": pd,
                "q_mvar": qd,
                "status": True,
            }

        # Shunts
        gs = float(row[4])
        bs = float(row[5])
        if abs(gs) > 1e-6 or abs(bs) > 1e-6:
            shunt_id = len(shunts)
            shunts[shunt_id] = {
                "id": shunt_id,
                "name": f"Shunt {bus_id + 1}",
                "bus": bus_id,
                "g_mw": gs,
                "b_mvar": bs,
                "status": True,
            }

    # Map generators
    for idx, row in enumerate(ppc['gen']):
        bus_id = int(row[0])  # already 0-based in PPC
        pg = float(row[1])
        qg = float(row[2])
        qmax = float(row[3])
        qmin = float(row[4])
        vg = float(row[5])
        status = bool(int(row[7]) > 0)
        pmax = float(row[8])
        pmin = float(row[9])

        generators[idx] = {
            "id": idx,
            "name": f"Gen {idx + 1} at Bus {bus_id + 1}",
            "bus": bus_id,
            "p_mw": pg,
            "q_mvar": qg,
            "vm_pu": vg,
            "p_min_mw": pmin,
            "p_max_mw": pmax,
            "q_min_mvar": qmin,
            "q_max_mvar": qmax,
            "status": status,
        }

    # Map branches
    for idx, row in enumerate(ppc['branch']):
        fbus = int(row[0])
        tbus = int(row[1])
        r = float(row[2])
        x = float(row[3])
        b = float(row[4])
        ratio = float(row[8])
        if ratio == 0.0:
            ratio = 1.0
        angle_deg = float(row[9])
        import math
        shift_rad = math.radians(angle_deg)
        status = bool(int(row[10]) > 0)
        rate_a = float(row[5]) if float(row[5]) > 0 else None

        branches[idx] = {
            "id": idx,
            "name": f"Branch {fbus + 1}-{tbus + 1}",
            "from_bus": fbus,
            "to_bus": tbus,
            "r_pu": r,
            "x_pu": x,
            "b_pu": b,
            "tap_ratio": ratio,
            "shift_rad": shift_rad,
            "rating_mva": rate_a,
            "status": status,
        }

    return {
        "base_mva": base_mva,
        "buses": buses,
        "branches": branches,
        "generators": generators,
        "loads": loads,
        "shunts": shunts,
    }


def solve_and_extract_oracle(net, case_name):
    # AC power flow
    pp.runpp(net, calculate_voltage_angles=True, init="flat", numba=False)
    ac_buses = {}
    import math
    for idx, row in net.res_bus.iterrows():
        ac_buses[int(idx)] = {
            "vm_pu": float(row["vm_pu"]),
            "va_deg": float(row["va_degree"]),
            "va_rad": math.radians(float(row["va_degree"])),
            "p_mw": float(row["p_mw"]),
            "q_mvar": float(row["q_mvar"]),
        }

    # DC power flow
    pp.rundcpp(net, numba=False)
    dc_buses = {}
    for idx, row in net.res_bus.iterrows():
        dc_buses[int(idx)] = {
            "vm_pu": float(row["vm_pu"]),
            "va_deg": float(row["va_degree"]),
            "va_rad": math.radians(float(row["va_degree"])),
            "p_mw": float(row["p_mw"]),
        }

    return {
        "case": case_name,
        "ac": {
            "converged": bool(net.converged),
            "buses": ac_buses,
        },
        "dc": {
            "converged": True,
            "buses": dc_buses,
        }
    }


def main():
    os.makedirs("data/cases", exist_ok=True)

    # 1. Case 9
    net9 = pn.case9()
    pp.runpp(net9, numba=False)
    ppc9 = net9._ppc

    with open("data/cases/case9.m", "w") as f:
        f.write(ppc_to_matpower_m(ppc9, "case9"))

    arc_json9 = ppc_to_arc_json(ppc9)
    with open("data/cases/case9.json", "w") as f:
        json.dump(arc_json9, f, indent=2)

    oracle9 = solve_and_extract_oracle(pn.case9(), "case9")
    with open("data/cases/case9_oracle.json", "w") as f:
        json.dump(oracle9, f, indent=2)

    print("Exported case9.m, case9.json, case9_oracle.json")

    # 2. Case 14
    net14 = pn.case14()
    pp.runpp(net14, numba=False)
    ppc14 = net14._ppc

    with open("data/cases/case14.m", "w") as f:
        f.write(ppc_to_matpower_m(ppc14, "case14"))

    arc_json14 = ppc_to_arc_json(ppc14)
    with open("data/cases/case14.json", "w") as f:
        json.dump(arc_json14, f, indent=2)

    oracle14 = solve_and_extract_oracle(pn.case14(), "case14")
    with open("data/cases/case14_oracle.json", "w") as f:
        json.dump(oracle14, f, indent=2)

    print("Exported case14.m, case14.json, case14_oracle.json")


if __name__ == "__main__":
    main()
