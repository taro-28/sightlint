# Native image inspection fixtures

`corpus.json` stores exact PNG input bytes as hexadecimal, or references immutable case IDs
in `../png-raster/corpus.json`. Expected `bounds` are half-open `[x,y,width,height]` device-pixel
rectangles. Expected groups independently declare axis, gap sequence, and measured pattern.
They do not label a semantic UX pass/failure.

Run:

```sh
python3 tools/generate_inspection_corpus.py --check
cargo test --locked -p sightlint-cli --test image_inspection_e2e -- --nocapture
```

To materialize a case for manual inspection from the repository root:

```python
import json
from pathlib import Path

cases = json.loads(Path('fixtures/image-inspection/corpus.json').read_text())['cases']
case = next(item for item in cases if item['id'] == 'cards-mutated')
if 'rasterCase' in case:
    rasters = json.loads(Path('fixtures/png-raster/corpus.json').read_text())['cases']
    case = next(item for item in rasters if item['id'] == case['rasterCase'])
# Exclusive creation avoids accidentally overwriting another image.
with Path('inspection-example.png').open('xb') as output:
    output.write(bytes.fromhex(case['pngHex']))
```

Then run `cargo run -p sightlint-cli -- inspect-image inspection-example.png`.
This renders no new artwork and sends nothing to a hosted model. The example is a tiny
synthetic raster, not a real application screenshot. See
[the product evaluation contract](../../evaluation/image-inspection.md).
