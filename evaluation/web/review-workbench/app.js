(() => {
  "use strict";

  const token = new URLSearchParams(window.location.hash.slice(1)).get("token");
  window.history.replaceState(null, "", "/");
  const status = document.querySelector("#status");
  let documentState = null;
  let selectedCaseId = null;

  const confirmationLabels = {
    sourceFirstNotBlind: "I reviewed source first and understand this is not a blind review.",
    sightlintOutputNotUsed: "I did not use SightLint output to decide an answer.",
    existingOracleNotViewed: "I did not view the existing oracle before finalization.",
    generatedCaptureNotUsed: "I did not use a generated capture, report, screenshot, or Artifact IR as the answer.",
    implementationOutputNotUsed: "I did not use implementation-authored output as an answer.",
    noPrivateData: "My response contains no protected, personal, or customer data.",
    noCredentials: "My response contains no credentials or private locations.",
    noExternalProcessing: "I did not use hosted or external processing for these answers.",
    cleanLocalBrowserProfile: "I used a local browser profile with extensions and synchronization disabled.",
  };

  function element(selector) {
    const value = document.querySelector(selector);
    if (value === null) throw new Error(`Missing workbench element: ${selector}`);
    return value;
  }

  function setStatus(message, error = false) {
    status.textContent = message;
    status.classList.toggle("error", error);
  }

  async function api(path, method = "GET", body = null) {
    if (token === null || token.length === 0) throw new Error("The local session token is missing.");
    const headers = { "X-SightLint-Review-Token": token };
    if (body !== null) headers["Content-Type"] = "application/json";
    const response = await fetch(path, {
      method,
      headers,
      body: body === null ? null : JSON.stringify(body),
    });
    const payload = await response.json();
    if (!response.ok) {
      const message = payload?.error?.message ?? `Request failed with status ${response.status}`;
      throw new Error(message);
    }
    return payload;
  }

  function setValue(selector, value) {
    element(selector).value = value ?? "";
  }

  function optionalValue(selector) {
    const value = element(selector).value;
    return value === "" ? null : value;
  }

  function currentAnswer() {
    return documentState.state.answers.find((answer) => answer.caseId === selectedCaseId);
  }

  function readDeclaration() {
    const reviewer = documentState.state.reviewer;
    documentState.state.submissionId = element("#submission-id").value;
    reviewer.stableProjectId = element("#reviewer-id").value;
    reviewer.qualificationCategory = optionalValue("#qualification-category");
    reviewer.qualificationRationale = element("#qualification-rationale").value;
    reviewer.independence = optionalValue("#independence");
    reviewer.independenceRationale = element("#independence-rationale").value;
    reviewer.priorExposureStatus = optionalValue("#exposure-status");
    reviewer.priorExposureRationale = element("#exposure-rationale").value;
    reviewer.conflictStatus = optionalValue("#conflict-status");
    reviewer.conflictRationale = element("#conflict-rationale").value;
    reviewer.reviewedOn = element("#reviewed-on").value;
    reviewer.priorExposureCaseIds = [...document.querySelectorAll("[data-exposure-case]:checked")]
      .map((input) => input.value)
      .sort();
    for (const field of Object.keys(confirmationLabels)) {
      documentState.state.confirmations[field] = element(`#confirmation-${field}`).checked;
    }
  }

  function readCurrentCase() {
    if (selectedCaseId === null) return;
    const answer = currentAnswer();
    answer.acquisition.status = optionalValue("#acquisition-status");
    answer.acquisition.observedValueKind = optionalValue("#acquisition-value-kind");
    answer.acquisition.value = element("#acquisition-value").value || null;
    answer.acquisition.confidence = optionalValue("#acquisition-confidence");
    answer.acquisition.rationale = element("#acquisition-rationale").value;
    answer.rule.outcome = optionalValue("#rule-outcome");
    answer.rule.requiredEvidence = optionalValue("#rule-evidence");
    answer.rule.confidence = optionalValue("#rule-confidence");
    answer.rule.rationale = element("#rule-rationale").value;
    normalizeAcquisition(answer.acquisition);
  }

  function normalizeAcquisition(acquisition) {
    if (acquisition.status !== "observed") {
      acquisition.observedValueKind = null;
      acquisition.value = null;
    } else if (acquisition.observedValueKind !== "text") {
      acquisition.value = null;
    }
  }

  function writeDeclaration() {
    const state = documentState.state;
    const reviewer = state.reviewer;
    setValue("#submission-id", state.submissionId);
    setValue("#reviewer-id", reviewer.stableProjectId);
    setValue("#qualification-category", reviewer.qualificationCategory);
    setValue("#qualification-rationale", reviewer.qualificationRationale);
    setValue("#independence", reviewer.independence);
    setValue("#independence-rationale", reviewer.independenceRationale);
    setValue("#exposure-status", reviewer.priorExposureStatus);
    setValue("#exposure-rationale", reviewer.priorExposureRationale);
    setValue("#conflict-status", reviewer.conflictStatus);
    setValue("#conflict-rationale", reviewer.conflictRationale);
    setValue("#reviewed-on", reviewer.reviewedOn);
    const exposed = new Set(reviewer.priorExposureCaseIds);
    for (const input of document.querySelectorAll("[data-exposure-case]")) input.checked = exposed.has(input.value);
    for (const field of Object.keys(confirmationLabels)) {
      element(`#confirmation-${field}`).checked = state.confirmations[field];
    }
  }

  function question(caseId, authority) {
    return documentState.questionnaire.questions.find(
      (candidate) => candidate.caseId === caseId && candidate.authority === authority,
    );
  }

  function writeCurrentCase() {
    const answer = currentAnswer();
    const acquisitionQuestion = question(selectedCaseId, "acquisition");
    const ruleQuestion = question(selectedCaseId, "rule");
    element("#case-heading").textContent = selectedCaseId;
    element("#fixture-frame").src = documentState.fixtureRoutes[selectedCaseId];
    element("#acquisition-prompt").textContent = acquisitionQuestion.prompt;
    element("#rule-prompt").textContent = ruleQuestion.prompt;
    setValue("#acquisition-status", answer.acquisition.status);
    setValue("#acquisition-value-kind", answer.acquisition.observedValueKind);
    setValue("#acquisition-value", answer.acquisition.value);
    setValue("#acquisition-confidence", answer.acquisition.confidence);
    setValue("#acquisition-rationale", answer.acquisition.rationale);
    setValue("#rule-outcome", answer.rule.outcome);
    setValue("#rule-evidence", answer.rule.requiredEvidence);
    setValue("#rule-confidence", answer.rule.confidence);
    setValue("#rule-rationale", answer.rule.rationale);
    updateAcquisitionControls();
    renderProgress();
  }

  function judgmentComplete(answer) {
    const acquisition = answer.acquisition;
    const acquisitionValueComplete = acquisition.status !== "observed"
      || acquisition.observedValueKind === "absent"
      || acquisition.observedValueKind === "text" && typeof acquisition.value === "string" && acquisition.value.length > 0;
    const acquisitionComplete = acquisition.status !== null
      && acquisitionValueComplete
      && acquisition.confidence !== null
      && acquisition.rationale.length > 0;
    const ruleComplete = answer.rule.outcome !== null
      && answer.rule.requiredEvidence !== null
      && answer.rule.confidence !== null
      && answer.rule.rationale.length > 0;
    return [acquisitionComplete, ruleComplete];
  }

  function renderProgress() {
    readDeclaration();
    readCurrentCase();
    const completed = documentState.state.answers
      .flatMap(judgmentComplete)
      .filter(Boolean).length;
    element("#progress-count").textContent = `${completed} / 8`;
    for (const button of document.querySelectorAll("[data-case-id]")) {
      const answer = documentState.state.answers.find((candidate) => candidate.caseId === button.dataset.caseId);
      button.classList.toggle("complete", judgmentComplete(answer).every(Boolean));
    }
  }

  function updateAcquisitionControls() {
    const observed = element("#acquisition-status").value === "observed";
    element("#acquisition-value-kind").disabled = !observed;
    const text = observed && element("#acquisition-value-kind").value === "text";
    element("#acquisition-value").disabled = !text;
    if (!observed) {
      element("#acquisition-value-kind").value = "";
      element("#acquisition-value").value = "";
    } else if (!text) {
      element("#acquisition-value").value = "";
    }
  }

  function buildEditablePayload() {
    readDeclaration();
    readCurrentCase();
    return {
      expectedStateDigest: documentState.state.stateDigest,
      submissionId: documentState.state.submissionId,
      reviewer: documentState.state.reviewer,
      confirmations: documentState.state.confirmations,
      answers: documentState.state.answers,
    };
  }

  async function save(finalize) {
    try {
      setStatus(finalize ? "Validating and finalizing…" : "Saving local draft…");
      const response = await api(finalize ? "/api/finalize" : "/api/save", "POST", buildEditablePayload());
      if (finalize) {
        documentState = response;
        element("#completion").hidden = false;
        element("#completion-summary").textContent =
          `Digest ${response.finalization.submissionDigest}; evidence status ${response.finalization.evidenceStatus}.`;
        element("#comparison-command").textContent = response.finalization.comparisonArgv.join(" ");
        element("#save").disabled = true;
        element("#finalize").disabled = true;
        setStatus("Finalized. The local server has stopped and answers are locked.");
      } else {
        documentState.state = response.state;
        writeDeclaration();
        writeCurrentCase();
        setStatus(`Draft saved at state digest ${response.state.stateDigest}.`);
      }
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    }
  }

  function initializeControls() {
    const exposureCases = element("#exposure-cases");
    const navigation = element("#case-navigation");
    for (const caseId of documentState.questionnaire.scope.caseIds) {
      const label = document.createElement("label");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.value = caseId;
      checkbox.dataset.exposureCase = "true";
      label.append(checkbox, document.createTextNode(caseId));
      exposureCases.append(label);

      const button = document.createElement("button");
      button.type = "button";
      button.dataset.caseId = caseId;
      button.textContent = caseId.replace("support-inbox-", "");
      button.addEventListener("click", () => {
        readCurrentCase();
        selectedCaseId = caseId;
        for (const candidate of navigation.querySelectorAll("button")) {
          candidate.setAttribute("aria-current", candidate === button ? "step" : "false");
        }
        writeCurrentCase();
      });
      navigation.append(button);
    }

    const confirmations = element("#confirmations");
    for (const [field, message] of Object.entries(confirmationLabels)) {
      const label = document.createElement("label");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.id = `confirmation-${field}`;
      label.append(checkbox, document.createTextNode(message));
      confirmations.append(label);
    }

    const sourceSelect = element("#source-select");
    for (const source of documentState.sources) {
      const option = document.createElement("option");
      option.value = source.path;
      option.textContent = source.path;
      sourceSelect.append(option);
    }
    const showSource = () => {
      const source = documentState.sources.find((candidate) => candidate.path === sourceSelect.value);
      element("#source-view").textContent = source?.contentUtf8 ?? "";
    };
    sourceSelect.addEventListener("change", showSource);
    showSource();

    for (const control of document.querySelectorAll("input, select, textarea")) {
      control.addEventListener("input", renderProgress);
      control.addEventListener("change", renderProgress);
    }
    element("#acquisition-status").addEventListener("change", updateAcquisitionControls);
    element("#acquisition-value-kind").addEventListener("change", updateAcquisitionControls);
    element("#save").addEventListener("click", () => save(false));
    element("#finalize").addEventListener("click", () => save(true));
    element("#judgment-form").addEventListener("submit", (event) => event.preventDefault());
    element("#toggle-declaration").addEventListener("click", () => {
      const fields = element("#declaration-fields");
      fields.hidden = !fields.hidden;
      element("#toggle-declaration").setAttribute("aria-expanded", String(!fields.hidden));
      element("#toggle-declaration").textContent = fields.hidden ? "Show declaration" : "Hide declaration";
    });
  }

  async function main() {
    try {
      documentState = await api("/api/state");
      selectedCaseId = documentState.questionnaire.scope.caseIds[0];
      initializeControls();
      writeDeclaration();
      element("[data-case-id]").setAttribute("aria-current", "step");
      writeCurrentCase();
      setStatus(documentState.locked ? "This session is locked." : "Ready. Answers have not been compared.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    }
  }

  main();
})();
