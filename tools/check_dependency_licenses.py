#!/usr/bin/env python3
"""Reject missing or unreviewed licenses in the locked Cargo and npm graphs."""

from __future__ import annotations

import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PROJECT_LICENSE = "MIT OR Apache-2.0"
ALLOWED_IDENTIFIERS = {
    "0BSD",
    "Apache-2.0",
    "BSD-3-Clause",
    "MIT",
    "Unicode-3.0",
    "Unlicense",
    "Zlib",
}
OPERATORS = {"AND", "OR", "WITH"}


def license_identifiers(expression: str) -> set[str]:
    """Extract SPDX-like identifiers from the dependency metadata expression."""
    return {
        token
        for token in re.findall(r"[A-Za-z0-9.-]+", expression)
        if token not in OPERATORS
    }


def validate_license(expression: object, package: str) -> None:
    """Require a nonempty expression containing only reviewed identifiers."""
    if not isinstance(expression, str) or not expression.strip():
        raise SystemExit(f"{package} does not declare a license")
    unknown = license_identifiers(expression) - ALLOWED_IDENTIFIERS
    if unknown:
        raise SystemExit(f"{package} uses unreviewed license identifiers: {sorted(unknown)}")


def cargo_licenses() -> int:
    """Validate workspace metadata and every locked Cargo dependency."""
    completed = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    )
    metadata = json.loads(completed.stdout)
    external = 0
    for package in metadata["packages"]:
        identifier = f"Cargo {package['name']}@{package['version']}"
        if package["source"] is None:
            if package["license"] != PROJECT_LICENSE:
                raise SystemExit(f"{identifier} does not inherit {PROJECT_LICENSE}")
        else:
            validate_license(package["license"], identifier)
            external += 1
    return external


def npm_licenses() -> tuple[int, int]:
    """Validate every private npm package root and locked external dependency."""
    lock_paths = [
        ROOT / "adapters" / "perception" / "package-lock.json",
        ROOT / "adapters" / "playwright" / "package-lock.json",
    ]
    external = 0
    for lock_path in lock_paths:
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        root_package = lock["packages"].get("")
        label = lock_path.parent.relative_to(ROOT)
        if not isinstance(root_package, dict) or root_package.get("license") != PROJECT_LICENSE:
            raise SystemExit(f"npm root package {label} does not declare {PROJECT_LICENSE}")
        for path, package in lock["packages"].items():
            if path == "":
                continue
            name = package.get("name") or path.removeprefix("node_modules/")
            validate_license(package.get("license"), f"npm {name}@{package.get('version', '?')}")
            external += 1
    return len(lock_paths), external


def python_licenses() -> int:
    """Validate the exact hash-locked Python adapter dependency records."""
    lock_path = ROOT / "adapters" / "pdf" / "dependency-lock.json"
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    packages = lock.get("packages")
    if lock.get("lockVersion") != "0.1.0" or not isinstance(packages, list):
        raise SystemExit("PDF Python dependency lock has an unsupported shape")
    if len(packages) != 1:
        raise SystemExit("PDF Python dependency lock must contain exactly one package")
    package = packages[0]
    if not isinstance(package, dict):
        raise SystemExit("PDF Python dependency lock contains an invalid package")
    identifier = f"Python {package.get('name', '?')}@{package.get('version', '?')}"
    validate_license(package.get("license"), identifier)
    if (
        package.get("name") != "pypdf"
        or package.get("version") != "6.17.0"
        or package.get("requiresPython") != ">=3.9"
    ):
        raise SystemExit(f"{identifier} differs from the reviewed PDF dependency")
    digest = package.get("sha256")
    if not isinstance(digest, str) or re.fullmatch(r"sha256:[0-9a-f]{64}", digest) is None:
        raise SystemExit(f"{identifier} has no valid wheel digest")
    requirements = (ROOT / "adapters" / "pdf" / "requirements.txt").read_text(
        encoding="utf-8"
    )
    if (
        "--only-binary=:all:" not in requirements
        or f"pypdf=={package['version']} --hash={digest}" not in requirements
    ):
        raise SystemExit(f"{identifier} is not exact and hash-locked in requirements.txt")
    return len(packages)


def main() -> None:
    """Validate every locked package ecosystem."""
    cargo_count = cargo_licenses()
    npm_roots, npm_count = npm_licenses()
    python_count = python_licenses()
    print(
        f"dependency licenses: {cargo_count} Cargo and {npm_count} external npm packages "
        f"across {npm_roots} private roots, plus {python_count} Python package, verified"
    )


if __name__ == "__main__":
    main()
