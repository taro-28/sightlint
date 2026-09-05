#!/usr/bin/env python3
"""Build and verify SightLint's deterministic source-only alpha artifact."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
LICENSE = "MIT OR Apache-2.0"
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024
MAX_EXPANDED_BYTES = 128 * 1024 * 1024
MAX_MEMBERS = 10_000
TAG_PATTERN = re.compile(r"v[0-9]+\.[0-9]+\.[0-9]+-alpha\.[0-9]+")


class ReleaseError(ValueError):
    """A stable release-contract validation error."""


def workspace_version(root: Path = ROOT) -> str:
    """Read the workspace package version without requiring a TOML dependency."""
    in_package = False
    for line in (root / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_package = stripped == "[workspace.package]"
            continue
        if in_package:
            match = re.fullmatch(r'version\s*=\s*"([^"]+)"', stripped)
            if match is not None:
                return match.group(1)
    raise ReleaseError("Cargo.toml does not declare workspace.package.version")


def validate_tag(tag: str, root: Path = ROOT) -> str:
    """Validate the tag, license metadata, release notes, and package boundary."""
    version = workspace_version(root)
    if TAG_PATTERN.fullmatch(tag) is None or tag != f"v{version}":
        raise ReleaseError(f"release tag must be v{version}")

    for name in ("LICENSE-APACHE", "LICENSE-MIT"):
        if not (root / name).is_file():
            raise ReleaseError(f"release is missing {name}")
    if not (root / "docs" / "releases" / f"{tag}.md").is_file():
        raise ReleaseError(f"release is missing docs/releases/{tag}.md")

    cargo_manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
    if f'license = "{LICENSE}"' not in cargo_manifest:
        raise ReleaseError("workspace license metadata does not match ADR 0007")
    for crate_manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        text = crate_manifest.read_text(encoding="utf-8")
        if "publish = false" not in text or "license.workspace = true" not in text:
            relative = crate_manifest.relative_to(root).as_posix()
            raise ReleaseError(f"{relative} must remain unpublished and inherit the license")

    package = json.loads((root / "adapters" / "playwright" / "package.json").read_text(encoding="utf-8"))
    if package.get("private") is not True or package.get("license") != LICENSE:
        raise ReleaseError("Playwright package must remain private and use the project license")
    return version


def sha256(path: Path) -> str:
    """Return a lowercase SHA-256 digest for one file."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_checksum(archive: Path) -> Path:
    """Write the release checksum beside an archive."""
    checksum = archive.with_name(f"{archive.name}.sha256")
    checksum.write_bytes(f"{sha256(archive)}  {archive.name}\n".encode("utf-8"))
    return checksum


def create_source_archive(tag: str, output_dir: Path, root: Path = ROOT) -> tuple[Path, Path]:
    """Create a gzip-compressed git archive with fixed gzip metadata."""
    validate_tag(tag, root)
    output_dir.mkdir(parents=True, exist_ok=True)
    archive = output_dir / f"sightlint-{tag}-source.tar.gz"
    prefix = f"sightlint-{tag}/"
    completed = subprocess.run(
        ["git", "archive", "--format=tar", f"--prefix={prefix}", "HEAD"],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise ReleaseError(f"git archive failed: {detail or 'unknown error'}")
    if len(completed.stdout) > MAX_EXPANDED_BYTES:
        raise ReleaseError("source archive exceeds the expanded-size limit")

    with tempfile.NamedTemporaryFile(dir=output_dir, delete=False) as temporary:
        temporary_path = Path(temporary.name)
        with gzip.GzipFile(filename="", mode="wb", fileobj=temporary, compresslevel=9, mtime=0) as compressed:
            compressed.write(completed.stdout)
    temporary_path.replace(archive)
    return archive, write_checksum(archive)


def checksum_record(checksum: Path) -> tuple[str, str]:
    """Parse one strict sha256sum-compatible record."""
    text = checksum.read_text(encoding="utf-8")
    match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)\n", text)
    if match is None:
        raise ReleaseError("checksum file must contain one canonical SHA-256 record")
    return match.group(1), match.group(2)


