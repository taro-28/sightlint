import assert from "node:assert/strict";
import { createConnection, createServer } from "node:net";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { ManagedServer } from "../src/server.js";
import type { ManagedCaptureRequest } from "../src/types.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

function freePort(): Promise<number> {
  return new Promise((resolvePort, reject) => {
    const server = createServer();
    server.unref();
    server.once("error", reject);
    server.listen({ host: "127.0.0.1", port: 0 }, () => {
      const address = server.address();
      assert.ok(address !== null && typeof address === "object");
      server.close((error) => error === undefined ? resolvePort(address.port) : reject(error));
    });
  });
}

function connected(port: number): Promise<boolean> {
  return new Promise((resolveConnected) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    const finish = (value: boolean): void => {
      socket.removeAllListeners();
      socket.destroy();
      resolveConnected(value);
    };
    socket.setTimeout(200, () => finish(false));
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
  });
}

async function waitForConnection(port: number, expected: boolean): Promise<void> {
  const deadline = Date.now() + 10_000;
  do {
    if (await connected(port) === expected) return;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 25));
  } while (Date.now() < deadline);
  assert.fail(`port ${port} did not reach connected=${String(expected)}`);
}

function request(port: number, childPort: number): ManagedCaptureRequest {
  return {
    $schema: "../../../adapters/playwright/schemas/capture-request-0.2.schema.json",
    protocolVersion: "0.2.0",
    artifact: { id: "managed-server-tree", title: "Managed server tree" },
    target: {
      kind: "managedLoopbackHttp",
      pathAndQuery: "/",
      state: "server-tree",
      readinessSelector: "main",
    },
    server: {
      argv: [
        process.execPath,
        "adapters/playwright/tests/fixtures/managed-server.mjs",
        "--mode", "serve",
        "--port", "{port}",
        "--child-port", String(childPort),
      ],
      port,
      startupTimeoutMs: 10_000,
    },
    environment: {
      viewport: { width: 1280, height: 800, unit: "cssPixel" },
      deviceScaleFactor: 1,
      textScale: 1,
      locale: "en-US",
      timezoneId: "UTC",
      colorScheme: "light",
      reducedMotion: "reduce",
    },
    privacy: { accessibleNameMode: "selectedNodes", externalProcessing: false },
    network: { mode: "sameOriginLoopback" },
    screenshot: { reference: "evaluation/web/generated/server-tree.png" },
  };
}

test("managed server stop removes its process tree and releases both ports", { timeout: 60_000 }, async () => {
  const port = await freePort();
  const childPort = await freePort();
  const managed = new ManagedServer(request(port, childPort), repositoryRoot);
  await managed.start();
  try {
    await waitForConnection(port, true);
    await waitForConnection(childPort, true);
    assert.equal(managed.record.commandAuthority, "explicitCliFlag");
    assert.equal(managed.record.childNetwork, "notControlled");
    assert.match(managed.record.commandDigest, /^sha256:[0-9a-f]{64}$/u);
  } finally {
    await managed.stop();
  }
  await waitForConnection(port, false);
  await waitForConnection(childPort, false);
});
