"""Run maintained vendor contracts without changing vendor manifests or locks.

Cargo workspace tests exclude the vendor crates. This runner builds disposable
fixtures from production pins, checks named contracts actually exist, and runs
each required test exactly (zero tests or ignored tests are failures).
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def source_digest(path):
    checksum = hashlib.sha256()
    for file in sorted((path / "src").rglob("*.rs")):
        checksum.update(file.relative_to(path).as_posix().encode("utf-8"))
        checksum.update(b"\0")
        checksum.update(file.read_bytes())
    return checksum.hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target")
    parser.add_argument("--core-path", type=Path, default=ROOT / "vendor/gpui-pre")
    parser.add_argument("--base-path", type=Path, default=ROOT / "vendor/gpui-base")
    parser.add_argument("--windows-path", type=Path, default=ROOT / "vendor/gpui-pre-windows")
    parser.add_argument("--layer", choices=["all", "native", "base"], default="all")
    parser.add_argument("--requirements", type=Path, default=ROOT / "tools/virtualization-contracts.json")
    args = parser.parse_args()
    requirements = json.loads(args.requirements.read_text(encoding="utf-8"))
    core, base, windows = (p.resolve() for p in (args.core_path, args.base_path, args.windows_path))
    target = args.target_dir.resolve()
    fixture_root = ROOT / "target/virtualization-contracts"
    fixture_root.mkdir(parents=True, exist_ok=True)
    protected = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for source in {core, base, windows}:
        protected.extend(source / name for name in ["Cargo.toml", "Cargo.lock"])
    original = {str(path): digest(path) for path in protected}
    patches = "\n[patch.crates-io]\n" + "\n".join(
        f'{name} = {{ path = {json.dumps(path.as_posix())} }}'
        for name, path in [("gpui-pre", core), ("gpui-base", base), ("gpui-pre-windows", windows)]
    ) + "\n"
    report = {
        "sources": {}, "layers": {},
        "requirements_sha256": digest(args.requirements),
        "native_test_source_sha256": digest(ROOT / "vendor/gpui-pre/tests/virtualization.rs"),
    }
    for name, path in [("core", core), ("base", base), ("windows", windows)]:
        manifest = path / "Cargo.toml"
        package = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]
        report["sources"][name] = {"path": str(path), "version": package["version"], "manifest_sha256": digest(manifest), "src_sha256": source_digest(path)}
    report_path = fixture_root / "last-run.json"
    try:
        for layer in ["native", "base"]:
            if args.layer not in ["all", layer]:
                continue
            contract = requirements[layer]
            if not contract["tests"] or len(set(contract["tests"])) != len(contract["tests"]):
                raise RuntimeError(f"{layer}: required tests must be nonempty and unique")
            fixture = fixture_root / layer
            fixture.mkdir(parents=True, exist_ok=True)
            if layer == "native":
                manifest = f'''[package]
name = "yue-virtual-native-contract"
version = "0.1.0"
edition = "2024"
[workspace]
[[test]]
name = "virtualization"
path = {json.dumps((ROOT / 'vendor/gpui-pre/tests/virtualization.rs').as_posix())}
[dependencies]
gpui = {{ package = "gpui-pre", path = {json.dumps(core.as_posix())}, features = ["test-support"] }}
''' + patches
                selection = ["--test", "virtualization"]
            else:
                manifest = (base / "Cargo.toml").read_text(encoding="utf-8")
                manifest = re.sub(r"(?ms)^\[dev-dependencies\.gpui-fast(?:-platform)?\].*?(?=^\[|\Z)", "", manifest)
                old = 'path = "src/lib.rs"'
                if manifest.count(old) != 1:
                    raise RuntimeError("Base lib path changed; review the fixture adapter before upgrading")
                manifest = manifest.replace(old, f'path = {json.dumps((base / "src/lib.rs").as_posix())}')
                manifest += "\n[workspace]\n" + patches
                selection = ["--lib", "--features", "test-support"]
            (fixture / "Cargo.toml").write_text(manifest, encoding="utf-8")
            # Bootstrap only this disposable lock. Cargo retains production
            # pins and may add Base's test-only dependencies. Source locks stay
            # untouched. Subsequent checks below run the captured executable.
            shutil.copyfile(ROOT / "Cargo.lock", fixture / "Cargo.lock")
            command = ["cargo", "test", "--manifest-path", str(fixture / "Cargo.toml"), "--target-dir", str(target), *selection, "--no-run", "--message-format=json"]
            if args.offline:
                command.append("--offline")
            print(f"Building {layer} contracts", flush=True)
            build = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
            (fixture / "build.log").write_text(build.stdout + build.stderr, encoding="utf-8")
            if build.returncode:
                raise RuntimeError(f"{layer} contract compile failed; see {fixture / 'build.log'}")
            executables = []
            for line in build.stdout.splitlines():
                item = json.loads(line)
                if item.get("reason") == "compiler-artifact" and item.get("executable") and item.get("profile", {}).get("test"):
                    executables.append(item["executable"])
            if len(executables) != 1:
                raise RuntimeError(f"{layer}: expected one test artifact, got {len(executables)}")
            executable = executables[0]
            listing = subprocess.run([executable, "--list"], capture_output=True, text=True, encoding="utf-8", check=True)
            found = set(re.findall(r"(?m)^(.+): test$", listing.stdout))
            missing = set(contract["tests"]) - found
            if missing:
                raise RuntimeError(f"{layer}: required contracts missing: {sorted(missing)}")
            passed = []
            for test in contract["tests"]:
                run = subprocess.run([executable, test, "--exact", "--nocapture", "--test-threads=1"], capture_output=True, text=True, encoding="utf-8", timeout=120)
                (fixture / f"{test.replace('::', '-')}.log").write_text(run.stdout + run.stderr, encoding="utf-8")
                if run.returncode or not re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored;", run.stdout):
                    raise RuntimeError(f"{layer}: {test} failed, skipped, or ran zero tests; see logs in {fixture}")
                passed.append(test)
            report["layers"][layer] = {"passed": passed, "lock_sha256": digest(fixture / "Cargo.lock")}
            print(f"{layer}: {len(passed)} required contracts passed", flush=True)
        report["status"] = "passed"
    except BaseException as error:
        report["status"] = "failed"
        report["error"] = str(error)
        raise
    finally:
        changed = [path for path, before in original.items() if digest(Path(path)) != before]
        if changed:
            report["status"] = "failed"
            report["error"] = f"source manifests/locks changed unexpectedly: {changed}"
        report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        if changed:
            raise RuntimeError(f"source manifests/locks changed unexpectedly: {changed}")
    print(f"Source manifests/locks unchanged. Report: {report_path}", flush=True)


if __name__ == "__main__":
    main()