def validate_member(member: tarfile.TarInfo, expected_root: str) -> None:
    """Reject archive entries that could escape or change extraction semantics."""
    path = PurePosixPath(member.name)
    if path.is_absolute() or ".." in path.parts or not path.parts or path.parts[0] != expected_root:
        raise ReleaseError(f"unsafe archive member: {member.name}")
    if member.issym() or member.islnk() or member.isdev() or member.isfifo():
        raise ReleaseError(f"unsupported archive member type: {member.name}")
    if not (member.isdir() or member.isfile()):
        raise ReleaseError(f"unknown archive member type: {member.name}")


def verify_archive(archive: Path, checksum: Path, tag: str) -> list[tarfile.TarInfo]:
    """Verify checksum, resource limits, paths, and required source members."""
    expected_digest, expected_name = checksum_record(checksum)
    if expected_name != archive.name or sha256(archive) != expected_digest:
        raise ReleaseError("source archive checksum does not match")
    if archive.stat().st_size > MAX_ARCHIVE_BYTES:
        raise ReleaseError("source archive exceeds the compressed-size limit")

    expected_root = f"sightlint-{tag}"
    with tarfile.open(archive, mode="r:gz") as bundle:
        members = bundle.getmembers()
    if not members or len(members) > MAX_MEMBERS:
        raise ReleaseError("source archive has an invalid member count")
    expanded = 0
    names: set[str] = set()
    for member in members:
        validate_member(member, expected_root)
        if member.name in names:
            raise ReleaseError(f"duplicate archive member: {member.name}")
        names.add(member.name)
        if member.isfile():
            expanded += member.size
            if expanded > MAX_EXPANDED_BYTES:
                raise ReleaseError("source archive exceeds the expanded-size limit")

    required = {
        f"{expected_root}/Cargo.toml",
        f"{expected_root}/LICENSE-APACHE",
        f"{expected_root}/LICENSE-MIT",
        f"{expected_root}/docs/releases/{tag}.md",
    }
    missing = required - names
    if missing:
        raise ReleaseError(f"source archive is missing required members: {sorted(missing)}")
    return members


def extract_archive(archive: Path, output_dir: Path, members: list[tarfile.TarInfo], tag: str) -> Path:
    """Extract verified regular files without tarfile.extractall path ambiguity."""
    output_dir.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, mode="r:gz") as bundle:
        for member in members:
            destination = output_dir.joinpath(*PurePosixPath(member.name).parts)
            if member.isdir():
                destination.mkdir(parents=True, exist_ok=True)
                continue
            destination.parent.mkdir(parents=True, exist_ok=True)
            source = bundle.extractfile(member)
            if source is None:
                raise ReleaseError(f"archive member has no file body: {member.name}")
            with destination.open("wb") as target:
                shutil.copyfileobj(source, target)
            os.chmod(destination, member.mode & 0o777)
    return output_dir / f"sightlint-{tag}"


def parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)

    validate = commands.add_parser("validate-tag")
    validate.add_argument("--tag", required=True)

    source = commands.add_parser("source-archive")
    source.add_argument("--tag", required=True)
    source.add_argument("--output-dir", type=Path, required=True)

    verify = commands.add_parser("verify-archive")
    verify.add_argument("--tag", required=True)
    verify.add_argument("--archive", type=Path, required=True)
    verify.add_argument("--checksum", type=Path, required=True)
    verify.add_argument("--extract-dir", type=Path)
    return root


def main(argv: list[str] | None = None) -> int:
    """Execute one release-contract operation."""
    arguments = parser().parse_args(argv)
    try:
        if arguments.command == "validate-tag":
            version = validate_tag(arguments.tag)
            print(f"release tag {arguments.tag} matches workspace {version}")
        elif arguments.command == "source-archive":
            archive, checksum = create_source_archive(arguments.tag, arguments.output_dir)
            print(archive)
            print(checksum)
        else:
            members = verify_archive(arguments.archive, arguments.checksum, arguments.tag)
            if arguments.extract_dir is not None:
                print(extract_archive(arguments.archive, arguments.extract_dir, members, arguments.tag))
            else:
                print(f"verified {len(members)} source archive members")
    except (OSError, ReleaseError, json.JSONDecodeError) as error:
        print(f"release: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
