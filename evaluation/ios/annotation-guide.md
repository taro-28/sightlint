# iOS annotation guide 0.1.0

Annotations are authored from repository fixture intent, instrumented UIKit source facts,
independent XCUITest observations, and human screenshot review before comparing adapter output.
Never copy normalized IR or a CheckReport into an oracle merely to make a test pass.

For acquisition annotations:

- record UIKit source allocation separately from window/safe-area intersection and XCUI frame;
- preserve source/XCUI disagreement as `frameConflict`; never average or silently repair it;
- use `mappedExactLayout` only for a window-attached, non-hidden, positive-alpha,
  identity-transform source View with a unique identifier, nonempty allocation, and nonempty window
  intersection;
- use the explicit `notMapped*` reason for hidden, transparent, detached, offscreen, empty, or
  transformed Views;
- never label XCUI frame or `isHittable` as an exact touch region, activation point, or rendered
  ink;
- record missing pixel identity and hit geometry as `cantTell`; SwiftUI, focus navigation,
  occlusion, and dynamic behavior remain `untested`.

For rule annotations, label only `visual.bounds.within-canvas@0.1.0` over admitted exact-source
UIKit `layoutBox` facts. A failure says a source allocation crosses the point canvas; it does not
prove user harm, accessibility failure, touch-target failure, or general UX quality.

Every case records source ownership, dual license, fictional-data privacy, public split,
false-positive risk, abstentions, and absence of a protected holdout. Any stronger maturity,
compatibility, or blocking claim requires new independent evidence and a separate decision.
