# Reusable isolated GUI-test template

This directory is a product-neutral starting point for GUI projects that can
run on Linux/X11. The default check launches the application on a private Xvfb
display, finds its window by title, runs an optional project assertion hook,
captures the exact window, optionally compares a reviewed baseline, and returns
one concise pass/fail line.

The normal service has no network and never mounts the host X11/Wayland socket,
browser profile, `/dev/uinput`, or Docker socket. The optional `inspect` profile
publishes authenticated noVNC on localhost only and uses an internal Docker
network with no external egress.

## Adopt it

1. Copy this directory into the target repository (keeping
   `tools/gui-test-template/` avoids changing the helper paths), then merge
   `dockerignore.example` into the repository-root `.dockerignore`.
2. In `compose.yaml`, replace `your-app-package`, `your-app-binary`, and
   `^Your App` with the real Cargo package, binary, and X11 title regex.
3. Edit `container/launch-project.sh` to set deterministic fixture/data paths
   and app-level state arguments. Do not add host desktop mounts.
4. Edit `container/project-assert.sh` to call the project's semantic test hook
   or accessibility/IPC driver. It receives the discovered X11 window ID. Do
   not replace it with coordinate clicks.
5. If the project is not Rust, replace only the `builder` stage in the
   Dockerfile while retaining the `/out/app` runtime contract.
6. Add a repository task such as `gui-test: tools/gui-test-template/bin/run.sh`.
7. Run once, review `artifacts/gui-test/.../screenshots/smoke.png`, and only
   then copy it to `baselines/smoke.png` if a pixel gate is appropriate.

Docker Engine, Docker Compose v2.20+, and BuildKit are required. The Rust
example pins the Rust 1.96.0 Bookworm multi-platform digest and a dated Debian
Bookworm runtime, uses `cargo build --locked`, and caches registry, Git, and
target data. Re-pin both image references deliberately when adopting updates.

## Commands

```sh
# Concise automated gate; detailed output is saved under artifacts/gui-test/.
tools/gui-test-template/bin/run.sh

# Optional human inspection. A temporary VNC password is printed.
tools/gui-test-template/bin/inspect.sh

# Stop only the template's inspection stack.
tools/gui-test-template/bin/inspect.sh down
```

Set `GUI_TEST_MAX_DIFF_PIXELS` to an intentional nonzero tolerance only after
reviewing which pixels vary. `GUI_TEST_SETTLE_SECONDS` is a fallback; a
project-level readiness hook in `project-assert.sh` is preferable.

## Substitution map

| File | Replace | Purpose |
| --- | --- | --- |
| `compose.yaml` | package, binary, title regex | Build and window identity |
| `Dockerfile` | builder dependencies/stage | Project toolchain |
| `container/launch-project.sh` | environment and arguments | Deterministic state |
| `container/project-assert.sh` | assertion body | Semantic readiness/behavior |
| `baselines/smoke.png` | reviewed image, optional | Visual regression contract |

The template deliberately does not include credentials, application paths,
host mounts other than artifacts, or an automated baseline-approval command.
