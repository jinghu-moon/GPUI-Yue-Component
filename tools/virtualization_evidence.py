"""Fail closed on missing native tests or incomplete span comparison evidence.

Timing is application CPU work, not GPU latency or presented FPS. Shared runner
ratios are coarse alarms; review raw samples on a fixed machine before adoption.
"""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import re
from statistics import median


def check_native_output(output, expected_filter):
    output = re.sub(r"\x1b\[[0-9;]*m", "", output)
    results = re.findall(
        r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out", output)
    if len(results) != 1 or results[0][:5] != ("ok", "1", "0", "0", "0"):
        raise ValueError("native probe must execute exactly one passing, non-ignored test")
    if len(re.findall(r"\btest " + re.escape(expected_filter) + r" \.\.\.", output)) != 1:
        raise ValueError("expected native test identity missing or duplicated")


def check_identity(identity):
    if identity.get("order") != ["before/0", "after/0", "after/1", "before/1"]:
        raise ValueError("ABBA order missing")
    if not re.fullmatch(r"[0-9a-f]{40}", identity.get("before", "")):
        raise ValueError("committed baseline identity missing")
    if not re.fullmatch(r"[0-9a-f]{64}", identity.get("workload_sha256", "")):
        raise ValueError("workload hash missing")
    builds = identity.get("build_identity", {})
    for version in ["before", "after"]:
        for field in ["manifest_sha256", "lock_sha256", "dependency_records_sha256", "vendor_sources_sha256"]:
            if not re.fullmatch(r"[0-9a-f]{64}", builds.get(version, {}).get(field, "")):
                raise ValueError("incomplete build identity")
    if builds["before"]["dependency_records_sha256"] != builds["after"]["dependency_records_sha256"]:
        raise ValueError("dependency identities differ")


def check_span_samples(rows, identity, max_regression_percent=None):
    check_identity(identity)
    cases = {f"{stage}-{same_row}-{count}" for stage in ["span", "cold"]
             for same_row in ["false", "true"] for count in [0, 100, 1000, 10000]}
    expected = {(version, str(round_), case, str(sample))
                for version in ["before", "after"] for round_ in [0, 1]
                for case in cases for sample in range(5)}
    observed = set()
    values = {}
    config_values = {}
    runs = []
    # Repeated samples from the same deterministic draw trace must build exactly
    # the same cells, even if both timing and an oracle assertion were removed.
    builds = {}
    for row in rows:
        key = tuple(row.get(field, "") for field in ["version", "round", "case", "sample"])
        if key not in expected or key in observed:
            raise ValueError(f"unexpected or duplicate sample: {key}")
        observed.add(key)
        if not runs or runs[-1] != key[:2]:
            runs.append(key[:2])
        nums = {}
        for field in ["ns", "p50", "p95", "p99", "config_ns", "render_ns", "builds"]:
            value = row.get(field, "")
            if not re.fullmatch(r"\d+", value):
                raise ValueError(f"invalid {field}: {value}")
            nums[field] = int(value)
        if nums["ns"] <= 0 or not 0 < nums["p50"] <= nums["p95"] <= nums["p99"] <= nums["ns"]:
            raise ValueError("invalid timing quantiles")
        if nums["config_ns"] + nums["render_ns"] != nums["ns"]:
            raise ValueError("configuration/render totals differ")
        cold = row["case"].startswith("cold-")
        if (cold and (nums["builds"] != 0 or nums["render_ns"] != 0)) or (not cold and nums["builds"] <= 0):
            raise ValueError("unexpected draw work count")
        if row["case"] in builds and builds[row["case"]] != nums["builds"]:
            raise ValueError("before/after worksets differ")
        builds[row["case"]] = nums["builds"]
        values.setdefault((row["version"], row["case"]), []).append(nums["ns"])
        config_values.setdefault((row["version"], row["case"]), []).append(nums["config_ns"])
    if observed != expected or identity.get("rows") != len(expected):
        raise ValueError(f"incomplete balanced samples: {len(observed)}/{len(expected)}")
    if runs != [("before", "0"), ("after", "0"), ("after", "1"), ("before", "1")]:
        raise ValueError("sample blocks do not follow the recorded ABBA order")
    ratios = {case: median(values["after", case]) / median(values["before", case]) for case in sorted(cases)}
    if max_regression_percent is not None:
        if not math.isfinite(max_regression_percent) or max_regression_percent < 0:
            raise ValueError("invalid regression threshold")
        # This alarm includes cold initialization and zero-span controls.
        limit = 1 + max_regression_percent / 100
        # The zero-span cold control totals only ~10 microseconds for 16
        # configurations on the development host. Require an absolute change
        # of 50 microseconds as well; percentages of sub-microsecond operations
        # are unstable on shared runners. This is not a product frame budget.
        failed = {case: ratio for case, ratio in ratios.items() if ratio > limit
                  and median(values["after", case]) - median(values["before", case]) > 50_000}
        if failed:
            raise ValueError(f"coarse CPU regression alarm: {failed}")
        if identity.get("after_feature") == "span-cache":
            for case in ["span-false-10000", "span-true-10000"]:
                before = median(config_values["before", case])
                after = median(config_values["after", case])
                if before <= 0 or after > before * 0.5:
                    raise ValueError(f"large stable span configuration reuse disappeared: {case}")
    return ratios


def main():
    parser = argparse.ArgumentParser(__doc__)
    sub = parser.add_subparsers(dest="kind", required=True)
    native = sub.add_parser("native")
    native.add_argument("--log", type=Path, required=True)
    native.add_argument("--filter", required=True)
    perf = sub.add_parser("perf")
    perf.add_argument("--csv", type=Path, required=True)
    perf.add_argument("--identity", type=Path, required=True)
    perf.add_argument("--max-regression-percent", type=float)
    args = parser.parse_args()
    if args.kind == "native":
        check_native_output(args.log.read_text(encoding="utf-8-sig"), args.filter)
        print(f"Verified one executed native test: {args.filter}")
    else:
        with args.csv.open(encoding="utf-8", newline="") as stream:
            rows = list(csv.DictReader(stream))
        identity = json.loads(args.identity.read_text(encoding="utf-8"))
        ratios = check_span_samples(rows, identity, args.max_regression_percent)
        workload = Path(__file__).resolve().parents[1] / identity["workload"]
        raw = workload.read_bytes()
        lf = raw.replace(b"\r\n", b"\n")
        if identity["workload_sha256"] not in {hashlib.sha256(data).hexdigest() for data in [raw, lf, lf.replace(b"\n", b"\r\n")]}:
            raise ValueError("workload differs beyond checkout line endings")
        print(json.dumps({"rows": len(rows), "after_before_cpu_ratios": ratios}, indent=2))


if __name__ == "__main__":
    main()
