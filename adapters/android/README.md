# Bounded Android capture adapter

`sightlint_android.py` is the untrusted local file adapter defined by ADR 0045. It consumes a
digest-pinned capture manifest produced by the repository-owned Android instrumentation runner and
its paired PNG. It does not connect to a device, invoke `adb`, install or execute an APK, perform
accessibility actions, or make network requests.

Build the public binary, then invoke the adapter with an output path that does not exist:

```bash
cargo build --locked -p sightlint-cli
python3 adapters/android/sightlint_android.py \
  --request evaluation/android/requests/android-atlas-clean.json \
  --repository-root . \
  --sightlint-binary target/debug/sightlint \
  --artifact-ir-out /tmp/sightlint-android-ir.json
target/debug/sightlint check /tmp/sightlint-android-ir.json --profile base --format json
```

The adapter prints one canonical partial-coverage response to stdout and writes normalized
Artifact IR only after the public `sightlint normalize` command accepts it. Adapter/protocol/input
errors use exit code `2`.

Exact View screen allocations may become `layoutBox` observations. Accessibility bounds stay
separate platform-semantic facts and never become `hitBox` or rendered geometry. The paired PNG is
validated through public `adapt-image`, uses a separate canvas, and is reconciled only at screen
extent. See `evaluation/android/README.md` for evaluation governance and explicit non-claims.
