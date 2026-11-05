"""Inventory actual COM capabilities of the production-pinned Windows bridge.

This is deliberately not a claim of complete Grid/screen-reader support. Source
is copied from Cargo metadata to a disposable fixture; registry and production
manifests/locks are never edited. Changed capabilities require explicit review.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TESTS = ["grid_roles_names_and_full_description_reach_com",
         "pinned_bridge_inventory_keeps_missing_grid_and_realization_explicit",
         "cell_invoke_and_focus_forward_real_action_requests"]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def identities(path):
    return {(p["name"], p["version"], p.get("source"), p.get("checksum"))
            for p in tomllib.loads(path.read_text(encoding="utf-8"))["package"]
            if p.get("source")}


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    if sys.platform != "win32":
        raise RuntimeError("Windows COM contracts require Windows; no skip-as-pass")
    protected = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    before = {p: digest(p) for p in protected}
    common = ["--offline"] if args.offline else []
    metadata = subprocess.run(["cargo", "metadata", "--locked", "--format-version=1",
                               "--filter-platform", "x86_64-pc-windows-msvc", *common],
                              cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=True)
    package, = [p for p in json.loads(metadata.stdout)["packages"] if p["name"] == "accesskit_windows"]
    if package["version"] != "0.34.0":
        raise RuntimeError("Pinned bridge changed; review expected capabilities first")
    source = Path(package["manifest_path"]).parent
    source_hashes = {p: digest(p) for p in [source / "Cargo.toml", *sorted((source / "src").rglob("*.rs"))]}
    base = ROOT / "target/windows-grid-bridge"
    base.mkdir(parents=True, exist_ok=True)
    fixture = Path(tempfile.mkdtemp(prefix="probe-", dir=base))
    report = {"bridge_version": package["version"], "source": str(source),
              "contract_sha256": digest(ROOT / "tools/contracts/windows_grid_bridge.rs"),
              "source_sha256": {str(p.relative_to(source)): v for p, v in source_hashes.items()},
              "passed": [], "status": "running"}
    try:
        shutil.copytree(source / "src", fixture / "src")
        shutil.copyfile(ROOT / "tools/contracts/windows_grid_bridge.rs", fixture / "src/yue_grid_contracts.rs")
        lib = fixture / "src/lib.rs"
        text = lib.read_text(encoding="utf-8")
        old = "#[cfg(test)]\nmod tests;"
        if text.count(old) != 1:
            raise RuntimeError("Bridge test entry changed; review fixture adapter")
        lib.write_text(text.replace(old, "#[cfg(test)]\nmod yue_grid_contracts;"), encoding="utf-8")
        manifest = (source / "Cargo.toml").read_text(encoding="utf-8")
        manifest = re.sub(r"(?ms)^\[dev-dependencies\.[^]]+\].*?(?=^\[|\Z)", "", manifest)
        (fixture / "Cargo.toml").write_text(manifest + "\n[workspace]\n", encoding="utf-8")
        shutil.copyfile(ROOT / "Cargo.lock", fixture / "Cargo.lock")
        build = subprocess.run(["cargo", "test", "--manifest-path", str(fixture / "Cargo.toml"),
                                "--target-dir", str(base / "build"), "--lib", "--no-run",
                                "--message-format=json", *common],
                               cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
        (fixture / "build.log").write_text(build.stdout + build.stderr, encoding="utf-8")
        if build.returncode:
            raise RuntimeError(f"Bridge fixture compile failed: {fixture / 'build.log'}")
        unexpected = identities(fixture / "Cargo.lock") - identities(ROOT / "Cargo.lock")
        if unexpected:
            raise RuntimeError(f"Fixture resolved unpinned production dependencies: {unexpected}")
        report["lock_sha256"] = digest(fixture / "Cargo.lock")
        executables = [p["executable"] for line in build.stdout.splitlines()
                       if (p := json.loads(line)).get("executable") and p.get("profile", {}).get("test")]
        executable, = executables
        report["executable_sha256"] = digest(Path(executable))
        for name in TESTS:
            name = "yue_grid_contracts::" + name
            run = subprocess.run([executable, name, "--exact", "--nocapture", "--test-threads=1"],
                                 cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=60)
            (fixture / f"{name.split('::')[-1]}.log").write_text(run.stdout + run.stderr, encoding="utf-8")
            if run.returncode or not re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", run.stdout):
                raise RuntimeError(f"Bridge contract failed/skipped/zero tests: {name}; see {fixture}")
            report["passed"].append(name)
        report["status"] = "passed"
    except BaseException as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        changed = [str(p) for p, value in {**before, **source_hashes}.items() if digest(p) != value]
        if changed:
            report.update(status="failed", error=f"Protected sources changed: {changed}")
        (fixture / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"Bridge report: {fixture / 'report.json'}", flush=True)
        if changed:
            raise RuntimeError(report["error"])
    print(f"{len(report['passed'])} direct COM contracts passed; Grid/GridItem/Realize remain unsupported.")


if __name__ == "__main__":
    main()
