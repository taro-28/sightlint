# Exact-color background and segmentation benchmark

This corpus implements the evaluation contract from ADR 0039. It captures a realistic but
fictional repository-owned Web UI through the isolated Playwright adapter, then runs the built
`sightlint benchmark-image-segmentation` command on each temporary screenshot. It compares three
named policies; it does not change or replace the strict `inspect-image` default.

The acquisition oracle is `annotations/acquisition.json`. Visible-surface targets were authored
from the fixture source and reviewed before candidate implementation. The separate
`annotations/rules.json` document records that no executable rule exists and every rule result is
therefore `untested`; semantic applicability remains `cantTell` or `inapplicable`. Generated
screenshots and benchmark reports are never checked in as expected output.

The corpus includes clean, targeted edge-contamination, recoloring, translation, device-scale,
modal, split-pane, gradient/illustration, and checkerboard stress states. It intentionally exposes
coverage/precision tradeoffs: a ranked exact color can always produce geometry while still being
an unsafe background hypothesis. Resource-limit abstention is expected and produces no partial
regions.

Metrics are small-corpus regression counts: usable-case coverage, region precision/recall, integer
bounds error, unsafe hypotheses, correct abstention, fragmentation/false grouping, deterministic
agreement, and mutation/metamorphic observations. They are not representative real-world UI/UX
accuracy. There is no private holdout or independent reviewer, no semantic grouping, no rule
verdict, no color management or alpha compositing, and no universal score.

The reviewed baseline measurements and non-admission decision are recorded in
[`results.md`](results.md). Run the dedicated public-process gate with:

```bash
cargo build --locked -p sightlint-cli
npm --prefix adapters/playwright run test:segmentation-evaluation
```
