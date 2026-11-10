"""Table size-scaling baselines for the production Table on the headless renderer.

Build first; this runner never compiles during sampling. A/B mean 1,000/100,000
rows of the same implementation, not before/after revisions or native FPS.
"""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time

from virtualization_evidence import check_native_output

ROOT = Path(__file__).resolve().parents[1]
CASE = "benchmark::tb01_headless_frame_sample"
QUERY = r"""@(Get-CimInstance Win32_Process | Where-Object {
    $_.Name -in @('rustc.exe', 'clippy-driver.exe', 'windows-showcase.exe') -or
    ($_.Name -eq 'cargo.exe' -and $_.CommandLine -notmatch '(?i)\bcargo(?:\.exe)?["\x27]?\s+dev(?:\s|$)')
}).Count"""


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_digest():
    files = {ROOT / "Cargo.lock", ROOT / "Cargo.toml", Path(__file__).resolve(), ROOT / "tools/virtualization_evidence.py"}
    for manifest in ["crates/yue-component/Cargo.toml", "vendor/gpui-base/Cargo.toml", "vendor/gpui-pre/Cargo.toml", "vendor/gpui-pre-windows/Cargo.toml"]:
        files.add(ROOT / manifest)
    for directory in ["crates/yue-component/src", "crates/yue-component/tests", "vendor/gpui-base/src", "vendor/gpui-pre/src", "vendor/gpui-pre-windows/src"]:
        files.update((ROOT / directory).rglob("*.rs"))
    value = hashlib.sha256()
    for path in sorted(files):
        value.update(path.relative_to(ROOT).as_posix().encode())
        value.update(path.read_bytes())
    return value.hexdigest()


def busy():
    output = subprocess.run(["powershell", "-NoProfile", "-Command", QUERY], check=True, capture_output=True, text=True)
    return output.stdout.strip() != "0"


def wait_quiet():
    deadline = time.monotonic() + 180
    quiet = 0
    announced = False
    while quiet < 2:
        occupied = busy()
        quiet = 0 if occupied else quiet + 1
        if occupied and not announced:
            print("Waiting for existing compilation/Showcase; no processes are stopped", flush=True)
            announced = True
        if time.monotonic() >= deadline:
            raise RuntimeError("quiet gate timed out; no samples accepted")
        if quiet < 2:
            time.sleep(2)


