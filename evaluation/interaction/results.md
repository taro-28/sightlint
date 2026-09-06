# Interaction evaluation result

The ADR 0047 public regression gate currently records:

| Metric | Result | Required |
|---|---:|---:|
| Acquisition fact coverage | 35 / 35 (1.00) | 1.00 |
| Evaluated case coverage | 8 / 8 (1.00) | 1.00 |
| Failure precision | 2 / 2 (1.00) | 1.00 |
| False-positive rate on reviewed clean/hard-negative cases | 0 / 3 (0.00) | 0.00 maximum |
| Abstention retention | 8 / 8 (1.00) | 1.00 |
| Mutation kill rate | 2 / 2 (1.00) | 1.00 |

These values are recomputed as assertions by `interaction-e2e.test.ts`; this file is a readable
record, not an oracle. The denominator consists entirely of public, maintainer-authored Atlas
fixture states. It is not a holdout, prevalence estimate, cross-browser result, or representative
Web interaction/UI/UX accuracy measurement.
