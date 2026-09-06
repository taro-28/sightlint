# iOS fixture applications

iOS fixtures are repository-owned fictional applications used to acquire independently reviewable
native-structure, accessibility, and rendered evidence. They are not copied from customer or
production applications.

`atlas-app/` is the first bounded UIKit fixture for ADR 0046. Its application instrumentation
captures source layout facts, while its XCUITest target captures a pre-query screenshot and then
independent accessibility facts from three static states. Generated apps, DerivedData, result
bundles, simulator runtimes, signing material, and temporary capture directories are not committed.

See `atlas-app/README.md` for the pinned capture procedure and limitations.
