from __future__ import annotations

import unittest
from pathlib import Path

from readme_guard import check_readme, check_repository


REPOSITORY = Path(__file__).resolve().parents[2]


class ReadmeGuardTest(unittest.TestCase):
    def test_repository_readmes_contain_no_work_history(self) -> None:
        self.assertEqual(check_repository(REPOSITORY), [])

    def test_rejects_dated_benchmark_result(self) -> None:
        text = (
            "# Benchmarks\n\n"
            "2026-07-15 고정 50 Hz PTY benchmark에서 300회 이동은 "
            "scroll frame 121회로 59.7% 줄었습니다.\n"
        )

        violations = check_readme(Path("docs/benchmarks/README.md"), text)

        self.assertEqual(
            {violation.reason for violation in violations},
            {"dated history", "work-log wording"},
        )

    def test_rejects_root_benchmark_snapshot_table(self) -> None:
        text = "# kfind\n\n## Benchmarks\n\n| Result | Value |\n| --- | --- |\n"

        violations = check_readme(Path("README.md"), text)

        self.assertEqual(
            [violation.reason for violation in violations],
            ["benchmark result table"],
        )

    def test_allows_fixture_provenance_revision(self) -> None:
        text = "- Source revision `349481e` is checksum-pinned.\n"

        self.assertEqual(check_readme(Path("data/fixtures/README.md"), text), [])


if __name__ == "__main__":
    unittest.main()
