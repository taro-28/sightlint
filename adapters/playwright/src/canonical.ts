import { createHash } from "node:crypto";

import type { JsonValue } from "./types.js";

function ordered(value: JsonValue): JsonValue {
  if (Array.isArray(value)) {
    return value.map(ordered);
  }
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
        .map(([key, child]) => [key, ordered(child)]),
    );
  }
  return Object.is(value, -0) ? 0 : value;
}

export function canonicalJson(value: JsonValue): string {
  return `${JSON.stringify(ordered(value))}\n`;
}

export function sha256(bytes: string | Uint8Array): string {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}
