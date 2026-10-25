<!--
Thanks for contributing to MateshaIDE!
Please fill in the sections below. Delete any that do not apply to your change.
-->

## Description

<!-- What does this PR do? What problem does it solve? Keep it short and clear. -->

## Type of change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (existing behaviour changes)
- [ ] Refactor / internal cleanup (no functional change)
- [ ] Performance improvement
- [ ] Documentation update
- [ ] Build / CI / tooling change

## Related issues

<!--
Link any issues this PR fixes or relates to, e.g.:
Fixes #123
Closes #45, #46
-->

## Changes

<!-- Bullet list of the notable changes, pointing at files where relevant. -->

-
-

## How was it tested?

<!--
Describe how you verified the change. Examples:
- Native build: cargo run, reproduced the original behaviour and confirmed the fix
- Browser build: wasm-pack build --release --target web --out-dir pkg --out-name matesha_ide
- Manual test steps
- Test plan
-->

## Visual changes

<!--
If the change affects rendering, layout, or interaction, add screenshots or a short
screen recording here (before/after is ideal). Mention whether you tested the native
build, the browser build, or both.
-->

- [ ] This PR changes visuals or interaction (screenshots attached above)

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo build` succeeds
- [ ] The change works in the native build (`cargo run`)
- [ ] The change works in the browser build (`wasm-pack build --release --target web --out-dir pkg --out-name matesha_ide`)
- [ ] No `dbg!`, `todo!`, or `unimplemented!` left in the code
- [ ] Any new `unsafe` block has a `// SAFETY:` comment explaining why it is sound
- [ ] Public items are documented with `///` doc comments
- [ ] Tests added or updated where the change is testable

## Notes for reviewers

<!--
Anything a reviewer should know: areas of uncertainty, trade-offs, follow-up work,
or places where you would especially like feedback.
-->
