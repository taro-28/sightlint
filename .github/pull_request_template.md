## Summary

<!-- What changes, and which milestone or contract does it advance? -->

## Architectural fit

<!-- Link relevant principles, ADRs, rule contracts, or explain why none are affected. -->

## Evidence and uncertainty

<!-- What observations prove the behavior? What remains cantTell or untested? -->

## Fixture and E2E coverage

<!-- Name the pass, fail/mutation, cantTell, inapplicable, and malformed cases added or explain why a category is not applicable. -->

## Validation

- [ ] `python3 tools/generate_e2e_fixtures.py --check`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --workspace --all-features`
- [ ] `cargo test --locked -p sightlint-cli --test e2e`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps`
- [ ] User-visible behavior is exercised through the built binary, not only library APIs
- [ ] Each new rule has a passing fixture and targeted mutation fixture
- [ ] Schema, fixtures, and documentation are updated when applicable
- [ ] No probabilistic result is presented as deterministic evidence
- [ ] Privacy and untrusted-input implications were considered
