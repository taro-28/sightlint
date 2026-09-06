# Atlas iOS fixture application

This repository-owned programmatic UIKit application is a fictional account/settings screen used
only for SightLint evaluation. It contains no customer data, credentials, analytics, or network
code. Source and captures are licensed `MIT OR Apache-2.0` with the rest of the repository.

## Pinned capture environment

- Xcode: 26.3 build 17C529
- iOS simulator SDK: 26.2
- iOS simulator runtime: 26.3.1 build 23D8133
- device: iPhone 17 Pro (`iPhone18,1`), arm64, 402×874 points at 3×
- Swift: 6.2.4
- deployment target: iOS 26.0
- application/capture runner version: 0.1.0
- locale/direction: `en-US` / LTR
- Dynamic Type: large
- appearance: light
- animations: disabled

The Xcode project has no package dependency, signing requirement for simulator capture, generated
project file, or checked-in build product.

## Build and capture

Boot the pinned simulator, then pass its exact UDID to the repository capture helper:

```bash
python3 tools/generate_ios_fixtures.py --capture \
  --device-id 00000000-0000-0000-0000-000000000000
```

The helper rejects a different Xcode, Swift, SDK, runtime, device, model, architecture, extent, or
scale. It runs the three XCUITests, exports their attachments, reads the fixture-authored UIKit
source hierarchy from the application container, and writes canonical captures without committing
them. After independent native-fact and visual review, update the pinned digests deliberately and
run `python3 tools/generate_ios_fixtures.py --check`.

The source hierarchy is written after layout stabilization. XCUITest then takes the screenshot
before querying individual elements because querying an offscreen accessibility element can affect
scroll state. Source, screenshot, and XCUI observations are still sequential, not atomic.

Only named classic UIKit views on this fixture/profile are covered. SwiftUI, custom accessibility
containers, device hardware, multiple scenes/windows, keyboard, dialogs, overlays, localization,
other Dynamic Type categories, rotation, focus navigation, touch delegates, occlusion, and dynamic
interaction are not covered.
