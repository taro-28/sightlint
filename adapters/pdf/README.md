# Bounded PDF source adapter

`sightlint_pdf.py` is an untrusted local process adapter for the narrow PDF contract accepted in
ADR 0044. It acquires explicit unrotated page boxes and rectangular internal Link annotation hit
regions. It does not interpret text, tags, paint, images, forms, actions, reading order, or viewer
behavior.

Install the reviewed pure-Python parser into an isolated Python 3.9+ environment:

```bash
python3 -m venv .venv-sightlint-pdf
.venv-sightlint-pdf/bin/python -m pip install --require-hashes -r adapters/pdf/requirements.txt
export PATH="$PWD/.venv-sightlint-pdf/bin:$PATH"
cargo build --locked -p sightlint-cli
```

Then invoke the adapter with a digest-pinned request and an output path that does not exist:

```bash
python3 adapters/pdf/sightlint_pdf.py \
  --request evaluation/pdf/requests/atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/sightlint-pdf-ir.json
target/debug/sightlint check /tmp/sightlint-pdf-ir.json --profile base --format json
```

The process prints one canonical partial-coverage response to stdout, writes Artifact IR only
after the public `normalize` command accepts it, and uses exit code `2` for adapter errors. It
never follows a PDF destination or action and makes no network request. Paths must resolve below
the supplied repository root. The pinned parser and request budgets reduce the selected attack
surface but do not provide an OS sandbox.

The response, extension, dependency, fixture, oracle, and metric contracts are versioned at
`0.1.0`. See `evaluation/pdf/README.md` for corpus governance and non-claims.
