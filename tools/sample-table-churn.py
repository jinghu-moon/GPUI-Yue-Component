"""TB05 full-snapshot churn, separate Fixed/Measured baselines (no A/B gain claim)."""
import argparse
import csv
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import subprocess

SPEC = importlib.util.spec_from_file_location("table_sampler", Path(__file__).with_name("sample-table.py"))
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
CASE = "churn::tb05_snapshot_churn"
PATTERN = re.compile(r"TABLE_CHURN,measured=(\d+),step=(\d+),prepare_ns=(\d+),accept_ns=(\d+),draw_effects_ns=(\d+),historical_sources=(\d+)")


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--build-record", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--attempts", type=int, default=3)
    args = parser.parse_args()
    if args.attempts < 1 or args.output.exists():
        parser.error("positive attempts and a fresh evidence directory are required")
    executable = args.executable.resolve(strict=True)
    records = [json.loads(s) for s in args.build_record.read_text(encoding="utf-8-sig").splitlines() if s.startswith("{")]
    artifact = [r for r in records if r.get("executable") and Path(r["executable"]).resolve() == executable]
    if len(artifact) != 1 or artifact[0]["target"]["name"] != "table" or artifact[0]["profile"]["opt_level"] != "3":
        parser.error("the release table test build record must identify this executable")
    args.output.mkdir(parents=True)
    source = BASE.source_digest()
    binary = BASE.digest(executable)
    runner = BASE.digest(Path(__file__))
    samples, runs, discarded = [], [], []
    for block in range(3):
        for measured in [0, 1]:
            label = f"block-{block}-measured-{measured}"
            for attempt in range(1, args.attempts + 1):
                BASE.wait_quiet()
                log = args.output / f"{label}-attempt-{attempt}.txt"
                with log.open("w", encoding="utf-8") as stream:
                    process = subprocess.Popen([str(executable), CASE, "--exact", "--ignored", "--nocapture", "--test-threads=1"], cwd=BASE.ROOT, env=dict(os.environ, TABLE_CHURN_MEASURED=str(measured)), stdout=stream, stderr=subprocess.STDOUT)
                    interfered = False
                    while process.poll() is None:
                        try:
                            process.wait(timeout=0.5)
                        except subprocess.TimeoutExpired:
                            interfered |= BASE.busy()
                    interfered |= BASE.busy()
                if process.returncode:
                    raise RuntimeError(f"{label} failed; no acceptance")
                if not interfered:
                    break
                discarded.append(log.name)
                print(f"Discarding {log.name}: compilation/Showcase observed", flush=True)
            else:
                raise RuntimeError("attempt limit; no acceptance")
            output = log.read_text(encoding="utf-8")
            BASE.check_native_output(output, CASE)
            data = [list(map(int, row)) for row in PATTERN.findall(output)]
            if len(data) != 100 or [r[1] for r in data] != list(range(100)) or any(r[0] != measured or min(r[2:5]) <= 0 or r[5] for r in data):
                raise ValueError("bad churn steps, timing or retained sources")
            samples.extend(dict(block=block, measured=r[0], step=r[1], prepare_ns=r[2], accept_ns=r[3], draw_effects_ns=r[4], historical_sources=r[5]) for r in data)
            runs.append(dict(block=block, measured=measured, log=log.name, log_sha256=BASE.digest(log)))
            print(f"{label}: 100 publications; all historical sources released", flush=True)
    if BASE.source_digest() != source or BASE.digest(executable) != binary or BASE.digest(Path(__file__)) != runner:
        raise ValueError("source, runner or executable changed")
    with (args.output / "samples.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=samples[0].keys())
        writer.writeheader()
        writer.writerows(samples)
    identity = dict(kind="tb05-full-snapshot-churn", source_sha256=source, executable_sha256=binary, runner_sha256=runner, build_record_sha256=BASE.digest(args.build_record), profile=artifact[0]["profile"], head=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=BASE.ROOT, text=True).strip(), os=platform.platform(), processor=platform.processor(), case=CASE, samples=600, independent_blocks=3, fixture="10000 rows x16 columns; Fixed and Measured independent variants; change visible row0, offscreen9000 and64 rows128..191;100 source replacements; preparation/accept/draw+effects separate; no GPU or timing-gain claim", runs=runs, discarded=discarded, accepted="all records current, no historical sources retained, idle has no renders; timing baseline only")
    (args.output / "identity.json").write_text(json.dumps(identity, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
