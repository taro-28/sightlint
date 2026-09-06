#!/usr/bin/env python3
"""Prepare, validate, and finalize deterministic source-only Web review records."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from web_review_contract import (
    BLANK_SUBMISSION_PATH,
    MAX_PACKET_BYTES,
    PACKET_PATH,
    ContractError,
    build_blank_submission,
    build_packet,
    canonical_bytes,
    finalize_submission,
    load_json,
    pretty_bytes,
    validate_packet,
    validate_submission,
)


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    actions = root.add_mutually_exclusive_group()
    actions.add_argument("--check", action="store_true", help="check committed generated records")
    actions.add_argument("--write", action="store_true", help="write committed generated records")
    actions.add_argument("--validate-packet", type=Path, metavar="PATH")
    actions.add_argument("--validate-submission", type=Path, metavar="PATH")
    actions.add_argument("--finalize-submission", type=Path, metavar="PATH")
    root.add_argument("--packet", type=Path, default=PACKET_PATH)
    return root


def check_or_write(write: bool) -> None:
    packet = build_packet()
    validate_packet(packet)
    blank = build_blank_submission(packet)
    validate_submission(blank, packet)
    generated = (
        (PACKET_PATH, pretty_bytes(packet)),
        (BLANK_SUBMISSION_PATH, pretty_bytes(blank)),
    )
    if write:
        for path, raw in generated:
            path.write_bytes(raw)
        print("web review prepare: wrote packet=1, blank_submission=1")
        return
    for path, expected in generated:
        try:
            observed = path.read_bytes()
        except OSError:
            raise ContractError("drift", f"generated file is missing: {path.name}")
        if observed != expected:
            raise ContractError("drift", f"generated file is stale: {path.name}")
    print("web review prepare: packet=valid, blank_submission=valid, drift=false")


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        if arguments.write:
            check_or_write(True)
        elif arguments.validate_packet is not None:
            packet = load_json(arguments.validate_packet, "review packet", MAX_PACKET_BYTES)
            validate_packet(packet)
            print("web review prepare: packet=valid, evidence_eligible=false")
        elif arguments.validate_submission is not None:
            packet = load_json(arguments.packet, "review packet", MAX_PACKET_BYTES)
            submission = load_json(arguments.validate_submission, "reviewer submission")
            validate_submission(submission, packet)
            print(f"web review prepare: submission={submission['lifecycle']}, evidence_status={submission['evidenceStatus']}")
        elif arguments.finalize_submission is not None:
            packet = load_json(arguments.packet, "review packet", MAX_PACKET_BYTES)
            submission = load_json(arguments.finalize_submission, "reviewer submission")
            sys.stdout.buffer.write(canonical_bytes(finalize_submission(submission, packet)))
        else:
            check_or_write(False)
    except (ContractError, OSError) as error:
        category = error.category if isinstance(error, ContractError) else "input"
        print(f"web-review-prepare: {category}: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

