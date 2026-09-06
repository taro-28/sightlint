import { spawn, type ChildProcess } from "node:child_process";
import { createConnection, createServer } from "node:net";

import { canonicalJson, sha256 } from "./canonical.js";
import { AdapterError, LIMITS, type JsonValue, type ManagedCaptureRequest } from "./types.js";

const LOOPBACK_HOST = "127.0.0.1";

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
}

async function settlesBefore(promise: Promise<void>, timeoutMs: number): Promise<boolean> {
  let timeout: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise.then(() => true),
      new Promise<boolean>((resolveTimeout) => {
        timeout = setTimeout(() => resolveTimeout(false), timeoutMs);
      }),
    ]);
  } finally {
    clearTimeout(timeout);
  }
}

function canConnect(port: number): Promise<boolean> {
  return new Promise((resolveConnection) => {
    const socket = createConnection({ host: LOOPBACK_HOST, port });
    const finish = (connected: boolean): void => {
      socket.removeAllListeners();
      socket.destroy();
      resolveConnection(connected);
    };
    socket.setTimeout(250, () => finish(false));
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
  });
}

async function assertPortAvailable(port: number): Promise<void> {
  await new Promise<void>((resolveAvailable, reject) => {
    const probe = createServer();
    probe.unref();
    probe.once("error", () => reject(new AdapterError("server-port-conflict", `port ${port} is already in use`)));
    probe.listen({ host: LOOPBACK_HOST, port, exclusive: true }, () => {
      probe.close((error) => {
        if (error === undefined) resolveAvailable();
        else reject(new AdapterError("server-port-check", "failed to complete the port availability check"));
      });
    });
  });
}

async function waitForPortState(port: number, connected: boolean, timeoutMs: number): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  do {
    if (await canConnect(port) === connected) return true;
    await delay(25);
  } while (Date.now() < deadline);
  return false;
}

async function forceKillWindows(pid: number): Promise<void> {
  if (!Number.isSafeInteger(pid) || pid <= 0) {
    throw new AdapterError("server-cleanup", "managed server PID is invalid");
  }
  await new Promise<void>((resolveKill, reject) => {
    const killer = spawn("taskkill.exe", ["/PID", String(pid), "/T", "/F"], {
      shell: false,
      windowsHide: true,
      stdio: "ignore",
    });
    killer.once("error", () => reject(new AdapterError("server-cleanup", "failed to start taskkill.exe")));
    killer.once("close", () => resolveKill());
  });
}

export interface ManagedServerRecord {
  commandDigest: string;
  port: number;
  startupTimeoutMs: number;
  commandAuthority: "explicitCliFlag";
  childNetwork: "notControlled";
}

export class ManagedServer {
  readonly #request: ManagedCaptureRequest;
  readonly #root: string;
  readonly #argv: string[];
  readonly #record: ManagedServerRecord;
  #child: ChildProcess | null = null;
  #stopping = false;
  #stopped = false;
  #resolveFailure: ((error: AdapterError) => void) | null = null;
  readonly #failure: Promise<AdapterError>;
  #exit: Promise<void> | null = null;
  #signalInProgress = false;

