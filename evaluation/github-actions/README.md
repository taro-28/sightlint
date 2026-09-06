# GitHub Actions integration evaluation

This corpus evaluates the deterministic projection from the existing Rust `CheckReport` into a
GitHub Actions job check. It does not evaluate a second rule engine or a hosted GitHub App.

The authorities are intentionally separate:

- `corpus.json` identifies runnable Artifact IR projections;
- `annotations/rules.json` records rule truth from the existing Web/interaction contracts without
  using implementation output;
- `source-maps/*.json` declare exact repository source locations without storing expected rule
  outcomes;
- `annotations/projection.json` states the expected annotation disposition without redefining the
  rule verdict;
- `metric-contract.json` fixes reviewed denominators and acceptance thresholds without storing
  current implementation output.

The dashboard cases use the realistic Atlas dashboard application. The interaction cases use
existing deterministic projections of the repository-owned Atlas settings application. Their
native/browser acquisition is already evaluated by the Web and interaction corpora; this suite
tests the downstream GitHub projection and does not reinterpret acquisition output as truth.

All content is fictional, owned by this repository, licensed `MIT OR Apache-2.0`, and contains no
personal/customer data, secrets, third-party assets, external processing, or telemetry. The labels,
mutations, source locations, and fixes are public and visible to implementers. This is a
smoke/development regression corpus, not a protected holdout or representative sample.

Run the governance check and public-binary E2E with:

```bash
python3 tools/check_github_actions_evaluation.py
cargo test --locked -p sightlint-cli --test github_actions_evaluation_e2e -- --nocapture
```

Do not generate an oracle from `sightlint github-check` output. Change an expectation only when a
reviewed source, rule, or projection contract changed; never adjust it merely to make the test pass.
