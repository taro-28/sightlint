# Android fixture applications

Android fixtures are repository-owned, fictional applications used to acquire independently
reviewable native-structure and rendered evidence. They are not copied from customer or production
applications.

`atlas-app/` is the first bounded classic-View fixture for ADR 0045. Its instrumentation runner
captures View layout facts, separate accessibility facts, device configuration, and a paired
screenshot from three static states. Generated APKs, SDKs, emulator images, signing material, and
temporary capture directories are not committed.

See `atlas-app/README.md` for the pinned capture procedure and limitations.
