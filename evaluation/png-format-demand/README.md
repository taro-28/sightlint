# PNG format-demand assessment

This directory records the ADR 0041 decision gate for optional PNG breadth. It is not a decoder
conformance corpus or a user-prevalence study.

`assessment.json` inventories every committed PNG, links the ephemeral Playwright segmentation
screenshots exercised by the public image command, and keeps synthetic unsupported cases separate
from product-demand evidence. The schema and `tools/check_png_format_demand.py` prevent a new
repository PNG or dependency from bypassing review.

All current product inputs are repository-owned synthetic data under `MIT OR Apache-2.0`. No
customer telemetry or artifact content is collected, and no protected holdout or representative
sampling is claimed. A missing observed format gap does not mean users never have unsupported
PNGs; it means broader decoder work is not admitted by current evidence.

Run:

```bash
python3 tools/check_png_format_demand.py
```
