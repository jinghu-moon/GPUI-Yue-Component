"""Positive and deliberately broken evidence; no Cargo build or UI required."""
import copy
import unittest
from virtualization_evidence import check_native_output, check_span_samples


def fixture():
    rows = []
    for version, round_ in [("before", 0), ("after", 0), ("after", 1), ("before", 1)]:
        for stage in ["span", "cold"]:
            for same_row in ["false", "true"]:
                for count in [0, 100, 1000, 10000]:
                    for sample in range(5):
                        rows.append(dict(version=version, round=str(round_), case=f"{stage}-{same_row}-{count}", sample=str(sample),
                                         ns="1000", p50="10", p95="20", p99="30", config_ns="1000" if stage == "cold" else "200",
                                         render_ns="0" if stage == "cold" else "800", builds="0" if stage == "cold" else "1600"))
    build = {field: "a" * 64 for field in ["manifest_sha256", "lock_sha256", "dependency_records_sha256", "vendor_sources_sha256"]}
    identity = dict(before="b" * 40, workload_sha256="c" * 64, order=["before/0", "after/0", "after/1", "before/1"], rows=len(rows), build_identity=dict(before=build, after=copy.deepcopy(build)))
    return rows, identity


class EvidenceTests(unittest.TestCase):
    def test_balanced_and_coarse_budget(self):
        rows, identity = fixture()
        self.assertEqual(len(check_span_samples(rows, identity, 35)), 16)

    def test_missing_duplicate_wrong_case_nan_and_unbalanced_are_rejected(self):
        for defect in ["missing", "duplicate", "case", "nan", "round", "workset", "quantile", "total", "identity", "dependency"]:
            with self.subTest(defect=defect):
                rows, identity = fixture()
                if defect == "missing": rows.pop()
                elif defect == "duplicate": rows[-1] = rows[0].copy()
                elif defect == "case": rows[0]["case"] = "span-unknown"
                elif defect == "nan": rows[0]["ns"] = "NaN"
                elif defect == "round": rows[0]["round"] = "2"
                elif defect == "workset": rows[0]["builds"] = "0"
                elif defect == "quantile": rows[0]["p95"] = "1"
                elif defect == "total": rows[0]["render_ns"] = "1"
                elif defect == "identity": identity["rows"] = 0
                elif defect == "dependency": identity["build_identity"]["after"]["dependency_records_sha256"] = "d" * 64
                with self.assertRaises(ValueError): check_span_samples(rows, identity)

    def test_regression_including_cold_is_rejected(self):
        for case in ["span-false-10000", "cold-true-10000"]:
            rows, identity = fixture()
            for row in rows:
                if row["version"] == "after" and row["case"] == case:
                    row["ns"] = "100000"
                    row["config_ns"] = str(100000 - int(row["render_ns"]))
            with self.assertRaisesRegex(ValueError, "regression alarm"): check_span_samples(rows, identity, 35)

    def test_tiny_absolute_changes_do_not_trigger_shared_runner_alarm(self):
        rows, identity = fixture()
        for row in rows:
            if row["version"] == "after":
                row["ns"] = "2000"
                row["config_ns"] = str(2000 - int(row["render_ns"]))
        check_span_samples(rows, identity, 35)

    def test_missing_reuse_benefit_is_rejected_even_without_total_regression(self):
        rows, identity = fixture()
        identity["after_feature"] = "span-cache"
        with self.assertRaisesRegex(ValueError, "reuse disappeared"):
            check_span_samples(rows, identity, 35)
        for row in rows:
            if row["version"] == "after" and row["case"].startswith("span-"):
                row["config_ns"], row["render_ns"] = "50", "950"
        check_span_samples(rows, identity, 35)

    def test_unrecorded_process_order_is_rejected(self):
        rows, identity = fixture()
        rows[0], rows[80] = rows[80], rows[0]
        with self.assertRaisesRegex(ValueError, "ABBA order"):
            check_span_samples(rows, identity)

    def test_native_exactly_one_executed(self):
        output = "running 1 test\ntest native::probe ... FRAME,case=grid\nok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out\n"
        check_native_output(output, "native::probe")
        for bad in [output.replace("1 passed", "0 passed"), output.replace("0 ignored", "1 ignored"), output.replace("native::probe", "native::other"), output + output]:
            with self.assertRaises(ValueError): check_native_output(bad, "native::probe")


if __name__ == "__main__":
    unittest.main(verbosity=2)
