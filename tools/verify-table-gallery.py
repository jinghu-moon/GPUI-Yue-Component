"""Offline check of Table gallery UI regression evidence (intentionally different scenes)."""
import csv
import hashlib
import importlib.util
import json
from pathlib import Path
from statistics import median
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    root = Path(sys.argv[1]).resolve()
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    for name, expected in manifest["files"].items():
        if digest(root / name) != expected:
            raise ValueError(f"file identity differs: {name}")
    sys.path.insert(0, str(root / "tools"))
    spec = importlib.util.spec_from_file_location("sampler", root / "tools/sample-showcase-scroll.py")
    sampler = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(sampler)
    info = json.loads((root / "comparison/identity.json").read_text(encoding="utf-8"))
    if info["cases"] != ["table"] or info["order"] != ["before", "after", "after", "before"] or info["seed"] != 0 or info["iterations"] != 1:
        raise ValueError("wrong workload or schedule")
    if info["ui_regression"]["max_regression_percent_each_block_p50_p95"] != 10:
        raise ValueError("frozen 10% regression budget differs")
    schedule = [(b,s,v) for b in range(3) for s,v in enumerate(info["order"])]
    if [(r["block"],r["slot"],r["version"]) for r in info["runs"]] != schedule or any(r["case"]!="table" for r in info["runs"]):
        raise ValueError("incomplete or reordered process runs")
    for folder, names, expected in [("source", manifest["source_files"], info["source_sha256"]), ("baseline/source", manifest["baseline_source_files"], manifest["baseline_source_sha256"])]:
        source = hashlib.sha256()
        for name in names:
            source.update(name.encode())
            source.update((root / folder / name).read_bytes())
        if source.hexdigest() != expected:
            raise ValueError("source identity differs")
    baseline = json.loads((root / "baseline/identity.json").read_text(encoding="utf-8"))
    if baseline["source_sha256"] != manifest["baseline_source_sha256"] or baseline["builds"]["after"] != info["builds"]["before"]:
        raise ValueError("baseline differs from the previously verified optimized binary")
    if info["builds"]["before"]["profiles"] != info["builds"]["after"]["profiles"]:
        raise ValueError("effective dev profiles changed")
    for version in ["before", "after"]:
        build = root / f"builds/{version}.jsonl"
        if digest(build) != info["builds"][version]["build_sha256"]:
            raise ValueError("build identity differs")
        records = [json.loads(s) for s in build.read_text(encoding="utf-8-sig").splitlines() if s.startswith("{")]
        profiles = {r["target"]["name"]: r["profile"] for r in records if r.get("reason")=="compiler-artifact" and r["target"]["name"] in info["builds"][version]["profiles"]}
        if profiles != info["builds"][version]["profiles"]:
            raise ValueError("profiles differ from compiler records")
        artifacts = [r for r in records if r.get("executable") and r["target"]["name"]=="windows-showcase"]
        if len(artifacts) != 1 or not artifacts[0]["profile"]["test"]:
            raise ValueError("wrong Showcase test artifact")
    native = [json.loads(s) for s in (root / "builds/native.jsonl").read_text(encoding="utf-8-sig").splitlines() if s.startswith("{")]
    if not any(r.get("reason")=="build-finished" and r["success"] for r in native):
        raise ValueError("native build failed")
    native_profiles = {r["target"]["name"]: r["profile"] for r in native if r.get("reason")=="compiler-artifact" and r["target"]["name"] in info["builds"]["after"]["profiles"]}
    if native_profiles != {name:dict(profile, test=False) for name,profile in info["builds"]["after"]["profiles"].items()}:
        raise ValueError("native effective profiles differ")
    work, rows = {}, []
    for run in info["runs"]:
        log = root / "comparison" / run["log"]
        if digest(log) != run["log_sha256"]:
            raise ValueError("raw log identity differs")
        values = sampler.samples(log.read_text(encoding="utf-8"), "table")
        counts = [q for _,_,q in values]
        if run["version"] in work and work[run["version"]] != counts:
            raise ValueError("within-version scene work differs")
        work[run["version"]] = counts
        times = sorted(n for _,n,_ in values)
        if [run[k] for k in ["p50","p95","p99"]] != [times[i] for i in [119,227,236]]:
            raise ValueError("quantiles differ")
        rows.extend(dict(case="table",block=str(run["block"]),slot=str(run["slot"]),version=run["version"],frame=str(f),ns=str(n),quads=str(q)) for f,n,q in values)
    with (root / "comparison/frames.csv").open(encoding="utf-8",newline="") as stream:
        if list(csv.DictReader(stream)) != rows:
            raise ValueError("CSV differs from raw logs")
    regressions = {}
    for block in range(3):
        regressions[block] = {}
        for field in ["p50","p95"]:
            def timing(version):
                return median(r[field] for r in info["runs"] if r["block"]==block and r["version"]==version)
            regression = (timing("after")/timing("before")-1)*100
            if regression > 10:
                raise ValueError(f"performance gate failed: {block}/{field}/{regression}")
            regressions[block][field] = regression
    print(json.dumps(dict(files=len(manifest["files"]),processes=len(info["runs"]),samples=len(rows),scenes="intentionally different",quad_ranges={v:[min(q),max(q)] for v,q in work.items()},discarded=info["discarded"],regression_percent=regressions,all_gates="passed"),indent=2))


if __name__ == "__main__":
    main()
