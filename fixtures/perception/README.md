# Perception process fixtures

These fixtures exercise protocol/process behavior only. The small segmentation-report documents
are hand-authored normalized sensor inputs, not product ground truth and not screenshots. Realistic
differential evaluation uses the repository-owned Northstar Web application under
`evaluation/perception/` and keeps its temporary browser captures out of the repository.

The fake workers cover malformed output, identity mismatch, timeout, and stdout overflow. They do
not model production failure rates. All files are repository-owned under `MIT OR Apache-2.0` and
contain no customer or personal data.
