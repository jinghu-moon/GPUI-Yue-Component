"""Compare the current virtualization source with one committed baseline.

Keeps isolated manifests/artifacts outside the repository, never checks out or
rewrites the production worktree. Runs the same source against both versions.
"""
import argparse
import csv
import io
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CACHE = Path(os.environ.get("YUE_VIRTUAL_BENCH", "C:/Temp/gpui-yue-series-20261104"))
TARGET = Path("C:/Temp/gpui-yue-owner-perf-20261104")


def run(args, **kwargs):
    try:
        return subprocess.run(args, cwd=ROOT, check=True, **kwargs)
    except subprocess.CalledProcessError as error:
        if error.stderr:
            print(error.stderr)
        raise


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--before", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--filter", default="series_")
    parser.add_argument("--workload", default="docs/research/virtual-list-optimization-series-2026-11-04.rs")
    parser.add_argument("--after-feature", choices=["masonry-views"], default=None)
    args = parser.parse_args()
    if not args.label.replace("-", "").isalnum():
        parser.error("label must contain only letters, digits or hyphens")
    reference = run(["git", "rev-parse", args.before], capture_output=True, text=True).stdout.strip()
    archive = run(["git", "archive", "--format=zip", reference,
                   "vendor/gpui-base", "vendor/gpui-pre"], capture_output=True).stdout
    baseline = CACHE / "baselines" / reference
    for entry in zipfile.ZipFile(io.BytesIO(archive)).infolist():
        if entry.is_dir():
            continue
        destination = baseline / entry.filename
        if not destination.resolve().is_relative_to(baseline.resolve()):
            raise ValueError("unexpected archive entry")
        data = zipfile.ZipFile(io.BytesIO(archive)).read(entry)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if not destination.exists() or destination.read_bytes() != data:
            destination.write_bytes(data)
    logs = CACHE / args.label
    logs.mkdir(parents=True, exist_ok=True)
    executables = {}
    for version in ["before", "after"]:
        fixture = CACHE / version
        fixture.mkdir(parents=True, exist_ok=True)
        vendor = baseline / "vendor" if version == "before" else ROOT / "vendor"
        manifest = f"""[package]
name = "yue-virtual-series-{version}"
version = "0.1.0"
edition = "2024"
[workspace]
[[test]]
name = "perf"
path = "{(ROOT / 'docs/research/virtual-list-optimization-series-2026-11-04.rs').as_posix()}"
[dependencies]
gpui-kit = {{ version = "0.8.0", default-features = false }}
gpui-base = {{ version = "0.8.0", features = ["test-support"] }}
[patch.crates-io]
gpui-base = {{ path = "{(vendor / 'gpui-base').as_posix()}" }}
gpui-pre = {{ path = "{(vendor / 'gpui-pre').as_posix()}" }}
gpui-pre-windows = {{ path = "{(ROOT / 'vendor/gpui-pre-windows').as_posix()}" }}
[features]
masonry-views = []
"""
        manifest = manifest.replace(
            (ROOT / 'docs/research/virtual-list-optimization-series-2026-11-04.rs').as_posix(),
            (ROOT / args.workload).resolve().as_posix(),
        )
        (fixture / "Cargo.toml").write_text(manifest, encoding="utf-8")
        if not (fixture / "Cargo.lock").exists():
            (fixture / "Cargo.lock").write_bytes((ROOT / "Cargo.lock").read_bytes())
        command = ["cargo", "test", "--manifest-path", str(fixture / "Cargo.toml"),
                   "--target-dir", str(TARGET), "--offline", "--release", "--test", "perf",
                   "--no-run", "--message-format=json"]
        if version == "after" and args.after_feature:
            command += ["--features", args.after_feature]
        print(f"Building {args.label} {version}", flush=True)
        built = run(command, capture_output=True, text=True, encoding="utf-8")
        (logs / f"{version}-build.txt").write_text(built.stderr, encoding="utf-8")
        for line in built.stdout.splitlines():
            artifact = json.loads(line)
            if artifact.get("reason") == "compiler-artifact" and artifact.get("executable"):
                executables[version] = artifact["executable"]
    # cargo dev may be a persistent, idle file watcher. It never exits, so do
    # not treat that command alone as compilation. Its cargo/rustc children
    # still block sampling, as do all ordinary Cargo commands.
    compiler_query = """@(Get-CimInstance Win32_Process | Where-Object {
        $_.Name -in @('rustc.exe', 'clippy-driver.exe') -or
        ($_.Name -eq 'cargo.exe' -and $_.CommandLine -notmatch '(?i)\\bcargo(?:\\.exe)?["\\x27]?\\s+dev(?:\\s|$)')
    }).Count"""
    # Require two quiet observations before sampling.
    quiet = 0
    while True:
        busy = run(["powershell", "-NoProfile", "-Command",
                    compiler_query],
                   capture_output=True, text=True).stdout.strip()
        if busy == "0":
            quiet += 1
            if quiet >= 2:
                break
        else:
            quiet = 0
        print("Waiting for compiler processes before sampling", flush=True)
        time.sleep(2)
    rows = []
    for version, round_ in [("before", 0), ("after", 0), ("after", 1), ("before", 1)]:
        result = run([executables[version], args.filter, "--nocapture", "--test-threads=1"],
                     capture_output=True, text=True, encoding="utf-8")
        (logs / f"{version}-{round_}.txt").write_text(result.stdout + result.stderr, encoding="utf-8")
        for line in result.stdout.splitlines():
            if "SERIES," in line:
                values = dict(part.split("=", 1) for part in line.split("SERIES,", 1)[1].split(","))
                rows.append({"version": version, "round": round_, **values})
        print(f"Completed {args.label} {version}/{round_}", flush=True)
    if not rows:
        raise ValueError("no measurement rows")
    destination = ROOT / f"docs/research/virtual-list-series-{args.label}-2026-11-04.csv"
    with destination.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=rows[0].keys())
        writer.writeheader()
        writer.writerows(rows)
    (logs / "identity.json").write_text(json.dumps({"before": reference, "filter": args.filter,
        "after_head": run(["git", "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip(),
        "workload": args.workload,
        "workload_sha256": hashlib.sha256((ROOT / args.workload).read_bytes()).hexdigest(),
        "after_feature": args.after_feature,
        "compiler_wait_policy": "exclude idle cargo dev watcher; require two quiet compiler observations",
        "order": ["before/0", "after/0", "after/1", "before/1"], "rows": len(rows)}, indent=2) + "\n", encoding="utf-8")
    print(destination)


if __name__ == "__main__":
    main()
