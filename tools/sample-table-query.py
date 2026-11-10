"""TB04 query pipeline baseline; build release table tests before sampling.

Measures CPU query stages and final accepted-view publication, not native FPS.
First execution of each condition is distinguished from repeated execution;
neither is a claim of a flushed CPU/OS cache.
"""
import argparse
import csv
import importlib.util
import json
from pathlib import Path
import platform
import re
import shutil
import statistics
import subprocess
import time

from virtualization_evidence import check_native_output

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("table_evidence", ROOT / "tools/sample-table.py")
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
CASE = "query::tb04_query_pipeline_sample"


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--build-record", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    executable = args.executable.resolve(strict=True)
    records = [json.loads(line) for line in args.build_record.read_text(encoding="utf-8-sig").splitlines() if line.startswith("{")]
    artifacts = [r for r in records if r.get("executable") and Path(r["executable"]).resolve() == executable]
    if len(artifacts) != 1 or artifacts[0].get("target", {}).get("name") != "table" or not artifacts[0]["profile"]["test"] or artifacts[0]["profile"]["opt_level"] == "0":
        parser.error("build record must identify the exact optimized table test executable")
    if args.output.exists():
        parser.error("choose a new evidence directory")
    args.output.mkdir(parents=True)
    identity = (common.source_digest(), common.digest(executable), common.digest(Path(__file__)))
    rows = []
    discarded = []
    runs = []
    for run in range(3):
        for attempt in range(1, 4):
            common.wait_quiet()
            log = args.output / f"run-{run}-attempt-{attempt}.txt"
            started = time.time()
            with log.open("w", encoding="utf-8") as stream:
                process = subprocess.Popen([str(executable), CASE, "--exact", "--ignored", "--nocapture", "--test-threads=1"], cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
                interference = False
                while process.poll() is None:
                    try:
                        process.wait(timeout=.5)
                    except subprocess.TimeoutExpired:
                        interference |= common.busy()
                interference |= common.busy()
            if process.returncode:
                raise RuntimeError("probe failed; raw output retained")
            if not interference:
                break
            discarded.append(log.name)
        else:
            raise RuntimeError("three interrupted attempts; no acceptance")
        output = log.read_text(encoding="utf-8")
        check_native_output(output, CASE)
        samples = re.findall(r"TABLE_QUERY,mode=(\d+),sample=(\d+),query=(\d+),validate=(\d+),filter=(\d+),sort=(\d+),projection=(\d+),accept=(\d+),total=(\d+),rows=(\d+)", output)
        if [(int(s[0]), int(s[1])) for s in samples] != [(mode, sample) for mode in range(11) for sample in range(6)]:
            raise ValueError("missing, duplicate or reordered samples")
        fields = ["mode", "sample", "query", "validate", "filter", "sort", "projection", "accept", "total", "rows"]
        for values in samples:
            row = dict(zip(fields, map(int, values)))
            if any(row[k] <= 0 for k in ["query", "validate", "filter", "sort", "projection", "accept"]):
                raise ValueError("invalid stage time")
            if sum(row[k] for k in ["validate", "filter", "sort", "projection"]) > row["query"]:
                raise ValueError("stage time exceeds whole query interval")
            rows.append(dict(run=run, **row))
        runs.append(dict(run=run, log=log.name, sha256=common.digest(log), wall_seconds=time.time()-started))
        print(f"run {run}: 66 complete oracle-checked samples", flush=True)
    if identity != (common.source_digest(), common.digest(executable), common.digest(Path(__file__))):
        raise ValueError("source or executable changed while sampling")
    for mode in range(11):
        if len({(r["total"], r["rows"]) for r in rows if r["mode"] == mode}) != 1:
            raise ValueError("result counts changed across runs")
    with (args.output / "queries.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=rows[0].keys())
        writer.writeheader()
        writer.writerows(rows)
    summary = {str(mode): {stage: statistics.median(r[stage] for r in rows if r["mode"] == mode and r["sample"] > 0) for stage in ["query", "filter", "sort", "projection", "accept"]} for mode in range(11)}
    metadata = dict(case=CASE, source_digest=identity[0], executable_sha256=identity[1], runner_sha256=identity[2], platform=platform.platform(), profile="release", artifacts=artifacts, runs=runs, discarded=discarded, repeated_medians_ns=summary)
    (args.output / "metadata.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    shutil.copy2(args.build_record, args.output / "build.jsonl")
    for name, command in [("head.txt", ["git", "rev-parse", "HEAD"]), ("diff.patch", ["git", "diff", "--binary"]), ("status.txt", ["git", "status", "--short"]), ("rustc.txt", ["rustc", "-Vv"])]:
        (args.output / name).write_bytes(subprocess.check_output(command, cwd=ROOT))
    print(json.dumps(summary, indent=2), flush=True)


if __name__ == "__main__":
    main()
