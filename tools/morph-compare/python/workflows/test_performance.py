import tempfile
import unittest
from pathlib import Path

from workflows.performance import (
    FIXTURE_LINE,
    STANDARD_CORPUS,
    parse_peak_rss_kib,
    summarize_runs,
    workflow_commands,
    write_fixture,
)


class CorpusSettingsTests(unittest.TestCase):
    def test_standard_corpus_preserves_the_product_contract(self) -> None:
        self.assertEqual(100 * 1024 * 1024, STANDARD_CORPUS.total_bytes)
        self.assertEqual(1_000, STANDARD_CORPUS.file_count)
        self.assertEqual(999, STANDARD_CORPUS.generated_files)
        self.assertEqual(976, STANDARD_CORPUS.generated_small_files + 1)

    def test_fixture_has_one_match_line_and_exact_size(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.txt"
            write_fixture(path, 4096)

            contents = path.read_bytes()

        self.assertEqual(4096, len(contents))
        self.assertTrue(contents.startswith(FIXTURE_LINE))
        self.assertEqual(1, contents.count("학교".encode()))


class WorkflowCommandTests(unittest.TestCase):
    def test_commands_match_agent_and_human_contracts(self) -> None:
        commands = workflow_commands(
            Path("/bin/kfind"), Path("/data"), Path("/corpus")
        )

        self.assertEqual(
            [
                "/bin/kfind",
                "--embedded",
                "--boundary",
                "any",
                "--pos",
                "noun",
                "--json",
                "학교",
                "/corpus",
            ],
            commands["agent"],
        )
        self.assertEqual(
            ["/bin/kfind", "--data-dir", "/data", "학교", "/corpus"],
            commands["human"],
        )


class PerformanceSummaryTests(unittest.TestCase):
    def test_parses_linux_peak_rss(self) -> None:
        self.assertEqual(
            12345,
            parse_peak_rss_kib("Name:\tkfind\nVmHWM:\t   12345 kB\n"),
        )

    def test_uses_median_and_keeps_observed_range(self) -> None:
        summary = summarize_runs(
            [
                {
                    "wall_seconds": 0.1,
                    "throughput_mib_s": 1000.0,
                    "peak_rss_kib": 100,
                },
                {
                    "wall_seconds": 0.3,
                    "throughput_mib_s": 300.0,
                    "peak_rss_kib": 300,
                },
                {
                    "wall_seconds": 0.2,
                    "throughput_mib_s": 500.0,
                    "peak_rss_kib": 200,
                },
            ]
        )

        self.assertEqual(3, summary["runs"])
        self.assertEqual(0.2, summary["wall_seconds"])
        self.assertEqual(500.0, summary["throughput_mib_s"])
        self.assertEqual(200, summary["peak_rss_kib"])
        self.assertEqual(0.1, summary["run_min"]["wall_seconds"])
        self.assertEqual(0.3, summary["run_max"]["wall_seconds"])


if __name__ == "__main__":
    unittest.main()
