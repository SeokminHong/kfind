import contextlib
import hashlib
import importlib.util
import io
import json
import runpy
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
SMART_MEASURE_PATH = BENCHMARK_PATH.with_name("measure-smart.py")


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

    @unittest.skipUnless(SMART_MEASURE_PATH.is_file(), "local Smart measurement tool")
    def test_smart_measurement_requires_runner_input_checksum(self) -> None:
        script = runpy.run_path(str(SMART_MEASURE_PATH))
        source_text = '{"id":"same-id","query":"서다","text":"문 앞에 서 있었다."}\n'
        changed_text = '{"id":"same-id","query":"보다","text":"문 앞에 서 있었다."}\n'
        source_sha = hashlib.sha256(source_text.encode()).hexdigest()
        changed_sha = hashlib.sha256(changed_text.encode()).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory) / "fixture.jsonl"
            fixture.write_text(source_text, encoding="utf-8")
            fixture_arg = str(fixture)
            arm = [fixture_arg, "test", fixture_arg, fixture_arg, fixture_arg]
            for kind, checksum in (
                ("matching", source_sha),
                ("missing", None),
                ("different", changed_sha),
            ):
                with self.subTest(kind=kind):
                    output = Path(directory) / kind
                    payload = {
                        "profile": "full-pos",
                        "boundary": "smart",
                        **{field: source_sha for _, field in script["ARTIFACTS"]},
                        "results": [{"id": "same-id", "spans": [], "latency_ms": 0.1}],
                        "initialization_seconds": 0.01,
                        "evaluation_seconds": 0.001,
                        "peak_rss_kib": 10,
                    }
                    if checksum is not None:
                        payload["fixture_sha256"] = checksum

                    def runner(command, **_):
                        Path(command[-1]).write_text(json.dumps(payload), encoding="utf-8")

                    argv = [
                        str(SMART_MEASURE_PATH), fixture_arg, str(output),
                        "--baseline", *arm, "--candidate", *arm,
                    ]
                    main = script["main"]
                    with (
                        mock.patch.object(sys, "argv", argv),
                        mock.patch.dict(main.__globals__, subprocess=mock.Mock(run=runner)),
                    ):
                        if kind == "matching":
                            self.assertEqual(main(), 0)
                            self.assertTrue((output / "summary.json").is_file())
                        else:
                            with self.assertRaisesRegex(ValueError, "실행 입력 체크섬"):
                                main()
                            self.assertFalse((output / "summary.json").exists())


if __name__ == "__main__":
    unittest.main()
