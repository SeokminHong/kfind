from __future__ import annotations

from pathlib import Path

from readme_guard import check_repository


REPOSITORY = Path(__file__).resolve().parents[2]


def main() -> int:
    violations = check_repository(REPOSITORY)
    for violation in violations:
        print(
            f"{violation.path}:{violation.line}: {violation.reason}: "
            f"{violation.text.strip()}"
        )
    return 1 if violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
