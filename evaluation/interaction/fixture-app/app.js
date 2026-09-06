(() => {
  "use strict";

  const scenario = new URLSearchParams(window.location.search).get("case") || "slow-success-clean";
  const state = document.querySelector("[data-testid=interaction-state]");
  const save = document.querySelector("[data-testid=save-settings]");
  const retry = document.querySelector("[data-sightlint-recovery=retry]");
  const saveDraft = document.querySelector("[data-sightlint-recovery=saveDraft]");
  const events = [];

  if (!(state instanceof HTMLElement) || !(save instanceof HTMLButtonElement) ||
      !(retry instanceof HTMLButtonElement) || !(saveDraft instanceof HTMLButtonElement)) {
    throw new Error("fixture structure is incomplete");
  }

  function declareState(value) {
    events.push({ kind: "stateChanged", state: value });
  }

  function showState(value) {
    state.dataset.sightlintState = value;
    state.textContent = {
      idle: "No unsaved changes",
      pending: "Saving settings…",
      optimistic: "Settings queued",
      success: "Settings saved",
      failure: "Settings could not be saved",
    }[value];
    declareState(value);
  }

  function hideRecoveries() {
    retry.hidden = true;
    saveDraft.hidden = true;
  }

  function beginPrimary() {
    hideRecoveries();
    if (scenario === "immediate-success") {
      events.push({ kind: "effectResolved", resolution: "success" });
      showState("success");
    } else if (scenario === "missing-pending-mutant") {
      // Targeted mutation: the latent effect starts without user-visible feedback.
    } else if (scenario === "pending-conflict") {
      // Deliberate source conflict: instrumentation claims pending while native DOM stays idle.
      declareState("pending");
    } else {
      showState("pending");
    }
  }

  function beginRecovery() {
    hideRecoveries();
    showState("pending");
  }

  save.addEventListener("click", beginPrimary);
  retry.addEventListener("click", beginRecovery);
  saveDraft.addEventListener("click", beginRecovery);

  window.__sightlintInteraction = Object.freeze({
    metadata: Object.freeze({ actionId: "save-settings", targetTestId: "save-settings" }),
    drainEvents() {
      return events.splice(0, events.length);
    },
    control(command) {
      if (command === "resolveSuccess") {
        events.push({ kind: "effectResolved", resolution: "success" });
        showState("success");
        hideRecoveries();
        return;
      }
      if (command === "reject") {
        events.push({ kind: "effectResolved", resolution: "failure" });
        showState("failure");
        hideRecoveries();
        if (scenario === "failure-retry-clean") retry.hidden = false;
        if (scenario === "failure-save-draft-alternative") saveDraft.hidden = false;
        return;
      }
      throw new Error(`unsupported control ${command}`);
    },
  });
})();
