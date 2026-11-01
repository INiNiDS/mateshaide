# Contributing to MateshaIDE

Thanks for taking the time to contribute! MateshaIDE is an IDE for mathematics — a Rust
application built on `wgpu` that runs both natively (via `winit`) and in the browser
(via WebAssembly).

Every contribution helps, and contributions of all sizes are welcome: bug reports,
documentation fixes, tests, and full features.

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## Table of contents

- [Ways to contribute](#ways-to-contribute)
- [Prerequisites](#prerequisites)
- [Setting up the project](#setting-up-the-project)
- [Code style and lints](#code-style-and-lints)
- [Project structure](#project-structure)
- [Commit message convention](#commit-message-convention)
- [Pull requests](#pull-requests)
- [Issues](#issues)
- [Reporting security issues](#reporting-security-issues)
- [License](#license)

---

## Ways to contribute

- **Report bugs** — open an issue using the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md).
- **Suggest features** — open an issue using the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md).
- **Improve the documentation** — fix typos, clarify instructions, or expand the README.
- **Write code** — pick up an open issue, or work on the plugin architecture described in the README.
- **Review pull requests** — comments, suggestions, and testing are all valuable.

Not a Rust programmer yet? Improving the README, triaging issues, and reproducing bugs are
all genuinely useful contributions.

---

## Prerequisites

| Tool | Notes |
| --- | --- |
| Rust (stable) | The project uses Rust **2024 edition**. Install via [rustup](https://rustup.rs/). |
| `wasm-pack` | Only needed for the browser build. `cargo install wasm-pack` |
| `wasm32-unknown-unknown` target | `rustup target add wasm32-unknown-unknown` |
| A GPU with Vulkan/Metal/DX12 | Required for the native build (`wgpu`). Software fallback (e.g. `lavapipe`) also works. |

The native version is noticeably smoother than the browser version, so please test natively
when you can.

---

## Setting up the project

```bash
git clone https://github.com/INiNiDS/mateshaide.git
cd mateshaide
cargo build
```

Run the project:

```bash
cargo run
```

Build the browser version:

```bash
wasm-pack build --release --target web \
  --out-dir web/pkg --out-name matesha_ide
```

The generated `web/pkg/` directory is what gets published to GitHub Pages (see
`.github/workflows/deploy.yml`). For a quick local preview use `python3 preview.py`.

---

## Code style and lints

The project enables `clippy::all` **and** `clippy::pedantic` as warnings, plus a set of
stricter lints (`dbg_macro`, `todo`, `clone_on_ref_ptr`). See `Cargo.toml` and `clippy.toml`
for the full configuration.

Before opening a pull request, make sure all of these pass cleanly:

```bash
cargo fmt --all -- --check   # formatting
cargo clippy --all-targets --all-features -- -D warnings
cargo build                   # must compile without errors
```

A few conventions the codebase follows:

- **No `unsafe` without a comment explaining why it is sound.** If you must use `unsafe`,
  add a `// SAFETY:` comment justifying it.
- **No `dbg!`, `todo!`, or `unimplemented!` in merged code.** Lint exceptions are not
  accepted; implement it or open an issue instead.
- **Prefer `?` and typed errors** (`thiserror` is already a dependency) over panics in
  library code. Panics in the binary entry point are acceptable.
- **Keep modules focused.** A module should have a single responsibility and expose a
  small, intentional public surface through its `mod.rs` re-exports.
- **Document public items** with `///` doc comments. Explain *why*, not just *what*.
- **Match surrounding style.** Look at neighbouring files before writing new code.

---

## Project structure

```
src/
├── app/         # Application lifecycle, event loop glue, input actions
├── canvas/      # Canvas model, camera, draw tools, undo/redo history, text layers
├── core/        # Core utilities: colors, collections
├── dev/         # Developer/test-mode helpers and debug spawning
├── engine/      # Engine setup (window, surface, device)
├── graphics/    # wgpu pipelines, renderer, overlay, text renderer
├── input/       # Input manager, shortcuts, actions, bindings
├── shape/       # Shape registry, instances, SDF math, WGSL shader
├── theme/       # Palette and theme definitions
├── wasm/        # WebAssembly entry point and JS bridge
└── main.rs      # Native binary entry point
```

Key ideas:

- The **canvas** owns the document state (shapes, text) and the **history** stack
  (undo/redo).
- The **camera** controls zoom and pan and produces the `CameraUniform` uploaded to the GPU.
- **Shapes** are drawn as instanced quads with an SDF evaluated in `shape_base.wgsl`.
- **Input** is bound through `input/shortcut.rs` and `input/action.rs`; adding a new
  keyboard shortcut should not require touching the render loop.
- The **plugin manager** is the planned core feature — contributions toward plugin
  architecture are especially welcome.

---

## Commit message convention

The repository follows [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>
```

Types in use: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `build`, `ci`, `chore`.

Examples:

```
feat(canvas): add zoom-to-cursor on wheel events
fix(renderer): clear depth buffer between frames
refactor(clippy): address pedantic lint warnings
docs(readme): document the new shortcut list
```

Keep the summary in the imperative mood, under ~72 characters, with no trailing period.

---

## Pull requests

1. **Open an issue first** for anything larger than a small fix, so the design can be
   discussed before you write a lot of code. For tiny fixes, just open the PR.
2. **Branch from `main`** with a descriptive name, e.g. `feat/zoom-to-cursor`.
3. **Keep the diff focused.** Unrelated refactors, formatting-only churn, and drive-by
   changes make reviews harder. If you need cleanup, open a separate PR.
4. **Make sure the checks pass** locally (see [Code style and lints](#code-style-and-lints)).
5. **Fill in the [pull request template](.github/PULL_REQUEST_TEMPLATE.md)** — describe what
   changed, why, and how you verified it.
6. **Include screenshots or a short screen recording** for anything that changes visuals or
   interaction. The native and web builds can look different; mention which one you tested.
7. **Test both builds** if your change touches platform-specific code (`src/wasm/`,
   `web-sys`, native-only paths).

By contributing, you agree that your contributions will be licensed under the
[AGPL-3.0](LICENSE) license of this project.

---

## Issues

- Search existing issues before opening a new one — duplicates get closed.
- Use the templates in `.github/ISSUE_TEMPLATE/`. The more precise the report, the faster it
  gets fixed.
- Include your OS, GPU, Rust version (`rustc --version`), and whether you were running the
  native build or the browser build.
- A minimal reproduction case is worth more than a long description.

---

## Reporting security issues

**Do not open a public issue for security problems.** Follow the process in
[SECURITY.md](SECURITY.md) instead.

---

## License

MateshaIDE is licensed under the **AGPL-3.0**. By contributing, you confirm that you are
comfortable with your work being distributed under that license.
