# Security Policy

## Supported versions

MateshaIDE is currently in active development and pre-1.0. Security fixes are applied to the
`main` branch and to the latest published release.

| Version | Supported |
| --- | --- |
| `main` (development) | ✅ |
| Latest release on [GitHub Releases](https://github.com/INiNiDS/mateshaide/releases) | ✅ |
| Older releases / tagged snapshots | ❌ |

Because the project is pre-1.0 and the API/architecture is still changing, backports to old
releases are not guaranteed. Please update to `main` or the latest release before reporting.

---

## Reporting a vulnerability

**Please do not open a public issue for security problems.** A public issue gives attackers
the details of the bug before it is fixed.

Instead, report it privately via **GitHub Security Advisories**:

1. Go to <https://github.com/INiNiDS/mateshaide/security/advisories/new>
2. Click **Report a vulnerability**
3. Fill in the summary, description, and impact
4. Submit

If private advisory reporting is unavailable to you, email
[security@ininids.in.rs](mailto:security@ininids.in.rs) and ask for a private channel.

Please include:

- A description of the vulnerability and the impact you believe it has
- The affected file(s), function(s), or module(s)
- Steps to reproduce, or a proof-of-concept if you have one
- The build you tested (native or browser) and your platform/GPU
- Any suggested mitigation

---

## What to expect

| Stage | Target |
| --- | --- |
| Acknowledgement of the report | 3 days |
| Initial assessment (valid / invalid / needs more info) | 7 days |
| Fix or mitigation plan | 14 days |
| Public disclosure | Coordinated with the reporter |

If a report is accepted, a fix will be prepared and merged, and a release containing the fix
will be published. The reporter will be credited in the release notes and the advisory unless
they prefer to remain anonymous.

If a report is declined, you will receive an explanation of why. We are happy to discuss
ambiguous cases, and we do not discourage good-faith research.

---

## Disclosure policy

- Report privately first, and give the maintainers a reasonable window to release a fix
  before making the issue public.
- We will not take legal action against good-faith research that stays within these rules:
  - no access to other users' data,
  - no denial of service,
  - no data exfiltration,
  - no social engineering or physical attacks,
  - no permanent modification of the repository,
  - respect for other users' privacy and for the AGPL-3.0 license.
- Findings that require a stable proof of concept, or that expose real user data, should be
  disclosed privately even if no deadline is set.

---

## Scope

### In scope

- The Rust source code in `src/` (native binary and the `wasm32` build)
- The WGSL shaders in `src/shape/`
- The build and deployment configuration in `.github/workflows/`
- The web build published on GitHub Pages
- The `wasm-bindgen` JavaScript bridge in `src/wasm/`

### Out of scope

- Vulnerabilities in upstream dependencies (`wgpu`, `winit`, `wasm-bindgen`, `glam`, etc.) —
  report those to the respective maintainers; we will happily help update the dependency
- Vulnerabilities in the browser, operating system, or GPU driver
- Denial of service caused only by very large documents created by the user themselves
- Issues found in third-party plugins, once the plugin system exists
- Lack of hardening features (e.g. missing sandboxing) with no demonstrated impact
- The web build served over `http://` rather than `https://` during local development

---

## Security notes for contributors

If you touch security-sensitive areas, keep these in mind:

- **Never** log or store secrets, tokens, or user documents in source code or logs. The
  `tracing` instrumentation must not include user content.
- **Do not** introduce `unsafe` code without a `// SAFETY:` comment explaining the invariant
  that makes it sound. Unsafe code is reviewed more strictly than safe code.
- **Validate untrusted input** — this includes anything coming from the browser, the
  filesystem, or a future plugin API. Bounds-check indices, validate file formats, and never
  assume a WGSL buffer or a `web-sys` value is well-formed.
- **Do not** use `unsafe` JS interop patterns (`unchecked` casts) without proving the layout
  matches.
- **WGSL shaders** run on the GPU: avoid unbounded loops, and validate any data used to
  index buffers.
- If you add a dependency, prefer well-maintained crates with a small dependency tree, and
  review their advisories.
