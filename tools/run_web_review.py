#!/usr/bin/env python3
"""Run the local-only Harbor human-review workbench."""

from __future__ import annotations

import argparse
import sys
import webbrowser
from pathlib import Path

from web_review_contract import (
    MAX_PACKET_BYTES,
    PACKET_PATH,
    QUESTIONNAIRE_PATH,
    ContractError,
    load_json,
    validate_harbor_questionnaire,
    validate_packet,
)
from web_review_workbench import WorkbenchSession, serve


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    root.add_argument("--scope", choices=["harbor"], required=True)
    root.add_argument("--draft", type=Path, required=True, metavar="PATH")
    root.add_argument("--final", type=Path, required=True, metavar="PATH")
    root.add_argument("--resume", action="store_true")
    root.add_argument("--host", default="127.0.0.1")
    root.add_argument("--port", type=int, default=4174)
    root.add_argument("--no-open", action="store_true")
    root.add_argument("--fictional-conformance", action="store_true", help=argparse.SUPPRESS)
    return root


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        packet = load_json(PACKET_PATH, "review packet", MAX_PACKET_BYTES)
        validate_packet(packet)
        questionnaire = load_json(QUESTIONNAIRE_PATH, "Harbor review questionnaire")
        validate_harbor_questionnaire(questionnaire, packet)
        purpose = "fictionalConformance" if arguments.fictional_conformance else "humanReviewCandidate"
        session = WorkbenchSession(
            packet,
            questionnaire,
            arguments.draft,
            arguments.final,
            purpose,
            arguments.resume,
        )
        server, url = serve(session, arguments.host, arguments.port)
        print(f"web review workbench: url={url}", flush=True)
        print("web review workbench: source=packet-only, questions=8, external_processing=false", flush=True)
        if not arguments.no_open:
            webbrowser.open(url, new=2)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            print("web review workbench: stopped without finalization", file=sys.stderr)
        finally:
            server.server_close()
    except (ContractError, OSError) as error:
        category = error.category if isinstance(error, ContractError) else "input"
        print(f"web-review-workbench: {category}: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
