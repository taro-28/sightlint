## Issue and milestone

<!-- Link one primary issue and the roadmap milestone/epic. Explain any deviation from issue #34's sequence. -->

## Base and scope

- Base branch/commit:
- Final head commit:
- User-visible claim made reachable by this PR:

<!-- Confirm this branch started from the latest green main, not a historical branch. -->

## Summary

<!-- What changes? Describe the smallest complete vertical path, not only internal modules. -->

## Architectural fit and decisions

<!-- Link accepted ADRs. State whether this changes architecture, schema, protocol, trust boundary,
compatibility, policy precedence, resource model, or report semantics. New ADR numbers continue at
0051 or later. -->

## Evidence, applicability, and policy

<!-- Identify exact/declared/empirical/inferred observations, evidence grade, selectors, units,
coordinate spaces, applicability, policy source, tolerance, alternatives, conflicts, and what
becomes cantTell/inapplicable/untested. Do not treat confidence as outcome or severity. -->

## Privacy, security, and resources

<!-- Describe untrusted inputs/processes, network/transmission behavior, time/memory/node/output
limits, dependency/supply-chain effects, and fixture provenance/license/privacy. -->

## Compatibility

<!-- Address Artifact IR, report, adapter/perception protocol, extension, rule semantics,
configuration/profile, CLI/exit codes, and evaluation manifest as applicable. -->

## Fixture and evaluation coverage

<!-- Name the applicable pass, targeted mutation/fail, cantTell, inapplicable, untested, malformed,
boundary, resource, determinism, metamorphic, differential, hard-negative, acquisition-oracle, and
rule-oracle cases. Explain why a category is not applicable rather than silently omitting it. -->

## Non-claims and remaining risks

<!-- State what this PR does not prove. Synthetic success is not real-world UI/UX accuracy. An
observation/advisory report is not a trusted CheckReport failure. -->

## Documentation and handoff

- [ ] `docs/handoff.md` was updated if current behavior, commands, status, priority, or risk changed
- [ ] `docs/roadmap.md` was updated if sequencing, milestone status, or exit criteria changed
- [ ] relevant ADR/index, rationale, evaluation, development, and user documentation were updated
- [ ] no historical Draft PR or legacy branch is being treated as authoritative

## Local validation

- [ ] `python3 tools/generate_e2e_fixtures.py --check`
- [ ] `python3 tools/generate_github_actions_schemas.py --check`
- [ ] `python3 tools/check_github_actions_evaluation.py`
- [ ] `python3 tools/generate_raster_corpus.py --check`
- [ ] `python3 tools/generate_alpha_assets.py --check`
- [ ] `python3 tools/generate_inspection_corpus.py --check`
- [ ] `python3 tools/check_alpha_evaluation.py`
- [ ] `python3 tools/check_png_format_demand.py`
- [ ] `python3 tools/check_web_evaluation.py`
- [ ] `python3 tools/check_perception_evaluation.py`
- [ ] `python3 tools/generate_pptx_fixtures.py --check`
- [ ] `python3 tools/check_pptx_evaluation.py`
- [ ] `python3 -m unittest adapters/pptx/tests/test_adapter.py`
- [ ] `python3 -m venv .venv-sightlint-pdf`
- [ ] `.venv-sightlint-pdf/bin/python -m pip install --disable-pip-version-check --require-hashes -r adapters/pdf/requirements.txt`
- [ ] `export PATH="$PWD/.venv-sightlint-pdf/bin:$PATH"`
- [ ] `python3 tools/generate_pdf_fixtures.py --check`
- [ ] `python3 tools/check_pdf_evaluation.py`
- [ ] `python3 -m unittest adapters/pdf/tests/test_adapter.py`
- [ ] `python3 tools/generate_android_fixtures.py --check`
- [ ] `python3 tools/check_android_evaluation.py`
- [ ] `python3 -m py_compile adapters/android/sightlint_android.py`
- [ ] `python3 tools/generate_ios_fixtures.py --check`
- [ ] `python3 tools/check_ios_evaluation.py`
- [ ] `python3 -m py_compile adapters/ios/sightlint_ios.py`
- [ ] `python3 tools/release.py validate-tag --tag v0.1.0-alpha.2`
- [ ] `python3 tools/check_dependency_licenses.py`
- [ ] `python3 -m unittest tools/test_release.py`
- [ ] `npm --prefix adapters/playwright ci --ignore-scripts`
- [ ] `npm --prefix adapters/playwright run install:browser`
- [ ] `npm --prefix adapters/playwright run check`
- [ ] `npm --prefix adapters/perception ci --ignore-scripts`
- [ ] `npm --prefix adapters/perception run check`
- [ ] `cargo build --locked -p sightlint-cli`
- [ ] `npm --prefix adapters/perception run test:e2e`
- [ ] `npm --prefix adapters/playwright run test:e2e`
- [ ] `npm --prefix adapters/playwright run test:managed-e2e`
- [ ] `npm --prefix adapters/playwright run test:server-e2e`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --workspace --all-features`
- [ ] `cargo test --locked -p sightlint-cli --test e2e`
- [ ] `cargo test --locked -p sightlint-cli --test github_actions_e2e`
- [ ] `cargo test --locked -p sightlint-cli --test png_filter_e2e`
- [ ] `cargo test --locked -p sightlint-cli --test png_raster_corpus -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test alpha_geometry_evaluation_e2e -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test image_inspection_e2e -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test image_segmentation_benchmark_e2e -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test evaluation_corpus`
- [ ] `cargo test --locked -p sightlint-cli --test github_actions_evaluation_e2e -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test web_evaluation_corpus -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test pptx_evaluation_e2e -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test pdf_evaluation_e2e -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test android_evaluation_e2e -- --nocapture`
- [ ] `cargo test --locked -p sightlint-cli --test ios_evaluation_e2e -- --nocapture`
- [ ] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps`
- [ ] `cargo +1.85.0 check --workspace --all-targets --all-features --locked`
- [ ] any new generator/process-adapter/public-E2E command is also in CI, `AGENTS.md`, and handoff

## Remote verification

- [ ] all required jobs passed on the exact final head, not an older commit
- [ ] Linux, macOS, Windows, and MSRV coverage passed as applicable
- [ ] changed files and trust/evidence boundaries were reviewed after the final push
- [ ] no self-writing or temporary write-enabled workflow was added
- [ ] no duplicate/unconnected implementation remains
- [ ] user-visible behavior is exercised through the built binary/process, not only library APIs
- [ ] no oracle was weakened merely to match implementation output

### Final-head CI evidence

- Workflow run:
- Required job results:

## Post-merge verification

<!-- Complete after merge or record in the issue/PR conversation. -->

- [ ] `main` points to the intended commit/tree
- [ ] `main` CI passed on that exact commit
- [ ] no temporary workflow, stale Draft PR, or generated drift remains
- [ ] issue/status/handoff were updated and the branch is scheduled for deletion
