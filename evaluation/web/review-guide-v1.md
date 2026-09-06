# Public Web review operations guide v1

- Guide version: `1.0.0`
- Governing decision: ADR 0053
- Human task: issue #77
- Operational holdout: issue #74, separately gated

## What this workflow does

The workflow packages the public Atlas and Harbor fixture source and capture requests without
including an answer. It lets a reviewer author acquisition observations and rule judgments in
separate fields, lock those bytes with a digest, and compare only afterward with the current public
oracles.

The fixture state and source are visible. This is source-first review, not blind evaluation. The
tools do not create human judgment or verify identity, qualification, independence, conflicts of
interest, or signatures. A structurally valid file does not by itself complete issue #77.

## Inputs that may be reviewed before finalization

Use only `evaluation/web/review-packet.json`. Its embedded file inventory contains:

- repository-owned Atlas and Harbor HTML, CSS, and JavaScript source; and
- their versioned offline Playwright capture requests.

Do not open SightLint output, generated screenshots, captured Artifact IR, reports, diagnostics,
`evaluation/web/annotations/*`, or another expected-value source before the first-pass submission
is finalized. Do not run `sightlint`, `sightlint-web`, or product evaluation E2E as a way to answer
the review.

The packet's screenshot field is only the logical output reference already present in a capture
request. No screenshot bytes are embedded.

## Authoring a submission

Copy `evaluation/web/reviewer-submission.blank.json` to a working location outside the repository.
Do not edit the generated blank file in place. For a Harbor pilot, keep only the four Harbor cases,
set `reviewScope.familyIds` and `reviewScope.caseIds` to exactly that sorted scope, and set
`completeForDeclaredScope` only after every retained case has at least one reviewed judgment.

Replace every reviewer placeholder factually:

- `stableProjectId` is a stable project-local identifier, not an invented credential;
- qualification records a category and concrete rationale;
- independence records the relationship to the annotation authors;
- prior exposure names every known case whose expected label was already seen;
- conflicts of interest are declared or explicitly recorded as none declared;
- `reviewedOn` is the actual calendar date.

Do not include personal contact details, customer/private data, credentials, private paths or
URLs, protected-holdout membership or labels, or externally processed material. The tool checks a
bounded set of structural and leakage indicators; the reviewer remains responsible for the truth
and completeness of the declarations.

### Acquisition judgments

An acquisition judgment identifies one `case`, `node`, or named `abstention` subject and a dotted
aspect in the acquisition annotation shape. Record:

- `observed` only for a source/native fact with a reviewed value;
- `cantTell` when the required evidence is missing or conflicting;
- `untested` when the acquisition method or review did not test the aspect.

`cantTell` and `untested` require a null value. Do not estimate unavailable geometry. Put units,
coordinate spaces, and tolerance basis in `unitOrCoordinateSpace`; record native and pixel evidence
separately; and use `nativePixelRelationship: conflict` rather than replacing one source with the
other.

### Rule judgments

A rule judgment identifies an exact rule/version/target and separately records applicability,
required-evidence sufficiency, and outcome. Preserve these outcomes:

- `passed`: applicable, sufficient evidence, expectation satisfied;
- `failed`: applicable, sufficient evidence, expectation not satisfied;
- `cantTell`: applicable or applicability-uncertain with insufficient/conflicting evidence;
- `inapplicable`: the policy does not apply to this target;
- `untested`: the review did not evaluate the policy for this target.

Always state the policy basis, valid alternative or hard-negative reasoning, false-positive and
false-negative risks, confidence, and rationale. A measurement or source-state name alone is not a
rule failure.

## Finalize before comparison

Validate a draft as often as needed while staying source-only:

```bash
python3 tools/prepare_web_review.py \
  --validate-submission REVIEWER-DRAFT.json
```

Then finalize to a different file:

```bash
python3 tools/prepare_web_review.py \
  --finalize-submission REVIEWER-DRAFT.json \
  > REVIEWER-FINAL.json
```

Finalization validates the declared scope and changes only `lifecycle` and `submissionDigest` in
memory. It emits canonical JSON with no trailing newline. Preserve the exact finalized bytes and
digest. If a real answer must change later, keep the prior finalized record and create a new
version; do not silently rewrite history.

Only after those bytes are locked may the reviewer inspect the existing public oracles and run:

```bash
python3 tools/compare_web_review.py \
  --submission REVIEWER-FINAL.json \
  > REVIEW-COMPARISON.json
```

The comparison process first verifies finalization and digest binding, then opens the separately
referenced acquisition and rule oracles. It has no output-path option, writes only to stdout, and
does not modify or adjudicate either side.

## Interpreting comparison output

The report keeps acquisition agreement and rule agreement separate as integer numerator and
denominator cells. Their denominators are all submitted acquisition and rule comparisons,
respectively. `disagreement` uses uniquely comparable rows as its denominator and counts unequal
values. `unresolved` uses every comparison as its denominator and includes every disagreement plus
every key that cannot be uniquely compared. `adjudicated` uses unresolved rows as its denominator
and has a zero numerator in version `1.0.0` because the tool never decides which side is right.
`abstentionAgreement` uses uniquely comparable rows where either side is `cantTell` or `untested`
as its denominator and counts matching abstentions. Rule `inapplicable` remains a rule agreement
rather than being collapsed into abstention. A zero denominator is reported as zero, not divided
or omitted.

Each row retains reviewer value, confidence, rationale, current oracle value, public source, and
oracle rationale. A separately responsible human must investigate disagreements. Do not edit an
expected value merely to increase agreement, and do not force unresolved evidence into pass or
failure.

## Conformance-only record

`evaluation/web/conformance/review/fictional-submission.json` contains an invented reviewer and
invented values designed to exercise clean agreement, one deliberate disagreement, unresolved
keys, `cantTell`, `inapplicable`, `untested`, passing/failing outcomes, and a hard negative. It
declares full public-label exposure and no independence. It must never be copied into a real
submission or cited as independent review, product accuracy, WCAG conformance, protected-holdout
performance, or blocking maturity.
