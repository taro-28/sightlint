import { spawn } from "node:child_process";

import { PerceptionError } from "./errors.mjs";

export function runBoundedProcess(program, args, input, options) {
  return new Promise((resolve, reject) => {
    let child;
    try {
      child = spawn(program, args, { shell: false, stdio: ["pipe", "pipe", "pipe"] });
    } catch {
      reject(new PerceptionError(options.spawnCode, `${options.label} could not be started`));
      return;
    }
    const stdout = [];
    const stderr = [];
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let terminalError = null;
    let settled = false;

    const stop = (error) => {
      if (terminalError === null) terminalError = error;
      child.kill("SIGKILL");
    };
    const timer = setTimeout(() => stop(new PerceptionError(options.timeoutCode, `${options.label} exceeded ${options.timeoutMs} ms`)), options.timeoutMs);

    child.stdout.on("data", (chunk) => {
      stdoutBytes += chunk.byteLength;
      if (stdoutBytes > options.maxStdoutBytes) {
        stop(new PerceptionError(options.stdoutCode, `${options.label} exceeded the stdout byte budget`));
      } else {
        stdout.push(chunk);
      }
    });
    child.stderr.on("data", (chunk) => {
      stderrBytes += chunk.byteLength;
      if (stderrBytes > options.maxStderrBytes) {
        stop(new PerceptionError(options.stderrCode, `${options.label} exceeded the stderr byte budget`));
      } else {
        stderr.push(chunk);
      }
    });
    child.on("error", () => {
      if (!settled) {
        settled = true;
        clearTimeout(timer);
        reject(new PerceptionError(options.spawnCode, `${options.label} could not be started`));
      }
    });
    child.on("close", (code, signal) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (terminalError !== null) {
        reject(terminalError);
      } else if (signal !== null) {
        reject(new PerceptionError(options.exitCode, `${options.label} terminated by signal ${signal}`));
      } else if (code !== 0) {
        reject(new PerceptionError(options.exitCode, `${options.label} exited with code ${code}`));
      } else {
        resolve({ stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr), stdoutBytes, stderrBytes });
      }
    });
    child.stdin.on("error", () => {});
    child.stdin.end(input);
  });
}
