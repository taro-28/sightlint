# Atlas Android fixture application

This repository-owned application is a fictional account/settings screen used only for SightLint
evaluation. It contains no customer, credential, analytics, network, or personal data. Source and
captures are licensed `MIT OR Apache-2.0` with the rest of the repository.

## Pinned capture environment

- Android API: 35
- emulator image: `system-images;android-35;google_apis;arm64-v8a`
- AVD hardware profile: Pixel 8, 1080×2400, 420 dpi, portrait
- Java: 17
- Gradle: 8.13
- Android Gradle Plugin: 8.10.1
- application/runner version: 0.1.0
- locale/direction: `en-US` / LTR
- font scale: 1.0
- animation scales: 0

The project intentionally does not commit a Gradle wrapper binary or SDK. Capture-time tooling is
not a SightLint runtime dependency or release artifact.

## Build and capture

Start the pinned `Pixel_8` AVD, then run the repository capture helper with explicit tool paths:

```bash
python3 tools/generate_android_fixtures.py --capture \
  --gradle-bin /path/to/gradle-8.13/bin/gradle \
  --adb-bin /path/to/android-sdk/platform-tools/adb \
  --emulator-bin /path/to/android-sdk/emulator/emulator \
  --serial emulator-5554
```

The helper verifies tool and AVD identity, builds both APKs, disables emulator animations, captures
all three states, and writes the capture files without committing them. After independent visual
and native-fact review, update the evaluation digests deliberately and run `--check`.

The hierarchy and screenshot are sequential captures of a static, animation-disabled state; they
are not atomic. Classic View layout and `AccessibilityNodeInfo` are covered. Compose, WebView
descendants, multiple application windows, IME, dialogs, overlays, magnification, foldables,
dynamic interaction, and physical touch delegates are not covered.
