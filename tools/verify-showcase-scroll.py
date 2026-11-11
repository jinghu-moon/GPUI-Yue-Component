"""Offline verification for the shared Showcase scrolling evidence bundle."""
import csv
import hashlib
import importlib.util
import json
from pathlib import Path
from statistics import median
import sys


def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    root = Path(sys.argv[1]).resolve()
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    for name, expected in manifest["files"].items():
        if digest(root / name) != expected: raise ValueError(f"identity mismatch: {name}")
    sys.path.insert(0, str(root / "tools"))
    spec = importlib.util.spec_from_file_location("scroll_sampler", root / "tools/sample-showcase-scroll.py")
    runner = importlib.util.module_from_spec(spec); spec.loader.exec_module(runner)
    info = json.loads((root / "comparison/identity.json").read_text(encoding="utf-8"))
    gate = json.loads((root / "acceptance.json").read_text(encoding="utf-8"))
    if gate != dict(cases=["button", "table", "list"], order=["before", "after", "after", "before"], required_gain_percent_each_page_each_block_p50_p95=20):
        raise ValueError("acceptance contract differs from the frozen sampler")
    if info["order"] != gate["order"] or info["cases"] != gate["cases"] or info["seed"] != 0 or info["iterations"] != 1:
        raise ValueError("wrong schedule/seed")
    expected = [(c,b,s,v) for c in gate["cases"] for b in range(3) for s,v in enumerate(gate["order"])]
    if [(r["case"],r["block"],r["slot"],r["version"]) for r in info["runs"]] != expected:
        raise ValueError("incomplete/duplicate/reordered process runs")
    source=hashlib.sha256()
    for name in manifest["source_files"]:
        source.update(name.encode()); source.update((root / "source" / name).read_bytes())
    if source.hexdigest() != info["source_sha256"]: raise ValueError("source fingerprint mismatch")
    for version in ["before", "after"]:
        build = root / f"builds/{version}.jsonl"
        if digest(build) != info["builds"][version]["build_sha256"]: raise ValueError("build identity mismatch")
        records = [json.loads(s) for s in build.read_text(encoding="utf-8-sig").splitlines() if s.startswith("{")]
        profiles = info["builds"][version]["profiles"]
        actual = {r["target"]["name"]: r["profile"] for r in records if r.get("reason")=="compiler-artifact" and r["target"]["name"] in profiles}
        if profiles != actual: raise ValueError("profiles differ from compiler records")
        artifacts = [r for r in records if r.get("executable") and r["target"]["name"]=="windows-showcase"]
        if len(artifacts) != 1 or not artifacts[0]["profile"]["test"]:
            raise ValueError("wrong Showcase test artifact")
        expected = {"gpui": "2", "gpui_base": "2", "gpui_windows": "2", "taffy": "3"} if version=="after" else dict.fromkeys(["gpui", "gpui_base", "gpui_windows", "taffy"], "0")
        for name, opt in expected.items():
            if profiles[name]["opt_level"] != opt or not profiles[name]["debug_assertions"] or not profiles[name]["overflow_checks"]:
                raise ValueError("unexpected effective package profile")
        if profiles["windows-showcase"]["opt_level"] != "0" or not profiles["windows-showcase"]["debug_assertions"] or not profiles["windows-showcase"]["overflow_checks"]:
            raise ValueError("application dev settings changed")
    diagnostic = json.loads((root / "diagnostic/identity.json").read_text(encoding="utf-8"))
    baseline = hashlib.sha256()
    for name in manifest["source_files"]:
        baseline.update(name.encode())
        baseline.update((root / "baseline/Cargo.toml" if name=="Cargo.toml" else root / "source" / name).read_bytes())
    if baseline.hexdigest() != diagnostic["source_sha256"] or diagnostic["builds"]["before"] != info["builds"]["before"]:
        raise ValueError("baseline source/build identity differs from diagnostic")
    if diagnostic["cases"] != gate["cases"] or diagnostic["order"] != ["before"] or len(diagnostic["runs"]) != 3:
        raise ValueError("incomplete original diagnostic")
    for run, case in zip(diagnostic["runs"], gate["cases"]):
        log = root / "diagnostic" / run["log"]
        if run["case"] != case or digest(log) != run["log_sha256"]:
            raise ValueError("diagnostic log identity differs")
        values = runner.samples(log.read_text(encoding="utf-8"), case)
        times = sorted(s[1] for s in values)
        if [run[k] for k in ["p50","p95","p99"]] != [times[i] for i in [119,227,236]]:
            raise ValueError("diagnostic quantiles differ")
    native = [json.loads(s) for s in (root / "builds/native.jsonl").read_text(encoding="utf-8-sig").splitlines() if s.startswith("{")]
    if not any(r.get("reason")=="build-finished" and r["success"] for r in native):
        raise ValueError("native build did not finish successfully")
    native_profiles = {r["target"]["name"]: r["profile"] for r in native if r.get("reason")=="compiler-artifact" and r["target"]["name"] in info["builds"]["after"]["profiles"]}
    for name, expected in info["builds"]["after"]["profiles"].items():
        if native_profiles.get(name) != dict(expected, test=False):
            raise ValueError("native build differs from the tested effective dev settings")
    work, rows = {}, []
    for run in info["runs"]:
        log = root / "comparison" / run["log"]
        if digest(log) != run["log_sha256"]: raise ValueError("raw log differs")
        values = runner.samples(log.read_text(encoding="utf-8"), run["case"])
        counts = [s[2] for s in values]
        if run["case"] in work and work[run["case"]] != counts: raise ValueError("scene work differs")
        work[run["case"]] = counts
        times=sorted(s[1] for s in values)
        if [run[k] for k in ["p50","p95","p99"]] != [times[i] for i in [119,227,236]]: raise ValueError("quantiles differ")
        rows.extend(dict(case=run["case"],block=str(run["block"]),slot=str(run["slot"]),version=run["version"],frame=str(f),ns=str(n),quads=str(q)) for f,n,q in values)
    with (root / "comparison/frames.csv").open(encoding="utf-8",newline="") as stream:
        if list(csv.DictReader(stream)) != rows: raise ValueError("CSV differs from raw evidence")
    result={}
    for case in gate["cases"]:
        result[case]={}
        for block in range(3):
            result[case][str(block)]={}
            for field in ["p50","p95"]:
                def timing(version):return median(r[field] for r in info["runs"] if r["case"]==case and r["block"]==block and r["version"]==version)
                gain=(1-timing("after")/timing("before"))*100
                if gain < gate["required_gain_percent_each_page_each_block_p50_p95"]: raise ValueError(f"performance gate failed: {case}/{block}/{field}")
                result[case][str(block)][field]=gain
    print(json.dumps(dict(verified_files=len(manifest["files"]),processes=len(info["runs"]),samples=len(rows),discarded=info["discarded"],all_gates="passed",gain_percent=result),indent=2))


if __name__ == "__main__": main()
