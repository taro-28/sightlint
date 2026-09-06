import { canonicalJson } from "../../../adapters/perception/src/canonical.mjs";

const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
const request = JSON.parse(Buffer.concat(chunks).toString("utf8"));
process.stdout.write(canonicalJson({
  $schema: "../schemas/response.schema.json",
  protocolVersion: "0.1.0",
  requestId: request.requestId,
  status: "unsupported",
  worker: { name: "wrong-worker", version: "0.1.0", runtime: { name: "node", version: process.versions.node }, backend: "cpu", model: { status: "notApplicable" } },
  inputSha256: request.input.sha256,
  familyStatus: [
    { family: "hierarchy", status: "untested", reason: "not implemented" },
    { family: "peerGroup", status: "untested", reason: "not implemented" },
    { family: "region", status: "unsupported", reason: "not implemented" },
    { family: "role", status: "untested", reason: "not implemented" },
    { family: "text", status: "unsupported", reason: "not implemented" }
  ],
  observations: [],
  repeatedRunAgreement: { status: "notMeasured", reason: "one run" },
  limitations: ["identity mismatch fixture"]
}));
