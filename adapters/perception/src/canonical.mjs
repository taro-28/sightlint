import { createHash } from "node:crypto";

function normalize(value) {
  if (typeof value === "number" && Object.is(value, -0)) {
    return 0;
  }
  if (Array.isArray(value)) {
    return value.map(normalize);
  }
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value).sort().map((key) => [key, normalize(value[key])]),
    );
  }
  return value;
}

export function canonicalJson(value) {
  return `${JSON.stringify(normalize(value))}\n`;
}

export function sha256(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}
