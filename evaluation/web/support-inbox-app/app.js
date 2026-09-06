(() => {
  "use strict";

  const allowedCases = new Set([
    "ambiguous-control",
    "clean",
    "labelledby-hard-negative",
    "unnamed-control-mutant",
  ]);
  const parameters = new URLSearchParams(window.location.search);
  const requestedCase = parameters.get("case") || "clean";
  const fixtureCase = allowedCases.has(requestedCase) ? requestedCase : "clean";
  const sendAction = document.querySelector("[data-testid='reply-send']");

  document.body.dataset.case = fixtureCase;
  if (fixtureCase === "unnamed-control-mutant") {
    sendAction?.removeAttribute("aria-label");
  } else if (fixtureCase === "labelledby-hard-negative") {
    sendAction?.removeAttribute("aria-label");
    sendAction?.setAttribute("aria-labelledby", "reply-send-label");
  } else if (fixtureCase === "ambiguous-control" && sendAction instanceof HTMLButtonElement) {
    const ambiguous = document.createElement("div");
    ambiguous.className = sendAction.className;
    ambiguous.dataset.testid = "reply-send";
    ambiguous.tabIndex = 0;
    ambiguous.innerHTML = '<span aria-hidden="true">➜</span>';
    sendAction.replaceWith(ambiguous);
  }

  document.documentElement.dataset.fixtureReady = "true";
  Object.defineProperty(window, "__SIGHTLINT_FIXTURE__", {
    configurable: false,
    enumerable: true,
    writable: false,
    value: Object.freeze({
      caseId: fixtureCase,
      externalAssets: false,
      externalNetwork: false,
      ready: true,
    }),
  });
})();
