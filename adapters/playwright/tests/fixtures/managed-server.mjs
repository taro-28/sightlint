import { spawn } from "node:child_process";
import { createReadStream } from "node:fs";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

function argumentsMap(values) {
  const result = new Map();
  for (let index = 0; index < values.length; index += 2) {
    const name = values[index];
    const value = values[index + 1];
    if (name === undefined || value === undefined) throw new Error("expected fixture flag/value pairs");
    result.set(name, value);
  }
  return result;
}

const args = argumentsMap(process.argv.slice(2));
const mode = args.get("--mode") ?? "serve";
const port = Number(args.get("--port"));
const childPort = args.has("--child-port") ? Number(args.get("--child-port")) : null;
const delayMs = Number(args.get("--delay-ms") ?? "0");

if (mode === "early-exit") process.exit(23);
if (mode === "timeout") {
  setInterval(() => {}, 1_000);
} else if (mode === "log-overflow") {
  process.stdout.write(Buffer.alloc(1024 * 1024 + 1, 0x78));
  setInterval(() => {}, 1_000);
} else {
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("invalid fixture port");
  if (mode === "serve" && childPort !== null) {
    spawn(process.execPath, [fileURLToPath(import.meta.url), "--mode", "child", "--port", String(childPort)], {
      detached: false,
      stdio: "ignore",
    });
  }

  const fixtureRoot = resolve(process.cwd(), "evaluation/web/fixture-app");
  const contentTypes = new Map([
    [".css", "text/css; charset=utf-8"],
    [".html", "text/html; charset=utf-8"],
    [".js", "text/javascript; charset=utf-8"],
  ]);
  const server = createServer(async (request, response) => {
    const target = new URL(request.url ?? "/", `http://127.0.0.1:${port}`);
    if (mode === "child") {
      response.writeHead(204).end();
      return;
    }
    if (target.pathname === "/redirect") {
      response.writeHead(302, { location: `/index.html${target.search}` }).end();
      return;
    }
    if (target.pathname === "/api/echo") {
      const chunks = [];
      for await (const chunk of request) chunks.push(chunk);
      response.writeHead(200, { "content-type": "application/octet-stream" });
      response.end(Buffer.concat(chunks));
      return;
    }
    if (target.pathname === "/api/large") {
      response.writeHead(200, { "content-type": "application/octet-stream" });
      response.end(Buffer.alloc(16 * 1024 * 1024 + 1, 0x61));
      return;
    }
    if (target.pathname === "/api/chunk") {
      response.writeHead(200, { "content-type": "application/octet-stream" });
      response.end(Buffer.alloc(8 * 1024 * 1024, 0x62));
      return;
    }
    if (target.pathname === "/api/empty") {
      response.writeHead(204).end();
      return;
    }
    if (target.pathname === "/sw.js") {
      response.writeHead(200, { "content-type": "text/javascript" }).end("self.addEventListener('fetch', () => {});\n");
      return;
    }
    const relativePath = target.pathname === "/" ? "index.html" : target.pathname.slice(1);
    if (!["index.html", "styles.css", "app.js"].includes(relativePath)) {
      response.writeHead(404).end("not found\n");
      return;
    }
    const path = resolve(fixtureRoot, relativePath);
    if (relativePath === "index.html") {
      let html = await readFile(path, "utf8");
      const networkCase = target.searchParams.get("networkCase") ?? "same-origin";
      const operations = [
        "fetch('/api/echo?private=not-serialized', { method: 'POST', body: 'bounded request body' })",
      ];
      if (networkCase === "external") operations.push("fetch('https://example.invalid/private?secret=yes')");
      if (networkCase === "blocked-transports") {
        operations.push("new Promise((resolve) => { const socket = new WebSocket('ws://127.0.0.1:" + port + "/socket?secret=yes'); socket.addEventListener('error', resolve); setTimeout(resolve, 100); })");
        operations.push("navigator.serviceWorker.register('/sw.js?secret=yes')");
      }
      if (networkCase === "request-large") operations.push("fetch('/api/echo', { method: 'POST', body: 'x'.repeat(1024 * 1024 + 1) })");
      if (networkCase === "response-large") operations.push("fetch('/api/large')");
      if (networkCase === "response-aggregate") operations.push("(async () => { for (let index = 0; index < 9; index += 1) await fetch('/api/chunk?index=' + index); })()");
      if (networkCase === "response-count") operations.push("(async () => { for (let index = 0; index < 513; index += 1) await fetch('/api/empty?index=' + index); })()");
      const injected = `<script>Promise.allSettled([${operations.join(",")}]).then(() => { document.documentElement.dataset.managedReady = "true"; });</script>`;
      html = html.replace("</body>", `${injected}</body>`);
      const body = Buffer.from(html);
      response.writeHead(200, { "content-type": "text/html; charset=utf-8", "content-length": body.byteLength });
      response.end(body);
      return;
    }
    response.writeHead(200, { "content-type": contentTypes.get(extname(path)) ?? "application/octet-stream" });
    createReadStream(path).pipe(response);
  });
  setTimeout(() => server.listen(port, "127.0.0.1"), delayMs);
}
