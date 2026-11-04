"""Prove the upgrade gate rejects missing tests and a real precision regression.

Only a temporary source copy below this repository's target directory is
mutated. Production source/manifests/locks are never edited. Run the ordinary
contract runner afterward to leave its last-run report on production sources.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--target-dir", type=Path, default=ROOT / "target")
    args = parser.parse_args()
    target = ROOT / "target"
    target.mkdir(exist_ok=True)
    logs = target / "virtualization-contracts-negative"
    logs.mkdir(exist_ok=True)
    production = ROOT / "vendor/gpui-pre/src/elements/div.rs"
    original = production.read_bytes()
    command = [sys.executable, str(ROOT / "tools/test-virtualization-contracts.py"), "--layer", "native", "--target-dir", str(args.target_dir)]
    if args.offline:
        command.append("--offline")
    results = {}

    def reject(label, extra, expected):
        result = subprocess.run(command + extra, cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
        (logs / f"{label}.log").write_text(result.stdout + result.stderr, encoding="utf-8")
        if result.returncode == 0 or expected not in result.stderr:
            raise RuntimeError(f"{label}: expected rejection was not observed; inspect {logs}")
        results[label] = json.loads((target / "virtualization-contracts/last-run.json").read_text())
        print(f"{label}: expected rejection verified", flush=True)

    try:
        # The generated directory is below the explicit workspace target;
        # TemporaryDirectory removes only this checked source-copy location.
        with tempfile.TemporaryDirectory(prefix="virtual-contract-negative-", dir=target) as temporary:
            scratch = Path(temporary).resolve()
            if not scratch.is_relative_to(target.resolve()):
                raise RuntimeError("negative fixture escaped the workspace target")
            requirements = json.loads((ROOT / "tools/virtualization-contracts.json").read_text())
            requirements["native"]["tests"].append("intentionally_missing_virtualization_contract")
            missing = scratch / "missing.json"
            missing.write_text(json.dumps(requirements), encoding="utf-8")
            reject("missing-contract", ["--requirements", str(missing)], "required contracts missing")
            core = scratch / "core"
            shutil.copytree(ROOT / "vendor/gpui-pre", core, ignore=shutil.ignore_patterns("target", ".git", "Cargo.lock"))
            source = core / "src/elements/div.rs"
            content = source.read_text(encoding="utf-8")
            old = """    pub fn precise_offset(&self) -> Point<f64> {
        self.0.borrow().offset.borrow_mut().precise()
    }"""
            new = """    pub fn precise_offset(&self) -> Point<f64> {
        let offset = self.offset();
        point(f64::from(offset.x), f64::from(offset.y))
    }"""
            if content.count(old) != 1:
                raise RuntimeError("precision mutation target changed; review the negative-control adapter")
            source.write_text(content.replace(old, new), encoding="utf-8")
            reject("precision-regression", ["--core-path", str(core)], "scroll_handle_preserves_exact_positions_and_explicit_jumps_reset_fractional_input failed")
            shutil.copyfile(target / "virtualization-contracts/native/scroll_handle_preserves_exact_positions_and_explicit_jumps_reset_fractional_input.log", logs / "precision-assertion.log")
    finally:
        if production.read_bytes() != original:
            raise RuntimeError("production source changed during the negative control")
    (logs / "last-run.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
