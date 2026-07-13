from __future__ import annotations

import hashlib
import os
import statistics
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path


MIB = 1024 * 1024
QUERY = "학교"
FIXTURE_LINE = "학교에서 새 문서를 검토했다.\n".encode()
FIXTURE_NAME = "fixture-00000.txt"
PADDING = b"// kfind product workflow padding\n"
DEFAULT_KFIND = Path("/usr/local/bin/kfind")
DEFAULT_GENERATOR = Path("/usr/local/bin/generate-corpus")
DEFAULT_DATA_DIR = Path("/opt/morph-benchmark/product-data")


@dataclass(frozen=True)
class CorpusSettings:
    total_bytes: int
    file_count: int
    fixture_bytes: int
    generated_small_files: int
    korean_percent: int = 5
    nfd_percent: int = 50
    seed: int = 0x004B_4649_4E44

    @property
    def generated_bytes(self) -> int:
        return self.total_bytes - self.fixture_bytes

    @property
    def generated_files(self) -> int:
        return self.file_count - 1


STANDARD_CORPUS = CorpusSettings(
    total_bytes=100 * MIB,
    file_count=1_000,
    fixture_bytes=64 * 1024,
    generated_small_files=975,
)
SMOKE_CORPUS = CorpusSettings(
    total_bytes=MIB,
    file_count=10,
    fixture_bytes=64 * 1024,
    generated_small_files=5,
)


def measure_product_workflows(
    runs: int,
    smoke: bool,
    *,
    kfind: Path = DEFAULT_KFIND,
    generator: Path = DEFAULT_GENERATOR,
    data_dir: Path = DEFAULT_DATA_DIR,
) -> dict[str, object]:
    if runs < 1:
        raise ValueError("product workflow runs must be at least 1")
    for executable in (kfind, generator):
        if not executable.is_file() or not os.access(executable, os.X_OK):
            raise ValueError(f"required executable is unavailable: {executable}")

    settings = SMOKE_CORPUS if smoke else STANDARD_CORPUS
    with tempfile.TemporaryDirectory(prefix="kfind-product-workflow-") as directory:
        corpus = Path(directory) / "corpus"
        build_corpus(generator, corpus, settings)
        commands = workflow_commands(kfind, data_dir, corpus)
        for command in commands.values():
            verify_single_result(command)

        measured: dict[str, list[dict[str, float | int]]] = {
            name: [] for name in commands
        }
        for run_index in range(runs):
            order = tuple(commands) if run_index % 2 == 0 else tuple(reversed(commands))
            for name in order:
                measured[name].append(measure_command(commands[name], settings.total_bytes))

        return {
            "profile": "smoke" if smoke else "standard",
            "cache": "one discarded warm-up, then warm-cache fresh processes",
            "query": QUERY,
            "corpus": corpus_metadata(corpus, settings),
            "workflows": {
                "agent": {
                    "input": "explicit POS",
                    "lexicon": "embedded",
                    "boundary": "any",
                    "output": "JSON Lines",
                    "command": display_command(commands["agent"], corpus, data_dir),
                    "matching_lines": 1,
                    "performance": summarize_runs(measured["agent"]),
                },
                "human": {
                    "input": "untagged",
                    "lexicon": "full-pos",
                    "boundary": "smart",
                    "output": "default text",
                    "command": display_command(commands["human"], corpus, data_dir),
                    "matching_lines": 1,
                    "performance": summarize_runs(measured["human"]),
                },
            },
        }


def build_corpus(
    generator: Path, corpus: Path, settings: CorpusSettings
) -> None:
    result = subprocess.run(
        [
            str(generator),
            str(corpus),
            "--total-bytes",
            str(settings.generated_bytes),
            "--files",
            str(settings.generated_files),
            "--small-files",
            str(settings.generated_small_files),
            "--small-file-bytes",
            str(settings.fixture_bytes),
            "--korean-percent",
            str(settings.korean_percent),
            "--nfd-percent",
            str(settings.nfd_percent),
            "--seed",
            str(settings.seed),
        ],
        text=True,
        capture_output=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"product corpus generator failed: {result.stderr.strip()}")
    write_fixture(corpus / FIXTURE_NAME, settings.fixture_bytes)
    files = sorted(path for path in corpus.iterdir() if path.is_file())
    actual_bytes = sum(path.stat().st_size for path in files)
    if len(files) != settings.file_count or actual_bytes != settings.total_bytes:
        raise ValueError(
            "product corpus shape mismatch: "
            f"expected {settings.file_count} files/{settings.total_bytes} bytes, "
            f"got {len(files)} files/{actual_bytes} bytes"
        )


