# Repository Instructions

## Completing Changes

After implementing and verifying each requested change, commit all intended
repository changes with a descriptive commit message and push the current
branch to its configured upstream, unless the user explicitly says not to
commit or not to push.

After pushing, confirm the checkout is clean and synchronized, then finish.
Remote CI may run independently: do not wait for, poll, monitor, or investigate
remote tests, and do not make their results a completion gate, unless the user
explicitly asks. Leave the CI configuration unchanged.

## Proportionate Testing

Match verification effort to change risk and scope. Use the smallest checks
that provide meaningful confidence.

- Do not run the entire app test suite for every change. Filter local tests by
  the affected module or specific behavior; touching the app crate does not
  justify running all of its tests.
- For tiny visual-only changes (spacing, color, borders, copy, or local
  styling), do not add an automated test by default. Run formatting/diff
  checks, the narrow compile/check for touched code, and inspect the directly
  affected rendered state.
- For behavior changes or bug fixes, add or update a focused regression test
  only when it protects meaningful logic or a likely recurrence. Run targeted
  tests for the affected crate/module and interaction path.
- For shared components, theme tokens, overlay/input infrastructure,
  persistence, cross-platform behavior, or broad refactors, run broader tests
  and representative visual coverage because the blast radius is larger.
- Run the full app or workspace suite, full theme/size screenshot matrix, or
  exhaustive GUI harness only for release checkpoints, major shared-system
  changes, or explicit requests—not after every ordinary change. Any full-suite
  run needs a concrete, change-specific justification.
- For ordinary visual QA, use at most one standard theme and one materially
  different glass theme, and only affected states/sizes. Do not capture every
  theme unless theme rendering changed.
- Reuse existing tests and harness states. Do not create a test or screenshot
  case for every small edit.
- Stop once proportionate checks pass; avoid repeated polish/test loops without
  new evidence.
- If a broader unrelated check already fails, report it instead of expanding
  the task to fix or repeatedly rerun it.
