from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from nikl_validate import ValidationError, validate_candidate


HEADER = "lemma\tpos\talternation\tflags\toverrides\tderivations\n"


class ValidateEnrichedTest(unittest.TestCase):
    def candidate(self, predicates: str, *, surface_only_count: int) -> Path:
        directory = Path(self.enterContext(tempfile.TemporaryDirectory()))
        encoded = predicates.encode("utf-8")
        (directory / "predicates.tsv").write_bytes(encoded)
        (directory / "STATS.toml").write_text(
            "schema_version = 5\n"
            f"surface_only_count = {surface_only_count}\n"
            f"artifact_bytes = {len(encoded)}\n",
            encoding="utf-8",
        )
        return directory

    def test_accepts_consistent_candidate(self) -> None:
        directory = self.candidate(
            HEADER + "밀다\tVV\tSurfaceOnly\t\t\tlexical.dictionary-voice=밀리다\n",
            surface_only_count=1,
        )

        result = validate_candidate(directory)

        self.assertEqual(result["surface_only_count"], 1)

    def test_checks_distribution_limit_after_generation(self) -> None:
        directory = self.candidate(
            HEADER + "밀다\tVV\tSurfaceOnly\t\t\tlexical.dictionary-voice=밀리다\n",
            surface_only_count=1,
        )

        with self.assertRaisesRegex(ValidationError, "distribution limit"):
            validate_candidate(directory, max_artifact_bytes=1)

    def test_rejects_stats_mismatch(self) -> None:
        directory = self.candidate(HEADER, surface_only_count=1)

        with self.assertRaisesRegex(ValidationError, "surface_only_count mismatch"):
            validate_candidate(directory)


if __name__ == "__main__":
    unittest.main()
