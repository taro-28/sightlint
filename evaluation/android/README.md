# Android acquisition and rule evaluation

This corpus is the public regression evaluation for ADR 0045 and Issue #56. It uses a
repository-owned realistic Android account/settings fixture application captured through Android
platform instrumentation on one pinned API 35 Pixel 8 emulator profile.

The three public cases are deliberately small:

- `android-atlas-clean` — smoke baseline;
- `android-atlas-off-canvas-control-mutant` — development mutation where only the Save button's
  View allocation moves beyond the screen while the platform accessibility rectangle is clipped;
- `android-atlas-scroll-offscreen-hard-negative` — challenge case with ordinary offscreen scroll
  content and inverted platform accessibility bounds that must remain conflict/abstention evidence.

Acquisition truth lives in `annotations/acquisition.json`. Rule truth lives independently in
`annotations/rules.json`. Adapter output and CheckReports are never stored as oracle data. Capture
manifests are platform-source artifacts produced independently of the file adapter and reviewed
against the fixture source and screenshots.

All source, captures, labels, and splits are visible to implementers. There is no protected
holdout, independent reviewer, representative application/device/API sample, or general Android,
accessibility, or UI/UX accuracy claim. Perfect corpus metrics are regression evidence only.

Run the static governance and drift checks with:

```bash
python3 tools/generate_android_fixtures.py --check
python3 tools/check_android_evaluation.py
```
