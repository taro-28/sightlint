## Summary

<!-- What changes, and which milestone or contract does it advance? -->

## Architectural fit

<!-- Link relevant principles, ADRs, rule contracts, or explain why none are affected. -->

## Evidence and uncertainty

<!-- What observations prove the behavior? What remains cantTell or untested? -->

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`
- [ ] Schema, fixtures, and documentation are updated when applicable
- [ ] No probabilistic result is presented as deterministic evidence
- [ ] Privacy and untrusted-input implications were considered
