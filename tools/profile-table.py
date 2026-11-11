"""Run prebuilt Table probes with quiet gates and bounded stage diagnostics.

Single version diagnoses stages. Two versions use three ABBA blocks per case;
both must have identical instrumentation and fixtures. Never builds or stops
external processes. Stage times include profiler overhead, never native FPS.
"""
import argparse
import csv
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess

SPEC = importlib.util.spec_from_file_location("table_sampler", Path(__file__).with_name("sample-table.py"))
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
CASES = {
    "cells": ("cells::tb02_cells_frame_sample", {"YUE_TABLE_BENCH_ROWS": "50000"}),
    "fixed": ("churn::tb05_snapshot_churn", {"TABLE_CHURN_MEASURED": "0"}),
    "measured": ("churn::tb05_snapshot_churn", {"TABLE_CHURN_MEASURED": "1"}),
    "query": ("query::tb04_query_pipeline_sample", {}),
    "measured-frames": ("complex::tb03_measured_frame_sample", {"YUE_TABLE_BENCH_ROWS": "20000"}),
    "tree": ("complex::tb06_tree_frame_sample", {"YUE_TABLE_BENCH_ROWS": "50000"}),
    "edit": ("edit::edit_versions_conflicts_cancel_restart_and_atomic_save", {}),
    "resize": ("layout::resize_pointer_final_coordinate_keyboard_release_and_rejection", {}),
    "range": ("cells::range_tickets_isolate_queries_owners_retries_and_placeholders", {}),
}
STAGES = re.compile(r"TABLE_STAGE,name=([^,\s]+),calls=(\d+),inclusive_ns=(\d+),self_ns=(\d+),max_ns=(\d+)")


def validate(output, case):
    BASE.check_native_output(output, case)
    capture = re.findall(r"TABLE_CAPTURE,elapsed_ns=(\d+),unattributed_ns=(\d+),dropped=(\d+)", output)
    stages = STAGES.findall(output)
    if len(capture) != 1 or not stages or len({s[0] for s in stages}) != len(stages):
        raise ValueError("missing/duplicate stage capture")
    elapsed, unattributed, dropped = map(int, capture[0])
    if dropped or sum(int(s[3]) for s in stages) + unattributed != elapsed:
        raise ValueError("capture drops or self times do not reconcile")
    if any(int(s[1]) == 0 or int(s[3]) > int(s[2]) or int(s[4]) > int(s[2]) for s in stages):
        raise ValueError("invalid stage accounting")
    return elapsed, unattributed, stages


def workload(output, case_name):
    """Verify full probe sequences and preserve work, not just elapsed totals."""
    if case_name in ["cells", "measured-frames", "tree"]:
        frames = [tuple(map(int, s)) for s in re.findall(r"TABLE_FRAME,rows=(\d+),frame=(\d+),ns=(\d+),renders=(\d+)", output)]
        rows = {"cells": 50000, "measured-frames": 20000, "tree": 50000}[case_name]
        if [(s[0], s[1]) for s in frames] != [(rows, i) for i in range(1000)] or any(s[2] <= 0 or not 0 < s[3] < 600 for s in frames):
            raise ValueError("invalid frame workset sequence")
        allocations = [tuple(map(int, s)) for s in re.findall(r"TABLE_ALLOC,rows=(\d+),sample=(\d+),allocations=(\d+),bytes=(\d+)", output)]
        if [(s[0], s[1]) for s in allocations] != [(rows, i) for i in range(10)] or any(s[2] <= 0 or s[3] <= 0 for s in allocations):
            raise ValueError("invalid allocation sequence")
        return dict(work=[(s[0], s[1], s[3]) for s in frames], frames=frames, allocations=allocations)
    if case_name in ["fixed", "measured"]:
        samples = [tuple(map(int, s)) for s in re.findall(r"TABLE_CHURN,measured=(\d+),step=(\d+),prepare_ns=(\d+),accept_ns=(\d+),draw_effects_ns=(\d+),historical_sources=(\d+)", output)]
        variant = int(case_name == "measured")
        if [(s[0], s[1], s[5]) for s in samples] != [(variant, i, 0) for i in range(100)] or any(min(s[2:5]) <= 0 for s in samples):
            raise ValueError("invalid replacement/retention sequence")
        return dict(work=[(s[0], s[1], s[5]) for s in samples], replacements=samples)
    if case_name == "query":
        samples = [tuple(map(int, s)) for s in re.findall(r"TABLE_QUERY,mode=(\d+),sample=(\d+),query=(\d+),validate=(\d+),filter=(\d+),sort=(\d+),projection=(\d+),accept=(\d+),total=(\d+),rows=(\d+)", output)]
        if [(s[0], s[1]) for s in samples] != [(mode, sample) for mode in range(11) for sample in range(6)] or any(min(s[2:8]) <= 0 or sum(s[3:7]) > s[2] for s in samples):
            raise ValueError("invalid query sequence")
        return dict(work=[(s[0], s[1], s[8], s[9]) for s in samples], queries=samples)
    return dict(work=case_name)



