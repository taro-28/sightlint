# PPTX source adapter 0.1.0

This local Python 3.9+ process is an untrusted acquisition adapter. It reads a bounded transitional
OOXML `.pptx`, maps supported directly declared slide shapes and groups to exact source
`layoutBox` geometry in EMUs, and asks the public `sightlint` binary to validate both optional PNG
renders and the resulting Artifact IR.

```bash
python3 adapters/pptx/sightlint_pptx.py \
  --request evaluation/pptx/requests/atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/atlas-clean.ir.json
```

The command writes a canonical response to stdout and creates the Artifact IR output exclusively.
Exit `0` means bounded acquisition succeeded with explicitly partial coverage; exit `2` means an
operational or validation error. Rule findings and exit `1` remain owned by a subsequent public
`sightlint check` invocation.

Version `0.1.0` is local-only and dependency-free. It never runs Office, follows external
relationships, extracts the archive, or serializes shape names/full source text. It records only
text byte count and an unsalted digest; low-entropy text may still be guessed, so output remains
sensitive source-derived metadata. Master/layout objects, theme-resolved styles, unsupported
objects/transforms, visible ink, text layout, font substitution, and rendered node identity are
not claimed. See ADR 0043 and `evaluation/pptx/` for the exact contract and regression corpus.
