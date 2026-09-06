(() => {
  "use strict";

  const states = new Set([
    "uniform-canvas",
    "edge-noise",
    "recolored-canvas",
    "translated-canvas",
    "device-scale-canvas",
    "modal-surface",
    "split-pane-hard-negative",
    "gradient-edge-hard-negative",
    "checkerboard-stress",
  ]);
  const parameters = new URLSearchParams(window.location.search);
  const requested = parameters.get("case") || "uniform-canvas";
  const state = states.has(requested) ? requested : "uniform-canvas";

  document.body.dataset.case = state;
  document.documentElement.dataset.fixtureReady = "true";
  Object.defineProperty(window, "__SIGHTLINT_SEGMENTATION_FIXTURE__", {
    configurable: false,
    enumerable: true,
    writable: false,
    value: Object.freeze({
      state,
      fictionalData: true,
      externalAssets: false,
      ready: true,
    }),
  });
})();