def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path)
    parser.add_argument("--before-build", type=Path, required=True)
    parser.add_argument("--after-build", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cases", nargs="+", choices=CASES, default=["cells", "fixed", "measured", "query"])
    parser.add_argument("--capture", choices=["on", "off"], default="on", help="off repeats the same fixtures without an active capture")
    parser.add_argument("--attempts", type=int, default=20)
    args = parser.parse_args()
    if args.output.exists() or args.attempts < 1 or bool(args.after) != bool(args.after_build):
        parser.error("fresh output, positive attempts and matching after/build required")
    args.output.mkdir(parents=True)
    versions = {"before": (args.before.resolve(strict=True), args.before_build.resolve(strict=True))}
    if args.after:
        versions["after"] = (args.after.resolve(strict=True), args.after_build.resolve(strict=True))
    identities = {}
    for version, (exe, build) in versions.items():
        records = [json.loads(s) for s in build.read_text(encoding="utf-8-sig").splitlines() if s.startswith("{")]
        found = [r for r in records if r.get("executable") and Path(r["executable"]).name == exe.name]
        if len(found) != 1 or found[0]["target"]["name"] != "table" or found[0]["profile"]["opt_level"] != "3" or "table-profile" not in found[0]["features"]:
            parser.error("build record must identify the release table-profile test binary")
        identities[version] = dict(executable_sha256=BASE.digest(exe), build_sha256=BASE.digest(build))
    initial = BASE.source_digest()
    runner = BASE.digest(Path(__file__))
    runs, samples, discarded = [], [], []
    worksets = {}
    order = ["before", "after", "after", "before"] if args.after else ["before"]
    for case_name in args.cases:
        case, environment = CASES[case_name]
        for block in range(3 if args.after else 1):
            for slot, version in enumerate(order):
                exe, _ = versions[version]
                label = f"{case_name}-block-{block}-slot-{slot}-{version}"
                for attempt in range(1, args.attempts + 1):
                    BASE.wait_quiet()
                    log = args.output / f"{label}-attempt-{attempt}.txt"
                    with log.open("w", encoding="utf-8") as stream:
                        p = subprocess.Popen([str(exe), case, "--exact", "--include-ignored", "--nocapture", "--test-threads=1"], cwd=BASE.ROOT, env=dict(os.environ, TABLE_PROFILE="1" if args.capture == "on" else "0", **environment), stdout=stream, stderr=subprocess.STDOUT)
                        interfered = False
                        while p.poll() is None:
                            try:
                                p.wait(timeout=.5)
                            except subprocess.TimeoutExpired:
                                interfered |= BASE.busy()
                        interfered |= BASE.busy()
                    if p.returncode:
                        raise RuntimeError(f"failed {log}; no acceptance")
                    if not interfered:
                        break
                    discarded.append(log.name)
                    print(f"Discarding {log.name}: compilation/Showcase observed", flush=True)
                else:
                    raise RuntimeError("whole-run retry limit; no acceptance")
                output = log.read_text(encoding="utf-8")
                if args.capture == "on":
                    elapsed, unattributed, stages = validate(output, case)
                else:
                    BASE.check_native_output(output, case)
                    if STAGES.search(output) or "TABLE_CAPTURE," in output:
                        raise ValueError("unexpected capture in inactive run")
                    elapsed, unattributed, stages = 0, 0, []
                measured_work = workload(output, case_name)
                if case_name in worksets and worksets[case_name] != measured_work["work"]:
                    raise ValueError("workset/result changed across versions or repetitions")
                worksets[case_name] = measured_work["work"]
                run = dict(case=case_name, block=block, slot=slot, version=version, log=log.name, log_sha256=BASE.digest(log), elapsed_ns=elapsed, unattributed_ns=unattributed, measurements=measured_work)
                runs.append(run)
                samples.extend(dict(case=case_name, block=block, slot=slot, version=version, stage=s[0], calls=int(s[1]), inclusive_ns=int(s[2]), self_ns=int(s[3]), max_ns=int(s[4])) for s in stages)
                print(f"{label}: {elapsed / 1e6:.1f}ms; top self " + ", ".join(f"{s[0]}={int(s[3])/1e6:.1f}ms" for s in sorted(stages, key=lambda s: int(s[3]), reverse=True)[:5]), flush=True)
    if BASE.source_digest() != initial or BASE.digest(Path(__file__)) != runner or any(BASE.digest(versions[v][0]) != identities[v]["executable_sha256"] for v in versions):
        raise ValueError("source/runner/executable changed during sampling")
    with (args.output / "stages.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=samples[0].keys() if samples else ["case", "block", "slot", "version", "stage", "calls", "inclusive_ns", "self_ns", "max_ns"])
        writer.writeheader()
        writer.writerows(samples)
    identity = dict(source_sha256=initial, runner_sha256=runner, versions=identities, capture=args.capture, cases=args.cases, order=order, blocks=3 if args.after else 1, runs=runs, discarded=discarded, head=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(), captured="synchronous elapsed wall time, inclusive/self/max; profiler overhead and descheduling included; no GPU or network wait")
    (args.output / "identity.json").write_text(json.dumps(identity, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
