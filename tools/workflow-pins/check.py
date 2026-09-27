"""Reject mutable external action refs in repository workflows."""

import re
from pathlib import Path


ACTION_REF = re.compile(r"^[^@\s]+/[^@\s]+@[0-9a-f]{40}$")
USES = re.compile(r"^\s*(?:-\s*)?uses:\s*([^\s#]+)")


def main() -> None:
    failures = []
    for path in sorted(Path(".github/workflows").glob("*.yml")):
        for number, line in enumerate(path.read_text().splitlines(), 1):
            match = USES.match(line)
            if match is None:
                continue
            ref = match.group(1)
            if not ref.startswith("./") and ACTION_REF.fullmatch(ref) is None:
                failures.append(f"{path}:{number}: {ref}")
    if failures:
        raise SystemExit("external actions need full commit SHAs:\n" + "\n".join(failures))
    print("All external GitHub Actions use full commit SHAs.")


if __name__ == "__main__":
    main()