def write_fixture(path: Path, size: int) -> None:
    if size < len(FIXTURE_LINE):
        raise ValueError("fixture size is smaller than its match line")
    with path.open("wb") as output:
        output.write(FIXTURE_LINE)
        remaining = size - len(FIXTURE_LINE)
        while remaining:
            chunk = PADDING[:remaining]
            output.write(chunk)
            remaining -= len(chunk)


def workflow_commands(
    kfind: Path, data_dir: Path, corpus: Path
) -> dict[str, list[str]]:
    return {
        "agent": [
            str(kfind),
            "--embedded",
            "--boundary",
            "any",
            "--pos",
            "noun",
            "--json",
            QUERY,
            str(corpus),
        ],
        "human": [
            str(kfind),
            "--data-dir",
            str(data_dir),
            QUERY,
            str(corpus),
        ],
    }


def verify_single_result(command: list[str]) -> None:
    result = subprocess.run(command, capture_output=True, env=benchmark_environment())
    if result.returncode != 0:
        raise RuntimeError(
            f"workflow warm-up failed with exit {result.returncode}: "
            f"{result.stderr.decode(errors='replace').strip()}"
        )
    if len(result.stdout.splitlines()) != 1:
        raise ValueError(
            "workflow corpus must produce exactly one output line, "
            f"got {len(result.stdout.splitlines())}: {' '.join(command)}"
        )


def measure_command(command: list[str], corpus_bytes: int) -> dict[str, float | int]:
    with tempfile.TemporaryFile() as stderr:
        started = time.perf_counter()
        process = subprocess.Popen(
            command,
            stdout=subprocess.DEVNULL,
            stderr=stderr,
            env=benchmark_environment(),
        )
        peak_rss_kib = 0
        while process.poll() is None:
            peak_rss_kib = max(peak_rss_kib, read_peak_rss_kib(process.pid))
            time.sleep(0.0005)
        wall_seconds = time.perf_counter() - started
        if process.returncode != 0:
            stderr.seek(0)
            message = stderr.read().decode(errors="replace").strip()
            raise RuntimeError(
                f"workflow command failed with exit {process.returncode}: {message}"
            )
        if peak_rss_kib == 0:
            raise RuntimeError("workflow command peak RSS was not observable")
    return {
        "wall_seconds": wall_seconds,
        "throughput_mib_s": corpus_bytes / MIB / wall_seconds,
        "peak_rss_kib": peak_rss_kib,
    }


def read_peak_rss_kib(pid: int) -> int:
    try:
        status = Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
    except FileNotFoundError:
        return 0
    return parse_peak_rss_kib(status)


def parse_peak_rss_kib(status: str) -> int:
    for line in status.splitlines():
        if line.startswith("VmHWM:"):
            fields = line.split()
            if len(fields) == 3 and fields[2] == "kB":
                return int(fields[1])
            raise ValueError(f"invalid VmHWM line: {line}")
    return 0


def benchmark_environment() -> dict[str, str]:
    environment = os.environ.copy()
    environment.update({"LC_ALL": "C.UTF-8", "LANG": "C.UTF-8"})
    return environment


def summarize_runs(
    runs: list[dict[str, float | int]],
) -> dict[str, object]:
    if not runs:
        raise ValueError("workflow performance requires at least one run")
    fields = ("wall_seconds", "throughput_mib_s", "peak_rss_kib")

    def normalized(field: str, value: float | int) -> float | int:
        return int(value) if field == "peak_rss_kib" else round(float(value), 6)

    return {
        "runs": len(runs),
        **{
            field: normalized(
                field, statistics.median(run[field] for run in runs)
            )
            for field in fields
        },
        "run_min": {
            field: normalized(field, min(run[field] for run in runs))
            for field in fields
        },
        "run_max": {
            field: normalized(field, max(run[field] for run in runs))
            for field in fields
        },
    }


def corpus_metadata(corpus: Path, settings: CorpusSettings) -> dict[str, object]:
    digest = hashlib.sha256()
    for path in sorted(item for item in corpus.iterdir() if item.is_file()):
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(MIB), b""):
                digest.update(chunk)
    return {
        "bytes": settings.total_bytes,
        "files": settings.file_count,
        "small_files": settings.generated_small_files + 1,
        "small_file_bytes": settings.fixture_bytes,
        "large_files": settings.generated_files - settings.generated_small_files,
        "korean_percent": settings.korean_percent,
        "nfd_percent": settings.nfd_percent,
        "seed": settings.seed,
        "sha256": digest.hexdigest(),
    }


def display_command(command: list[str], corpus: Path, data_dir: Path) -> str:
    return " ".join(
        "<corpus>"
        if value == str(corpus)
        else "<data-dir>"
        if value == str(data_dir)
        else value
        for value in command
    )
