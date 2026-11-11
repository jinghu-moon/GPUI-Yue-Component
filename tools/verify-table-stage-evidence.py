"""Offline verification of the Table stage evidence bundle and frozen gates."""
import csv
import hashlib
import importlib.util
import json
from pathlib import Path
from statistics import median
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def quantile(values, fraction):
    return sorted(values)[int((len(values) - 1) * fraction)]


def main():
    root = Path(sys.argv[1]).resolve()
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    for name, expected in manifest["files"].items():
        if digest(root / name) != expected:
            raise ValueError(f"file identity mismatch: {name}")
    sys.path.insert(0, str(root / "tools"))
    spec = importlib.util.spec_from_file_location("stage_runner", root / "tools/profile-table.py")
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    for version, source in manifest["sources"].items():
        value = hashlib.sha256()
        for name in source["files"]:
            value.update(name.encode())
            value.update((root / "sources" / version / name).read_bytes())
        if value.hexdigest() != source["sha256"]:
            raise ValueError("archived source identity differs")
    summaries = {}
    for directory in ["diagnostic", "active", "inactive", "confirmation"]:
        folder = root / directory
        identity = json.loads((folder / "identity.json").read_text(encoding="utf-8"))
        if identity["runner_sha256"] != digest(root / "tools/profile-table.py") or identity["discarded"]:
            raise ValueError("runner differs or discarded runs require explicit review")
        if directory != "diagnostic" and (identity["blocks"] != 3 or identity["order"] != ["before", "after", "after", "before"]):
            raise ValueError("three ABBA blocks required")
        expected_cases = {"diagnostic": ["cells", "fixed", "measured", "query", "measured-frames", "tree", "edit", "resize", "range"], "active": ["query"], "inactive": ["query", "cells"], "confirmation": ["cells"]}[directory]
        if identity["cases"] != expected_cases:
            raise ValueError("unexpected workload collection")
        expected_runs = [(case, block, slot, version) for case in expected_cases for block in range(identity["blocks"]) for slot, version in enumerate(identity["order"])]
        if [(r["case"], r["block"], r["slot"], r["version"]) for r in identity["runs"]] != expected_runs:
            raise ValueError("missing, duplicate or reordered process runs")
        for version, values in identity["versions"].items():
            if digest(root / f"builds/{version}.jsonl") != values["build_sha256"]:
                raise ValueError("build record mismatch")
        expected_source = manifest["sources"]["before" if directory == "diagnostic" else "after"]["sha256"]
        if identity["source_sha256"] != expected_source:
            raise ValueError("sampling source differs from archived source")
        worksets = {}
        expected_csv = []
        for run in identity["runs"]:
            log = folder / run["log"]
            if digest(log) != run["log_sha256"]:
                raise ValueError("raw log changed")
            output = log.read_text(encoding="utf-8")
            case = runner.CASES[run["case"]][0]
            if identity["capture"] == "on":
                elapsed, unattributed, stages = runner.validate(output, case)
                expected_csv.extend(dict(case=run["case"], block=str(run["block"]), slot=str(run["slot"]), version=run["version"], stage=s[0], calls=s[1], inclusive_ns=s[2], self_ns=s[3], max_ns=s[4]) for s in stages)
                if (elapsed, unattributed) != (run["elapsed_ns"], run["unattributed_ns"]):
                    raise ValueError("capture totals differ")
            else:
                runner.BASE.check_native_output(output, case)
                if "TABLE_CAPTURE," in output or runner.STAGES.search(output):
                    raise ValueError("inactive run contains active capture")
            work = runner.workload(output, run["case"])
            if json.dumps(work, sort_keys=True) != json.dumps(run["measurements"], sort_keys=True):
                raise ValueError("stored measurements differ from raw log")
            if run["case"] in worksets and worksets[run["case"]] != work["work"]:
                raise ValueError("cross-version workset/result mismatch")
            worksets[run["case"]] = work["work"]
        with (folder / "stages.csv").open(encoding="utf-8", newline="") as stream:
            if list(csv.DictReader(stream)) != expected_csv:
                raise ValueError("stage CSV differs from raw log")
        summaries[directory] = identity
    if summaries["confirmation"]["versions"] != summaries["inactive"]["versions"] or summaries["active"]["versions"] != summaries["inactive"]["versions"]:
        raise ValueError("confirmation/comparison executable identity changed")
    probe = root / "allocation-probe"
    probe_identity = json.loads((probe / "run/identity.json").read_text(encoding="utf-8"))
    value = hashlib.sha256()
    for name in manifest["sources"]["after"]["files"]:
        overlay = probe / "source-overlay" / name
        value.update(name.encode())
        value.update((overlay if overlay.exists() else root / "sources/after" / name).read_bytes())
    if value.hexdigest() != probe_identity["source_sha256"]:
        raise ValueError("temporary probe source identity differs")
    if probe_identity["cases"] != ["cells"] or len(probe_identity["runs"]) != 1 or probe_identity["capture"] != "on":
        raise ValueError("unexpected temporary probe workload")
    if digest(probe / "full-build.jsonl") != probe_identity["versions"]["before"]["build_sha256"]:
        raise ValueError("temporary probe build identity differs")
    probe_run = probe_identity["runs"][0]
    probe_log = probe / "run" / probe_run["log"]
    if digest(probe_log) != probe_run["log_sha256"]:
        raise ValueError("temporary probe log identity differs")
    probe_output = probe_log.read_text(encoding="utf-8")
    elapsed, unattributed, probe_stages = runner.validate(probe_output, runner.CASES["cells"][0])
    if (elapsed, unattributed) != (probe_run["elapsed_ns"], probe_run["unattributed_ns"]):
        raise ValueError("temporary probe accounting differs")
    probe_csv = [dict(case="cells", block="0", slot="0", version="before", stage=s[0], calls=s[1], inclusive_ns=s[2], self_ns=s[3], max_ns=s[4]) for s in probe_stages]
    with (probe / "run/stages.csv").open(encoding="utf-8", newline="") as stream:
        if list(csv.DictReader(stream)) != probe_csv:
            raise ValueError("temporary probe stage CSV differs")
    if "ALLOCATION_ORIGIN,bytes=42000,stage=gpui.dependencies" not in probe_output:
        raise ValueError("allocation origin observation missing")
    gate = json.loads((root / "acceptance.json").read_text(encoding="utf-8"))
    results = {}
    first_allocation_differences = []
    for directory in ["active", "inactive", "confirmation"]:
        runs = summaries[directory]["runs"]
        result = {}
        for block in range(3):
            result[str(block)] = {}
            if directory != "confirmation":
                selected = [r for r in runs if r["block"] == block and r["case"] == "query"]
                def query_total(version):
                    return median(sum(s[2] for s in r["measurements"]["queries"]) for r in selected if r["version"] == version)
                before, after = query_total("before"), query_total("after")
                improvement = (1 - after / before) * 100
                if improvement < gate["query"]["min_improvement_percent_each_block"]:
                    raise ValueError(f"query gate failed: {directory}/{block}/{improvement}")
                result[str(block)] = dict(query_before_ns=before, query_after_ns=after, improvement_percent=improvement)
            if directory in ["inactive", "confirmation"]:
                cells = [r for r in runs if r["block"] == block and r["case"] == "cells"]
                for fraction, name in [(0.5, "p50"), (0.95, "p95")]:
                    def frame_time(version):
                        return median(quantile([s[2] for s in r["measurements"]["frames"]], fraction) for r in cells if r["version"] == version)
                    before, after = frame_time("before"), frame_time("after")
                    regression = (after / before - 1) * 100
                    if regression > gate["cells"]["max_regression_percent_each_block"]:
                        raise ValueError(f"cells gate failed: {block}/{name}/{regression}")
                    result[str(block)][name] = dict(before_ns=before, after_ns=after, regression_percent=regression)
                for sample in range(10):
                    before = median(r["measurements"]["allocations"][sample][2] for r in cells if r["version"] == "before")
                    after = median(r["measurements"]["allocations"][sample][2] for r in cells if r["version"] == "after")
                    if after > before:
                        if directory == "confirmation":
                            raise ValueError("confirmation cells allocation count increased; do not accept")
                        first_allocation_differences.append(dict(block=block, sample=sample, before=before, after=after))
        results[directory] = result
    print(json.dumps(dict(verified_files=len(manifest["files"]), first_batch_allocation_criterion="failed" if first_allocation_differences else "passed", first_allocation_differences=first_allocation_differences, confirmation="passed", gates=results), indent=2))


if __name__ == "__main__":
    main()
