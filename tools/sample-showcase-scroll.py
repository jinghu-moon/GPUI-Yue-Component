"""Prebuilt Showcase wheel CPU comparison; real pages, no native FPS claim."""
import argparse
import csv
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
from statistics import median
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("table_sampling", ROOT / "tools/sample-table.py")
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
CASE = "scroll::tests::shared_pages_wheel_sample"


def fingerprint():
    value = hashlib.sha256()
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__), ROOT / "tools/sample-table.py", ROOT / "tools/virtualization_evidence.py"]
    for folder in ["examples/windows-showcase", "crates/yue-component", "vendor/gpui-base", "vendor/gpui-pre", "vendor/gpui-pre-windows"]:
        paths += [p for p in (ROOT / folder).rglob("*") if p.suffix in [".rs", ".toml"] and "target" not in p.parts]
    for p in sorted(paths):
        value.update(p.relative_to(ROOT).as_posix().encode()); value.update(p.read_bytes())
    return value.hexdigest()


def samples(output, case):
    BASE.check_native_output(output, CASE)
    rows = [tuple(map(int, r)) for r in re.findall(r"SCROLL_FRAME,case=" + case + r",frame=(\d+),ns=(\d+),quads=(\d+)", output)]
    if [r[0] for r in rows] != list(range(240)) or any(r[1] <= 0 or r[2] <= 0 for r in rows):
        raise ValueError("missing/invalid wheel samples")
    return rows


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--before-build", type=Path, required=True)
    parser.add_argument("--after", type=Path)
    parser.add_argument("--after-build", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cases", nargs="+", choices=["button", "table", "list"], default=["button", "table", "list"])
    parser.add_argument("--ui-regression", action="store_true", help="Table gallery changes the scene; require <=10% P50/P95 regression, with within-version work checks")
    args = parser.parse_args()
    if args.output.exists() or bool(args.after) != bool(args.after_build):
        parser.error("fresh output and matched after/build required")
    if args.ui_regression and (not args.after or args.cases != ["table"]):
        parser.error("UI regression requires two binaries and --cases table")
    versions = {"before": (args.before.resolve(), args.before_build.resolve())}
    if args.after: versions["after"] = (args.after.resolve(), args.after_build.resolve())
    builds = {}
    for version, (exe, build) in versions.items():
        records = [json.loads(s) for s in build.read_text(encoding="utf-8-sig").splitlines() if s.startswith("{")]
        artifact = [r for r in records if r.get("executable") and Path(r["executable"]).name == exe.name]
        if len(artifact) != 1 or artifact[0]["target"]["name"] != "windows-showcase" or not artifact[0]["profile"]["test"]:
            parser.error("build does not identify Showcase test binary")
        builds[version] = dict(exe_sha256=BASE.digest(exe), build_sha256=BASE.digest(build), profiles={r["target"]["name"]: r["profile"] for r in records if r.get("reason")=="compiler-artifact" and r["target"]["name"] in ["gpui", "gpui_base", "taffy", "gpui_windows", "windows-showcase"]})
    args.output.mkdir(parents=True)
    source = fingerprint()
    order = ["before", "after", "after", "before"] if args.after else ["before"]
    runs, csv_rows, discarded, work = [], [], [], {}
    for case in args.cases:
        for block in range(3 if args.after else 1):
            for slot, version in enumerate(order):
                exe, build = versions[version]
                for attempt in range(1, 4):
                    BASE.wait_quiet(ignore_foreign_app_wrappers=args.ui_regression)
                    name = f"{case}-{block}-{slot}-{version}-attempt-{attempt}.txt"
                    log = args.output / name
                    with log.open("w", encoding="utf-8") as stream:
                        process = subprocess.Popen([str(exe), CASE, "--exact", "--ignored", "--nocapture", "--test-threads=1"], cwd=ROOT, env=dict(os.environ, YUE_SCROLL_CASE=case, SEED="0", ITERATIONS="1"), stdout=stream, stderr=subprocess.STDOUT)
                        polluted = False
                        while process.poll() is None:
                            try: process.wait(timeout=.5)
                            except subprocess.TimeoutExpired: polluted |= BASE.busy(ignore_foreign_app_wrappers=args.ui_regression)
                        polluted |= BASE.busy(ignore_foreign_app_wrappers=args.ui_regression)
                    if process.returncode: raise RuntimeError(f"probe failed: {log}")
                    if not polluted: break
                    discarded.append(name)
                else: raise RuntimeError("interference retry budget exhausted")
                values = samples(log.read_text(encoding="utf-8"), case)
                counts = [r[2] for r in values]
                work_key = (case, version) if args.ui_regression else case
                if work_key in work and work[work_key] != counts: raise ValueError("rendered scene workload changed within the comparison contract")
                work[work_key] = counts
                times = sorted(r[1] for r in values)
                run = dict(case=case, block=block, slot=slot, version=version, log=name, log_sha256=BASE.digest(log), p50=times[119], p95=times[227], p99=times[236])
                runs.append(run)
                csv_rows.extend(dict(case=case, block=block, slot=slot, version=version, frame=f, ns=n, quads=q) for f,n,q in values)
                print(f"{case}/{block}/{slot}/{version}: P50={run['p50']/1e6:.2f}ms P95={run['p95']/1e6:.2f}ms", flush=True)
    if fingerprint() != source or any(BASE.digest(exe) != builds[v]["exe_sha256"] or BASE.digest(build) != builds[v]["build_sha256"] for v,(exe,build) in versions.items()):
        raise ValueError("source/build/binary changed during capture")
    with (args.output / "frames.csv").open("w", encoding="utf-8", newline="") as stream:
        writer=csv.DictWriter(stream, fieldnames=csv_rows[0].keys());writer.writeheader();writer.writerows(csv_rows)
    result = dict(source_sha256=source, builds=builds, cases=args.cases, order=order, runs=runs, discarded=discarded, seed=0, iterations=1)
    if args.ui_regression:
        result["ui_regression"] = dict(max_regression_percent_each_block_p50_p95=10, scene_equivalence="within each version; gallery intentionally changes the scene", quiet_gate="ignore only cargo run wrappers with a launched foreign top-level target EXE; compiler/test/Showcase gates remain active")
    (args.output / "identity.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    if args.after:
        for case in args.cases:
            for block in range(3):
                for field in ["p50", "p95"]:
                    def timing(version): return median(r[field] for r in runs if r["case"]==case and r["block"]==block and r["version"]==version)
                    gain = (1-timing("after")/timing("before"))*100
                    if gain < (-10 if args.ui_regression else 20): raise ValueError(f"frozen performance gate failed: {case}/{block}/{field}/{gain}")
        print("All Table gallery P50/P95 regression gates passed (<=10%; different scenes)" if args.ui_regression else "All page/block P50/P95 improvement gates passed (>=20%)",flush=True)


if __name__ == "__main__": main()
