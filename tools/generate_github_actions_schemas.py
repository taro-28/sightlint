#!/usr/bin/env python3
"""Generate deterministic GitHub Actions integration schemas from Rust types."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCHEMAS = {
    "github-source-map": ROOT / "schemas" / "github-source-map.schema.json",
    "github-actions-report": ROOT / "schemas" / "github-actions-report.schema.json",
}


def render(kind: str) -> str:
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--locked",
            "-p",
            "sightlint-cli",
            "--",
            "schema",
            "--kind",
            kind,
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    if completed.stderr:
        raise SystemExit(f"schema generation wrote unexpected stderr: {completed.stderr}")
    return completed.stdout


def apply(*, check: bool) -> int:
    differences: list[str] = []
    for kind, path in SCHEMAS.items():
        expected = render(kind)
        actual = path.read_text(encoding="utf-8") if path.exists() else None
        if actual != expected:
            differences.append(str(path.relative_to(ROOT)))
            if not check:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(expected, encoding="utf-8", newline="\n")

    if check and differences:
        print("GitHub Actions integration schemas differ from their Rust source:")
        for path in differences:
            print(f"- {path}")
        return 1

    if not check:
        print(f"generated {len(SCHEMAS)} GitHub Actions integration schemas")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail when committed schemas differ from their Rust source",
    )
    arguments = parser.parse_args()
    return apply(check=arguments.check)


if __name__ == "__main__":
    raise SystemExit(main())