def quantile(values, fraction):
    return sorted(values)[int((len(values) - 1) * fraction)]


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--executable", type=Path, required=True, help="prebuilt table integration-test executable")
    parser.add_argument("--output", type=Path, required=True, help="new evidence directory (must not exist)")
    parser.add_argument("--profile", choices=["dev", "release"], required=True)
    parser.add_argument("--workload", choices=["tb01", "m2-layout", "tb03", "tb06", "tb02"], default="tb01")
    parser.add_argument("--attempts", type=int, default=3, help="whole-run retries after interference; acceptance/quiet gates are unchanged")
    parser.add_argument("--build-record", type=Path, required=True, help="cargo --message-format=json output from the preceding no-run build")
    args = parser.parse_args()
    if args.attempts < 1:
        parser.error("attempts must be positive")
    case = {"tb02": "cells::tb02_cells_frame_sample", "tb01": CASE, "m2-layout": "benchmark::m2_headless_layout_frame_sample", "tb03": "complex::tb03_measured_frame_sample", "tb06": "complex::tb06_tree_frame_sample"}[args.workload]
    large_count = {"tb03": 20_000, "tb06": 50_000, "tb02": 50_000}.get(args.workload, 100_000)
    order = [1000, large_count, large_count, 1000]
    executable = args.executable.resolve(strict=True)
    build_records = [json.loads(line) for line in args.build_record.read_text(encoding="utf-8-sig").splitlines() if line.startswith("{")]
    artifact = [record for record in build_records if record.get("executable") and Path(record["executable"]).resolve() == executable]
    if len(artifact) != 1 or artifact[0].get("target", {}).get("name") != "table" or not artifact[0].get("profile", {}).get("test"):
        parser.error("build record must identify this exact table test executable")
    if args.output.exists():
        parser.error("evidence directory already exists; choose a new output")
    args.output.mkdir(parents=True)
    initial_sources = source_digest()
    executable_hash = digest(executable)
    rows = []
    runs = []
    discarded = []
    for block in range(3):
        for slot, count in enumerate(order):
            label = f"block-{block}-slot-{slot}-rows-{count}"
            env = dict(os.environ, YUE_TABLE_BENCH_ROWS=str(count))
            for attempt in range(1, args.attempts + 1):
                wait_quiet()
                log = args.output / f"{label}-attempt-{attempt}.txt"
                started = time.time()
                with log.open("w", encoding="utf-8") as stream:
                    process = subprocess.Popen([str(executable), case, "--exact", "--ignored", "--nocapture", "--test-threads=1"], cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
                    interfered = False
                    while process.poll() is None:
                        try:
                            process.wait(timeout=0.5)
                        except subprocess.TimeoutExpired:
                            interfered |= busy()
                    interfered |= busy()
                if process.returncode:
                    raise RuntimeError(f"{label}: test failed; raw log retained, no acceptance")
                if not interfered:
                    break
                discarded.append(log.name)
                print(f"Discarding {log.name}: compilation/Showcase observed", flush=True)
            else:
                raise RuntimeError(f"{args.attempts} interrupted attempts; raw logs retained, no acceptance")
            output = log.read_text(encoding="utf-8")
            check_native_output(output, case)  # Exactly one executed passing test; zero matches cannot pass.
            samples = re.findall(r"TABLE_FRAME,rows=(\d+),frame=(\d+),ns=(\d+),renders=(\d+)", output)
            if len(samples) != 1000 or [int(s[1]) for s in samples] != list(range(1000)):
                raise ValueError("missing, duplicate or reordered frames")
            mount = re.findall(r"TABLE_MOUNT,rows=(\d+),ns=(\d+)", output)
            if len(mount) != 1 or int(mount[0][0]) != count or int(mount[0][1]) <= 0:
                raise ValueError("mount sample missing")
            timings = []
            for observed_count, frame, ns, renders in samples:
                if int(observed_count) != count or int(ns) <= 0 or not 0 < int(renders) < 600:
                    raise ValueError("invalid sample or unbounded working set")
                timings.append(int(ns))
                rows.append(dict(block=block, slot=slot, rows=count, frame=int(frame), ns=int(ns), renders=int(renders)))
            allocations = []
            if args.workload in ["tb03", "tb06"]:
                allocations = re.findall(r"TABLE_ALLOC,rows=(\d+),sample=(\d+),allocations=(\d+),bytes=(\d+)", output)
                if len(allocations) != 10 or [int(a[1]) for a in allocations] != list(range(10)) or any(int(a[0]) != count or int(a[2]) <= 0 or int(a[3]) <= 0 for a in allocations):
                    raise ValueError("allocation attribution missing or invalid")
                if max(int(s[3]) for s in samples) > 152:
                    raise ValueError("working set exceeds ceil((448+4*40)/40)+2+1 row tracks times eight columns")
                if args.workload == "tb03" and "TABLE_RESOURCE,pool=512,width=64,height=64,pixel_bytes=8388608,gpu_bytes=unmeasured" not in output:
                    raise ValueError("raster bytes fixture missing")
                if args.workload == "tb06" and f"TABLE_TREE,nodes={count},roots={count//100},visible={count//100+990}" not in output:
                    raise ValueError("tree projection counts wrong")
            if args.workload == "tb02":
                allocations = re.findall(r"TABLE_ALLOC,rows=(\d+),sample=(\d+),allocations=(\d+),bytes=(\d+)", output)
                if len(allocations) != 10 or [int(a[1]) for a in allocations] != list(range(10)):
                    raise ValueError("TB02 allocation attribution missing")
                if max(int(s[3]) for s in samples) > 377:
                    raise ValueError("TB02 exceeds (ceil((448+160)/40)+3)*(ceil((1024+160)/80)+4)+16")
                if f"TABLE_MATRIX,rows={count},columns=256,sizing=Known,pins=2+2,frozen_rows=1+1,spans=16" not in output:
                    raise ValueError("TB02 matrix fixture identity missing")
            runs.append(dict(block=block, slot=slot, rows=count, mount_ns=int(mount[0][1]), allocations=allocations, p50_ns=quantile(timings, .5), p95_ns=quantile(timings, .95), p99_ns=quantile(timings, .99), log_sha256=digest(log), wall_seconds=round(time.time()-started, 3)))
            print(f"{label}: p50={runs[-1]['p50_ns']}ns, p95={runs[-1]['p95_ns']}ns", flush=True)
    # Work must be identical at every point in the trace, across all repetitions.
    expected = [r["renders"] for r in rows[:1000]]
    if args.workload in ["tb01", "m2-layout", "tb02"] and any([r["renders"] for r in rows[start:start+1000]] != expected for start in range(0, len(rows), 1000)):
        raise ValueError("different worksets across ABBA samples")
    if source_digest() != initial_sources or digest(executable) != executable_hash:
        raise ValueError("sources or executable changed during sampling")
    with (args.output / "frames.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=rows[0].keys())
        writer.writeheader(); writer.writerows(rows)
    ratios = {name: statistics.median(run[name] for run in runs if run["rows"] == large_count) / statistics.median(run[name] for run in runs if run["rows"] == 1000) for name in ["p50_ns", "p95_ns", "p99_ns"]}
    identity = dict(kind=f"{args.workload}-headless-size-scaling", profile=args.profile, cargo_profile=artifact[0]["profile"], build_record_sha256=digest(args.build_record), head=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), dirty=bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT)),
        source_sha256=initial_sources, executable_sha256=executable_hash, lock_sha256=digest(ROOT / "Cargo.lock"), os=platform.platform(), processor=platform.processor(), discarded=discarded,
        case=case, max_attempts=args.attempts, order=order, independent_blocks=3, warmup_frames=120, frames_per_run=1000, total_frames=len(rows),
        fixture=({"tb02": "64rem host; 28rem table; 256 text columns; Known row heights 40/48/56px; widths 80/96/112px; two pinned columns each end; start/end data row; 16 sparse 3x2 body spans; deterministic XY trace", "tb03": "64rem x 32rem; 28rem table; rem16 light/neutral; Rows Measured; 8 columns; 1/2/4 lines; Avatar/Tag/Button; fixed 512 64x64 raster pool 8388608 CPU bytes; GPU residency unmeasured", "tb06": "64rem x32rem; 28rem table; rem16 light/neutral; 100 nodes/root; first 10 expanded; 8 text columns; structure build included in mount"}.get(args.workload) or ("64rem x 32rem host; 28rem body; rem16; light/neutral; reduced motion; Rows Fixed; 12 text columns; repeated vertical trace" if args.workload == "tb01" else "48/64/80rem host every 10 frames; 28rem body; rem16; light/neutral; reduced motion; Rows Known grouped header; 12 text columns; start/end pinned column; start/end data row; fixed footer; deterministic XY trace; container feedback outside draw timing")), gate="two quiet observations; monitor compilation and Showcase throughout each sample; reject interference", ratios=ratios, runs=runs,
        accepted="correctness and bounded working set only; timing is an initial baseline, no regression threshold or native FPS claim")
    (args.output / "identity.json").write_text(json.dumps(identity, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(ratios), flush=True)


if __name__ == "__main__":
    main()
