# ADR 0048 — Managed loopback Web capture and server lifecycle

- Status: Accepted
- Date: 2026-09-06
- Issue: #62
- Parent: #31
- Roadmap: M7
- Owners: @taro-28

## Context

ADRs 0033–0036 provide a process-isolated Playwright capture and one-command Rust-kernel check,
but capture protocol `0.1.0` accepts only repository-contained static `file:` documents. A real
dogfood attempt against tabisaifu cannot reach its server-rendered authenticated entry form from
that boundary. Starting the application separately would leave startup, readiness, cleanup,
network policy, and command authority outside the repeatable SightLint invocation.

The requested product claim is deliberately narrower than arbitrary local-URL inspection. One
explicitly authorized invocation starts one caller-specified repository process without a shell,
captures one page from literal IPv4 loopback, invokes the unchanged deterministic rules, and
stops the complete managed process tree. This introduces command-execution, lifecycle, network,
resource, protocol, extension, and report compatibility decisions and therefore requires an ADR.

## Decision

### Versioned public surfaces

Capture request and response `0.2.0` add the managed target. Existing `0.1.0` requests continue to
use the repository-contained `file:` path and emit the same `0.1.0` response,
`org.sightlint.web@0.3.0`, and workflow report `0.1.0` bytes.

The managed path emits:

- capture request and response `0.2.0`;
- adapter implementation `0.4.0` in private package `0.5.0`;
- `org.sightlint.web@0.4.0`;
- Web workflow report `0.2.0`.

Artifact IR stays at `0.1.0`, CheckReport stays at `0.3.0`, and the three Web rule identifiers,
versions, applicability, policy, maturity, outcomes, and advisory enforcement do not change. The
Rust engine dispatches recognized Web extensions `0.3.0` and `0.4.0` explicitly and applies the
same semantic validation and rules after validating the version-specific acquisition envelope.

### Explicit command authority

A `0.2.0` request has exactly these new structures:

- `target.kind` is `managedLoopbackHttp`, with `pathAndQuery`, a stable `state`, and a
  `readinessSelector`;
- `server` supplies `argv`, `port`, and `startupTimeoutMs`;
- `network.mode` is `sameOriginLoopback`.

Both `sightlint-web` and `sightlint-web-check` require the bare CLI flag
`--allow-server-command` for this request kind. Without it, they fail before starting a process
with exit 2, empty stdout, and a stable diagnostic. The flag does not affect `0.1.0` requests.

`server.argv` is a nonempty array passed directly to the operating system with `shell: false`. It
contains `{port}` exactly once across all elements; SightLint replaces that token with the
validated decimal port. The process working directory is the canonical target repository root.
The process inherits the caller environment, but no environment value, command text, server log,
or PID is serialized. Command execution is an explicit capability, not an inferred permission.

The port is an integer from 1024 through 65535. Startup timeout is an integer from 1 through
180,000 milliseconds. Argv is limited to 64 nonempty elements and 8 KiB total UTF-8 bytes.
The preflight rejects an occupied port before spawning.

### Lifecycle

SightLint drains both server standard streams while counting a combined maximum of 1 MiB. It
waits for the child to listen on `127.0.0.1:<port>` rather than sleeping for a fixed interval.
Early exit, spawn failure, startup timeout, output overflow, and port conflict are distinct stable
operational errors.

The server is stopped after successful capture and on browser, kernel, output, and signal paths.
On POSIX, the child starts as a process-group leader; cleanup sends `SIGTERM` to the group, waits
up to five seconds, then sends `SIGKILL`. On Windows, cleanup verifies the captured positive PID
and runs `taskkill.exe /PID <pid> /T /F` without a shell. Cleanup confirms that the port becomes
free. Server teardown is complete before the command returns, so a later Rust-kernel failure
cannot orphan the server.

This is bounded lifecycle management, not an operating-system sandbox. The launched command can
read or write with caller privileges and can initiate its own external traffic. SightLint controls
only the browser context's traffic.

### Navigation and browser network boundary

`pathAndQuery` starts with one `/`, contains no fragment, literal control character, encoded
control byte, authority, or alternate origin, and is at most 2 KiB. The only top-level origin is
`http://127.0.0.1:<port>`. `localhost`, IPv6, HTTPS, LAN, file, and remote URLs are rejected.
Locale may be `en-US` or `ja-JP`; the existing viewport, DPR, text-scale, timezone, theme,
reduced-motion, privacy, and screenshot fields retain their meaning.

