"""Tests for deterministic, bounded source release packaging."""

from __future__ import annotations

import gzip
import hashlib
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

from tools import release


class ReleaseTests(unittest.TestCase):
    """Exercise tag validation, byte stability, checksum, and extraction safety."""

    def repository(self, directory: Path) -> Path:
        root = directory / "repository"
        files = {
            "Cargo.toml": (
                '[workspace]\nmembers = ["crates/example"]\n\n'
                '[workspace.package]\nversion = "0.1.0-alpha.2"\n'
                'license = "MIT OR Apache-2.0"\n'
            ),
            "LICENSE-APACHE": "Apache test license\n",
            "LICENSE-MIT": "MIT test license\n",
            "README.md": "# Test release\n",
            "adapters/playwright/package.json": (
                '{"name":"test","private":true,"license":"MIT OR Apache-2.0"}\n'
            ),
            "crates/example/Cargo.toml": (
                '[package]\nname = "example"\nversion.workspace = true\n'
                'publish = false\nlicense.workspace = true\n'
            ),
            "crates/example/src/lib.rs": "pub fn value() -> u8 { 1 }\n",
            "docs/releases/v0.1.0-alpha.2.md": "# Test alpha\n",
        }
        for name, content in files.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content.encode("utf-8"))
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        environment = {
            **os.environ,
            "GIT_AUTHOR_DATE": "2026-01-01T00:00:00Z",
            "GIT_COMMITTER_DATE": "2026-01-01T00:00:00Z",
        }
        subprocess.run(
            ["git", "-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "-qm", "fixture"],
            cwd=root,
            check=True,
            env=environment,
        )
        return root

    def test_source_archive_is_byte_stable_and_safely_extracts(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            directory = Path(raw_directory)
            root = self.repository(directory)
            first, first_checksum = release.create_source_archive(
                "v0.1.0-alpha.2", directory / "first", root
            )
            second, second_checksum = release.create_source_archive(
                "v0.1.0-alpha.2", directory / "second", root
            )
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first_checksum.read_text(), second_checksum.read_text())

            members = release.verify_archive(first, first_checksum, "v0.1.0-alpha.2")
            extracted = release.extract_archive(
                first, directory / "unpacked", members, "v0.1.0-alpha.2"
            )
            self.assertEqual((extracted / "README.md").read_text(), "# Test release\n")

    def test_wrong_tag_and_corrupt_checksum_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            directory = Path(raw_directory)
            root = self.repository(directory)
            with self.assertRaisesRegex(release.ReleaseError, "release tag must be"):
                release.validate_tag("v0.1.0-alpha.3", root)
            archive, checksum = release.create_source_archive(
                "v0.1.0-alpha.2", directory / "dist", root
            )
            archive.write_bytes(archive.read_bytes() + b"corrupt")
            with self.assertRaisesRegex(release.ReleaseError, "checksum does not match"):
                release.verify_archive(archive, checksum, "v0.1.0-alpha.2")

    def test_unsafe_archive_member_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            directory = Path(raw_directory)
            archive = directory / "sightlint-v0.1.0-alpha.2-source.tar.gz"
            raw_tar = io.BytesIO()
            with tarfile.open(fileobj=raw_tar, mode="w") as bundle:
                member = tarfile.TarInfo("../escape")
                member.size = 1
                bundle.addfile(member, io.BytesIO(b"x"))
            with archive.open("wb") as target:
                with gzip.GzipFile(filename="", mode="wb", fileobj=target, mtime=0) as compressed:
                    compressed.write(raw_tar.getvalue())
            checksum = archive.with_name(f"{archive.name}.sha256")
            checksum.write_text(
                f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(release.ReleaseError, "unsafe archive member"):
                release.verify_archive(archive, checksum, "v0.1.0-alpha.2")


if __name__ == "__main__":
    unittest.main()
