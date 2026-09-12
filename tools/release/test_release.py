from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import call, patch


MODULE_PATH = Path(__file__).with_name("release.py")
SPEC = importlib.util.spec_from_file_location("kfind_release", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("release module could not be loaded")
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


class ReleaseVersionTest(unittest.TestCase):
    @patch.object(release, "refresh_lockfiles")
    def test_set_version_preserves_dependency_requirements(self, refresh) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifests = {
                "Cargo.toml": "workspace.package",
                "tools/morph-compare/runner/Cargo.toml": "package",
            }
            for name, section in manifests.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                dependencies = (
                    "workspace.dependencies"
                    if section == "workspace.package"
                    else "dependencies"
                )
                path.write_text(
                    f'[{section}]\nversion = "1.0.1"\n'
                    f'\n[{dependencies}]\nanyhow = "1.0.102"\n'
                    'same = "1.0.1"\ninline = { version = "1.0.1" }\n',
                    encoding="utf-8",
                )
            for relative_path in release.CURRENT_VERSION_FILES:
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("kfind 1.0.1\n", encoding="utf-8")

            release.set_version("1.1.0", root)

            for name, section in manifests.items():
                manifest = tomllib.loads((root / name).read_text(encoding="utf-8"))
                package = manifest
                for key in section.split("."):
                    package = package[key]
                self.assertEqual(package["version"], "1.1.0")
                self.assertEqual(
                    manifest.get("workspace", manifest)["dependencies"],
                    {
                        "anyhow": "1.0.102",
                        "same": "1.0.1",
                        "inline": {"version": "1.0.1"},
                    },
                )
            refresh.assert_called_once_with(root)

    def test_missing_package_version_does_not_replace_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "Cargo.toml"
            source = '[package]\nname = "test"\n\n[dependencies]\nversion = "1.0.1"\n'
            path.write_text(source, encoding="utf-8")
            with self.assertRaises(RuntimeError):
                release.replace_manifest_version(path, "package", "1.0.1", "1.1.0")
            self.assertEqual(path.read_text(encoding="utf-8"), source)

    def test_stable_bumps_use_latest_stable_tag(self) -> None:
        tags = ["v0.2.1", "v1.0.0-rc.3", "v0.3.0-rc.1"]

        self.assertEqual(release.next_version(tags, "major", False), "1.0.0")
        self.assertEqual(release.next_version(tags, "minor", False), "0.3.0")
        self.assertEqual(release.next_version(tags, "patch", False), "0.2.2")

    def test_prerelease_continues_target_release_candidate(self) -> None:
        tags = ["v0.2.1", "v1.0.0-rc.1", "v1.0.0-rc.3", "v1.0.0-beta.1"]

        self.assertEqual(release.next_version(tags, "major", True), "1.0.0-rc.4")

    def test_prerelease_starts_at_first_release_candidate(self) -> None:
        self.assertEqual(
            release.next_version(["v1.2.3"], "minor", True),
            "1.3.0-rc.1",
        )

    def test_invalid_version_is_rejected(self) -> None:
        for version in ("v1.0.0", "1.0", "1.0.0-rc.0", "01.0.0"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                release.parse_version(version)

    def test_chocolatey_version_preserves_stable_release(self) -> None:
        self.assertEqual(release.chocolatey_version("1.2.3"), "1.2.3")

    def test_chocolatey_version_maps_release_candidate(self) -> None:
        self.assertEqual(
            release.chocolatey_version("1.2.3-rc.4"),
            "1.2.3-rc0004",
        )
        self.assertEqual(
            release.chocolatey_version("1.2.3-rc.10000"),
            "1.2.3-rc10000",
        )

    @patch.object(release.subprocess, "run")
    def test_lockfiles_update_only_workspace_packages(self, run) -> None:
        repository_root = Path("/repository")

        release.refresh_lockfiles(repository_root)

        self.assertEqual(
            run.call_args_list,
            [
                call(
                    [
                        "cargo",
                        "update",
                        "--workspace",
                        "--quiet",
                        "--manifest-path",
                        str(repository_root / manifest),
                    ],
                    cwd=repository_root,
                    check=True,
                    stdout=subprocess.DEVNULL,
                )
                for manifest in release.LOCKFILE_MANIFESTS
            ],
        )


if __name__ == "__main__":
    unittest.main()
