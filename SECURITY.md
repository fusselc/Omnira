# Security Policy

## Core Security Principles

Omnira is a local-first, privacy-preserving desktop app. For MVP (Windows local
GGUF chat), the security posture rests on these rules:

- **No required cloud dependency:** Essential functionality runs on your machine.
- **No telemetry by default:** We do not track user activity, prompts, or usage.
- **No default external network calls:** Model inference and app data stay offline
  unless a future, explicitly user-enabled feature requires otherwise.
- **Local data retention:** Conversations, settings, logs, and model references
  stay under `%LOCALAPPDATA%\Omnira\` (see `docs/data-ownership-and-storage.md`).
- **Process and storage boundaries:** Tauri IPC is typed and minimal; `llama-server`
  is loopback-only and protected by a per-session API key; the webview uses a
  strict CSP. Details live in `docs/local-security-boundary.md`.

---

## Supported Versions

During the MVP / development phase, only the latest code on `main` and the most
recent stable release receive active security attention.

| Version | Supported |
| :--- | :--- |
| `main` (latest) | Yes |
| Older than the latest release | No |

---

## Reporting a Vulnerability

**Do not report security vulnerabilities through public GitHub issues,
discussions, or pull requests.**

Report privately using **GitHub Private Vulnerability Reporting** (preferred):

1. Open the repository [Security advisories](https://github.com/fusselc/Omnira/security/advisories/new) page.
2. Choose **Report a vulnerability**.
3. Include the details below.

Private vulnerability reporting must be enabled in the repository Security
settings for that form to be available. If the form is unavailable, open a
private contact with the repository owner via their GitHub profile and mark the
message as a security report.

### What to include

- A concise description of the vulnerability and its potential impact.
- Clear reproduction steps, or a minimal proof of concept.
- Application version or commit hash, and OS environment.
- Relevant logs, terminal output, or screenshots (prompt content redacted
  unless necessary to demonstrate the issue).

---

## Response Timeline

- **Acknowledgement:** within 5 business days of receiving the report.
- **Initial assessment:** within 10 business days to confirm and classify severity.
- **Remediation:** critical issues targeted within 30 days; lower-severity issues
  in the next suitable release.

These are good-faith targets for a small project; complex issues may take longer.

---

## Scope

### In scope

- **Local runtime boundary:** failures of loopback-only binding, session API key
  enforcement, or unauthorized use of the managed `llama-server` by another
  local process.
- **IPC and webview:** vulnerabilities in Tauri `invoke` commands, capability
  configuration, CSP bypass, or injection that lets the webview run non-first-party
  code or exfiltrate the session API key.
- **Model / runtime parsing:** memory safety or unexpected-code-execution issues
  when loading or serving local GGUF models via the bundled llama.cpp runtime.
- **Local data exposure:** path traversal, privilege issues, or unintended
  disclosure of conversation data, settings, or session secrets from Omnira's
  local storage or IPC surface.
- **Dependencies and supply chain:** high/critical issues in core dependencies,
  or tampering with release artifacts / the packaging pipeline (including
  checksum verification of pinned `llama-server` binaries).
- **Logging privacy:** prompts or responses appearing in default local logs or
  redacted diagnostics exports.

### Out of scope

- **Physical or administrator access** to the host machine (full OS compromise).
  Note: attacks from another *unprivileged* local process against the runtime
  API **are** in scope — that is why loopback binding and the session API key exist.
- **Model behavioral flaws:** third-party model outputs, hallucinations, jailbreaks,
  or prompt-injection effects that are properties of model weights, not Omnira's
  host application.
- **Resource exhaustion / local DoS** from loading models larger than available
  GPU/VRAM or CPU/RAM.
- **Social engineering** that tricks users into running malicious software outside
  Omnira.
- **Post-MVP features not yet shipped** (for example RAG, ONNX/Windows ML workers,
  cloud providers, plugins). Report those against the code once they land; until
  then they are design topics in `docs/roadmap.md`.

---

## Coordinated Disclosure

Omnira follows coordinated disclosure:

1. Report privately and allow a reasonable window for analysis and a fix before
   public disclosure.
2. Limit proof-of-concept activity to what is necessary to demonstrate the issue;
   do not access, modify, or destroy other users' data.
3. After a fix ships, we will credit reporters in release notes or advisories
   unless anonymity is requested.

---

## Security Design References

- [Local security boundary](docs/local-security-boundary.md) — threat model, IPC,
  loopback + session key, webview CSP, logging privacy
- [Privacy](docs/privacy.md) — local-first defaults in plain language
- [Data ownership and storage](docs/data-ownership-and-storage.md) — where data lives
- [Packaging and process model](docs/packaging-process-model.md) — runtime fetch and
  SHA-256 verification for bundled `llama-server` artifacts

## Guidance for users

- Prefer GGUF files from reputable, verified authors and sources.
- Bundled runtime binaries are checksum-verified at fetch time; cryptographic
  verification of user-selected model files is planned for a later release.
- Keep your OS, GPU drivers, and Omnira installation up to date.
