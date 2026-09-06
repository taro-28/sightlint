# iOS acquisition and rule evaluation contract

This directory defines the versioned evaluation contract selected by ADR 0046 and Issue #60. The
corpus uses a repository-owned realistic UIKit account/settings fixture captured through paired
source instrumentation, a pre-query screenshot, and XCUITest on one pinned iPhone 17 Pro simulator
profile.

The required public cases are:

- `ios-atlas-clean` — smoke baseline;
- `ios-atlas-off-canvas-control-mutant` — development mutation affecting only one exact source
  layout allocation;
- `ios-atlas-scroll-offscreen-hard-negative` — challenge case whose valid offscreen or conflicting
  platform observation must remain excluded/abstaining.

Acquisition truth belongs in `annotations/acquisition.json`; rule truth belongs independently in
`annotations/rules.json`. Adapter output and CheckReports are never oracle data. Capture manifests
are platform-source artifacts produced independently of the file adapter and reviewed against the
fixture source and screenshots.

All source, captures, labels, and splits are visible to implementers. There is no protected
holdout, independent reviewer, representative application/device/runtime sample, or general iOS,
accessibility, or UI/UX accuracy claim. Perfect corpus metrics are regression evidence only.

Run `python3 tools/check_ios_evaluation.py` to verify source/capture drift, asset/request bindings,
independently authored acquisition and rule annotations, privacy assertions, splits, abstentions,
and the explicit lack of a protected holdout. Authentic recapture is a separate pinned-macOS
maintainer operation documented with the fixture application.
