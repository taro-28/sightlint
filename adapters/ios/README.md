# Bounded iOS source/XCUI capture adapter

`sightlint_ios.py` is the untrusted local file adapter defined by ADR 0046. It consumes a
digest-pinned capture manifest produced by the repository-owned UIKit/XCUITest fixture and its
paired PNG. It does not invoke Xcode or `simctl`, boot a simulator, install or launch an app,
execute an XCUI action, parse an `.xcresult`, or make network requests.

Build the public binary, then invoke the adapter with an output path that does not exist:

```bash
cargo build --locked -p sightlint-cli
python3 adapters/ios/sightlint_ios.py \
  --request evaluation/ios/requests/ios-atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/sightlint-ios-ir.json
target/debug/sightlint check /tmp/sightlint-ios-ir.json --profile base --format json
```

The adapter prints one canonical partial-coverage response and writes normalized Artifact IR only
after the public `sightlint normalize` command accepts it. Adapter/protocol/input errors use exit
code `2`; the Rust rule process alone owns findings and exit code `1`.

Admitted UIKit source allocations may become point-valued `layoutBox` observations. A clipped
direct `UIScrollView` content container and fully offscreen source views remain extension-only.
XCUI frames stay separate `platformSemantics` facts and never become hit, render, or ink geometry.
The paired PNG is validated through public `adapt-image`, uses a separate device-pixel canvas, and
is reconciled only by exact point extent and display scale. Source/XCUI frame disagreement is
preserved as `frameConflict`.

Protocol v0.1 accepts only the pinned repository-owned Atlas capture source and Xcode/runtime/device
profile. It serializes digests and byte lengths instead of plaintext UIKit/XCUI labels and values,
but identifiers, selectors, class/bundle/device/build metadata, geometry, paths, screenshots, and
unsalted low-entropy digests remain sensitive. The process has no external processing or retention
and adds no package dependency; request limits do not constitute an operating-system sandbox. See
`evaluation/ios/README.md` for evaluation governance and explicit non-claims.
