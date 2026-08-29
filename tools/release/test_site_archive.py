from __future__ import annotations

import importlib.util
import json
import tarfile
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("site_archive.py")
SPEC = importlib.util.spec_from_file_location("kfind_site_archive", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("site archive module could not be loaded")
site_archive = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(site_archive)


class SiteArchiveTest(unittest.TestCase):
    def test_archive_index_points_to_file_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            site = root / "site"
            (site / "assets").mkdir(parents=True)
            (site / "index.html").write_text("<h1>kfind</h1>\n", encoding="utf-8")
            (site / "assets/app.js").write_text("export {};\n", encoding="utf-8")
            component = root / "component.kfc"
            component.write_bytes(b"KFC fixture")
            archive = root / "site.tar"
            compressed = root / "site.tar.gz"
            index = root / "index.json"
            second_archive = root / "site-second.tar"
            second_compressed = root / "site-second.tar.gz"
            second_index = root / "index-second.json"

            site_archive.build_archive(
                "1.0.0-rc.4",
                site,
                component,
                archive,
                compressed,
                index,
            )
            site_archive.build_archive(
                "1.0.0-rc.4",
                site,
                component,
                second_archive,
                second_compressed,
                second_index,
            )

            self.assertEqual(archive.read_bytes(), second_archive.read_bytes())
            self.assertEqual(
                compressed.read_bytes(), second_compressed.read_bytes()
            )
            self.assertEqual(index.read_bytes(), second_index.read_bytes())

            index_value = json.loads(index.read_text(encoding="utf-8"))
            with archive.open("rb") as source:
                for path, metadata in index_value["files"].items():
                    source.seek(metadata["offset"])
                    indexed_bytes = source.read(metadata["length"])
                    expected = (
                        component.read_bytes()
                        if path == "api/component-resource"
                        else (site / path).read_bytes()
                    )
                    self.assertEqual(indexed_bytes, expected)

            with tarfile.open(compressed, "r:gz") as source:
                self.assertEqual(
                    source.extractfile("index.html").read(),
                    b"<h1>kfind</h1>\n",
                )

    def test_manifest_is_sorted_by_semver(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            existing = root / "existing.json"
            existing.write_text(
                json.dumps(
                    {
                        "latest": "2.0.0-rc.1",
                        "schemaVersion": 1,
                        "versions": [
                            {
                                "path": "/versions/1.0.0",
                                "prerelease": False,
                                "version": "1.0.0",
                            },
                            {
                                "path": "/versions/2.0.0-rc.1",
                                "prerelease": True,
                                "version": "2.0.0-rc.1",
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )
            output = root / "manifest.json"

            site_archive.update_manifest("1.1.0", existing, output)

            manifest = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(manifest["latest"], "2.0.0-rc.1")
            self.assertEqual(
                [entry["version"] for entry in manifest["versions"]],
                ["2.0.0-rc.1", "1.1.0", "1.0.0"],
            )

    def test_manifest_rejects_an_incorrect_existing_latest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            existing = root / "existing.json"
            existing.write_text(
                json.dumps(
                    {
                        "latest": "1.0.0",
                        "schemaVersion": 1,
                        "versions": [
                            {
                                "path": "/versions/2.0.0-rc.1",
                                "prerelease": True,
                                "version": "2.0.0-rc.1",
                            },
                            {
                                "path": "/versions/1.0.0",
                                "prerelease": False,
                                "version": "1.0.0",
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(RuntimeError, "latest is invalid"):
                site_archive.update_manifest(
                    "1.1.0",
                    existing,
                    root / "manifest.json",
                )


if __name__ == "__main__":
    unittest.main()
