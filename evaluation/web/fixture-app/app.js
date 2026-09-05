(() => {
  "use strict";

  const allowedCases = new Set([
    "peer-spacing-clean",
    "peer-spacing-mutant",
    "out-of-viewport-mutant",
    "intentional-grouping",
    "ambiguous-peer-group",
    "clipping-mutant",
    "control-boundaries",
    "control-offscreen-mutant",
    "intentional-overlay",
    "notification-clean",
    "occlusion-clean",
    "occlusion-mutant",
    "overflow-mutant",
    "peer-dimension-mutant",
    "responsive-mobile-mutant",
    "rtl-vertical",
    "transformed-text-mutant",
  ]);
  const parameters = new URLSearchParams(window.location.search);
  const requestedCase = parameters.get("case") || "peer-spacing-clean";
  const fixtureCase = allowedCases.has(requestedCase) ? requestedCase : "peer-spacing-clean";
  const requestedTextScale = Number(parameters.get("textScale") || "1");
  const textScale = requestedTextScale === 1.25 ? 1.25 : 1;

  document.body.dataset.case = fixtureCase;
  document.documentElement.dir = fixtureCase === "rtl-vertical" ? "rtl" : "ltr";
  document.documentElement.style.fontSize = `${16 * textScale}px`;
  document.documentElement.dataset.fixtureReady = "true";

  Object.defineProperty(window, "__SIGHTLINT_FIXTURE__", {
    configurable: false,
    enumerable: true,
    writable: false,
    value: Object.freeze({
      caseId: fixtureCase,
      textScale,
      externalAssets: false,
      ready: true,
    }),
  });
})();