After TCP readiness, navigation must finish at the same origin with a final HTTP 2xx response,
page `load`, the declared readiness selector attached, and `document.fonts.ready`. The browser
may use any HTTP method against the same origin. Every external HTTP(S) request is aborted and
causes capture failure. WebSocket connections are closed before connection, and service-worker
registration is blocked; both attempted categories are counted.

Browser request bodies are limited to 1 MiB. A response is buffered up to 16 MiB. One capture is
limited to 512 loopback responses and 64 MiB of response bytes. The source digest covers a
canonical ordering of method, a SHA-256 target identity that includes but does not disclose the
query, request-body digest, status, and the exact buffered response bytes. It does not serialize
raw queries, request or response bodies, headers, cookies, or server output.

### Attribution and evidence

Managed capture cannot prove which repository file caused an HTTP response. Web extension
`0.4.0` therefore records `sourceFiles: []`, `sourceKind: loopbackResponses`, and a
`loopbackResponses` summary containing the aggregate digest, request count, total response bytes,
query-free route path, and redacted target digest. The document source path is the query-free
route path. Evidence remains exact browser/source, render, or platform-semantic evidence about
the captured response state; it is not exact source-code attribution.

Workflow report `0.2.0` retains each exact DOM locator but marks
`sourceAttribution: unavailable` and `sourceFiles: []`. The locator remains a navigation hint to
the runtime node, not a file or line claim. Existing `0.1.0` workflow targets retain their source
bundle and `navigationHintNotExactSourceLine` attribution.

## Evaluation

A repository-owned dependency-free Node server fixture covers delayed startup, redirects,
same-origin APIs and request bodies, an external-request mutant, WebSocket and service-worker
attempt recording, early exit, timeout, log overflow, and port conflict. Lifecycle tests verify
that the process tree and port are absent after success, failure, `SIGINT`, and `SIGTERM` on the
supported CI operating systems.

Public-process E2E serves the existing Atlas fixture and invokes the real
`sightlint-web-check` plus built Rust binary. Independently authored expectations cover a clean
case, the existing unnamed-control mutation, and the intentional-dialog-overlay `cantTell` hard
negative. Human and JSON output, old/new schemas, authorization, cleanup, and deterministic
kernel output for identical Artifact IR are checked. Browser/report bytes are only compared when
the declared fixture state and compatibility environment are deterministic.

The external dogfood runs tabisaifu's
`/test/login?next=%2Fentries%2Fnew` with its explicit test environment supplied as Wrangler
arguments. It verifies redirect completion, final 2xx, the 「支払い追加」 form, valid human/JSON
reports, cleanup, port release, and unchanged tracked target-repository state. Because the app
generates CSRF values, UUIDs, and current time, bytes across separate server invocations are not a
determinism contract. Each run must internally bind captured bytes to its digest, and the Rust
kernel must remain deterministic for one fixed Artifact IR.

## Consequences

- A coding agent can dogfood one real local Web route with a single bounded command.
- Command authority and the child's unsandboxed privileges are visible and opt-in.
- Loopback response identity is evidence-backed without copying secrets or claiming source files.
- The compatibility surface grows, while old static-fixture behavior remains stable.
- The managed process and browser gates add cross-platform lifecycle work and execution time.

## Alternatives considered

### Require users to start the server separately

Rejected for this slice because it cannot prove server readiness, ownership, cleanup, or the
exact process configuration used by the capture.

### Accept a shell command string

Rejected because quoting and expansion are platform-dependent and create avoidable injection and
secret-disclosure risks. Direct argv is the public contract.

### Accept arbitrary localhost or remote URLs

Rejected. Existing credentials, proxies, DNS, HTTPS state, and remote content require a broader
threat, privacy, and reproducibility decision. Literal same-origin IPv4 loopback is sufficient for
the first dogfood.

### Infer source files from routes, stack traces, or framework manifests

Rejected because those are framework-dependent hypotheses. The report must say attribution is
unavailable until a separate evidence-backed source-map protocol exists.

### Move server launch or Playwright into Rust

Rejected because the untrusted application/browser lifecycle belongs in the Node adapter. The
Rust kernel continues to receive only validated Artifact IR and own verdicts.

## Non-goals

- multiple pages/routes, interaction actions, server reuse, or project/directory scanning;
- `localhost`, IPv6, HTTPS, LAN, remote targets, proxying, or child-process network sandboxing;
- source file/line inference, automatic source edits, or retained server logs;
- new rules, changed policy, rule promotion, or blocking-maturity changes;
- representative UI/UX accuracy, WCAG conformance, or whole-application assessment.