  public constructor(request: ManagedCaptureRequest, canonicalRepositoryRoot: string) {
    this.#request = request;
    this.#root = canonicalRepositoryRoot;
    this.#argv = request.server.argv.map((value) => value.replace("{port}", String(request.server.port)));
    this.#record = {
      commandDigest: sha256(canonicalJson(this.#argv as unknown as JsonValue)),
      port: request.server.port,
      startupTimeoutMs: request.server.startupTimeoutMs,
      commandAuthority: "explicitCliFlag",
      childNetwork: "notControlled",
    };
    this.#failure = new Promise((resolveFailure) => {
      this.#resolveFailure = resolveFailure;
    });
  }

  public get record(): ManagedServerRecord {
    return this.#record;
  }

  public async start(): Promise<void> {
    await assertPortAvailable(this.#request.server.port);
    const [program, ...arguments_] = this.#argv;
    if (program === undefined) {
      throw new AdapterError("invalid-server-command", "managed server command is empty");
    }
    const child = spawn(program, arguments_, {
      cwd: this.#root,
      env: process.env,
      shell: false,
      detached: process.platform !== "win32",
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    this.#child = child;
    this.#exit = new Promise((resolveExit) => {
      child.once("exit", () => resolveExit());
      child.once("error", () => resolveExit());
    });
    let outputBytes = 0;
    const drain = (chunk: Buffer): void => {
      outputBytes += chunk.byteLength;
      if (outputBytes > LIMITS.maxServerOutputBytes) {
        this.#fail(new AdapterError("server-output-too-large", "managed server output exceeded 1048576 bytes"));
      }
    };
    child.stdout?.on("data", drain);
    child.stderr?.on("data", drain);
    child.once("error", () => {
      this.#fail(new AdapterError("server-spawn", "failed to start the managed server command"));
    });
    child.once("exit", (code, signal) => {
      if (!this.#stopping) {
        const detail = signal === null ? `exit code ${String(code)}` : `signal ${signal}`;
        this.#fail(new AdapterError("server-early-exit", `managed server exited before cleanup (${detail})`));
      }
    });
    process.once("SIGINT", this.#onSigint);
    process.once("SIGTERM", this.#onSigterm);

    const listening = (async (): Promise<void> => {
      const ready = await waitForPortState(
        this.#request.server.port,
        true,
        this.#request.server.startupTimeoutMs,
      );
      if (!ready) {
        throw new AdapterError(
          "server-startup-timeout",
          `managed server did not listen within ${this.#request.server.startupTimeoutMs} ms`,
        );
      }
    })();
    try {
      await this.guard(listening);
    } catch (error) {
      await this.stop();
      throw error;
    }
  }

  public async guard<T>(operation: Promise<T>): Promise<T> {
    return Promise.race([
      operation,
      this.#failure.then((error) => Promise.reject(error)),
    ]);
  }

  public async stop(): Promise<void> {
    if (this.#stopped) return;
    this.#stopping = true;
    process.removeListener("SIGINT", this.#onSigint);
    process.removeListener("SIGTERM", this.#onSigterm);
    const child = this.#child;
    if (child !== null && child.exitCode === null && child.signalCode === null) {
      if (process.platform === "win32") {
        await forceKillWindows(child.pid ?? -1);
      } else if (child.pid !== undefined && child.pid > 0) {
        try {
          process.kill(-child.pid, "SIGTERM");
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "ESRCH") {
            throw new AdapterError("server-cleanup", "failed to terminate the managed server process group");
          }
        }
        const exited = await settlesBefore(this.#exit ?? Promise.resolve(), LIMITS.serverShutdownTimeoutMs);
        if (!exited) {
          try {
            process.kill(-child.pid, "SIGKILL");
          } catch (error) {
            if ((error as NodeJS.ErrnoException).code !== "ESRCH") {
              throw new AdapterError("server-cleanup", "failed to kill the managed server process group");
            }
          }
        }
      }
    }
    if (this.#exit !== null) {
      await settlesBefore(this.#exit, LIMITS.serverShutdownTimeoutMs);
    }
    const released = await waitForPortState(
      this.#request.server.port,
      false,
      LIMITS.serverShutdownTimeoutMs,
    );
    this.#stopped = true;
    if (!released) {
      throw new AdapterError("server-cleanup", `managed server did not release port ${this.#request.server.port}`);
    }
  }

  #fail(error: AdapterError): void {
    if (!this.#stopping && this.#resolveFailure !== null) {
      const resolveFailure = this.#resolveFailure;
      this.#resolveFailure = null;
      resolveFailure(error);
    }
  }

  readonly #onSigint = (): void => {
    this.#handleSignal(130);
  };

  readonly #onSigterm = (): void => {
    this.#handleSignal(143);
  };

  #handleSignal(exitCode: number): void {
    if (this.#signalInProgress) return;
    this.#signalInProgress = true;
    void this.stop().finally(() => process.exit(exitCode));
  }
}
