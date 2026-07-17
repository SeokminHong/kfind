import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


BENCHMARK_PATH = Path(__file__).parents[1] / "benchmark.py"


def load_benchmark():
    if not BENCHMARK_PATH.is_file():
        return None
    sys.path.insert(0, str(BENCHMARK_PATH.parent))
    spec = importlib.util.spec_from_file_location(
        "morph_compare_benchmark", BENCHMARK_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load benchmark module from {BENCHMARK_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


benchmark = load_benchmark()


@unittest.skipIf(benchmark is None, "benchmark.py is not present in this image stage")
class BenchmarkOutputTests(unittest.TestCase):
    def test_progress_and_report_output_are_opt_in(self) -> None:
        quiet = benchmark.parse_args([])
        verbose = benchmark.parse_args(["--progress", "--print-report"])

        self.assertFalse(quiet.progress)
        self.assertFalse(quiet.print_report)
        self.assertTrue(verbose.progress)
        self.assertTrue(verbose.print_report)

    def test_report_markdown_is_quiet_by_default(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report.json"
            stdout = io.StringIO()
            with (
                mock.patch.object(benchmark, "render_markdown", return_value="# report\n"),
                contextlib.redirect_stdout(stdout),
            ):
                result = benchmark.write_report(output, {"result": "ok"})

            self.assertEqual(result, 0)
            self.assertEqual(stdout.getvalue(), "")
            self.assertEqual(output.read_text(encoding="utf-8"), '{\n  "result": "ok"\n}\n')
            self.assertEqual(
                output.with_suffix(".md").read_text(encoding="utf-8"), "# report\n"
            )

    def test_verbose_report_prints_markdown(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "report.json"
            stdout = io.StringIO()
            with (
                mock.patch.object(benchmark, "render_markdown", return_value="# report\n"),
                contextlib.redirect_stdout(stdout),
            ):
                benchmark.write_report(output, {"result": "ok"}, print_report=True)

            self.assertEqual(stdout.getvalue(), "# report\n")

    def test_progress_is_opt_in(self) -> None:
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            benchmark.announce_progress(False, "hidden")
            benchmark.announce_progress(True, "running")

        self.assertEqual(stdout.getvalue(), "[morphology] running\n")


if __name__ == "__main__":
    unittest.main()
