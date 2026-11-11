"""Exercise the Windows process gate using fixed process inventories."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
SPEC = importlib.util.spec_from_file_location("sampling", ROOT / "tools/sample-table.py")
SAMPLING = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SAMPLING)


@unittest.skipUnless(os.name == "nt", "Windows process inventory query")
class SamplingGate(unittest.TestCase):
    def query(self, processes):
        with patch.object(SAMPLING.subprocess, "run") as call:
            call.return_value.stdout = "0"
            SAMPLING.busy(ignore_foreign_app_wrappers=True)
            query = call.call_args.args[0][-1]
        inventory = json.dumps(processes).replace("'", "''")
        query = "$fixture = '" + inventory + "' | ConvertFrom-Json\n" + query.replace("Get-CimInstance Win32_Process", "$fixture")
        result = subprocess.run(["powershell", "-NoProfile", "-Command", query], check=True, capture_output=True, text=True)
        return int(result.stdout.strip())

    def test_only_a_launched_foreign_app_wrapper_can_be_ignored(self):
        cargo = dict(ProcessId=10, ParentProcessId=1, Name="cargo.exe", CommandLine='"C:\\rust\\cargo.exe" run --release')
        app = dict(ProcessId=11, ParentProcessId=10, Name="demo.exe", ExecutablePath="C:\\foreign\\target\\release\\demo.exe")
        self.assertEqual(self.query([cargo, app]), 0)
        self.assertEqual(self.query([cargo]), 1)
        self.assertEqual(self.query([dict(cargo, CommandLine="cargo test --workspace"), app]), 1)
        self.assertEqual(self.query([dict(cargo, CommandLine="cargo build --release"), app]), 1)
        self.assertEqual(self.query([cargo, dict(app, ExecutablePath=None)]), 1)
        self.assertEqual(self.query([cargo, dict(app, ExecutablePath="C:\\foreign\\target\\release\\build\\demo\\build-script-build.exe")]), 1)
        self.assertEqual(self.query([cargo, dict(app, ExecutablePath=str(ROOT / "target/release/demo.exe"))]), 1)

    def test_compilers_and_showcase_always_block_sampling(self):
        cargo = dict(ProcessId=10, ParentProcessId=1, Name="cargo.exe", CommandLine="cargo run --release")
        app = dict(ProcessId=11, ParentProcessId=10, Name="demo.exe", ExecutablePath="C:\\foreign\\target\\release\\demo.exe")
        for name in ["rustc.exe", "clippy-driver.exe", "windows-showcase.exe"]:
            with self.subTest(name=name):
                self.assertEqual(self.query([cargo, app, dict(ProcessId=12, Name=name)]), 1)


if __name__ == "__main__":
    unittest.main()
