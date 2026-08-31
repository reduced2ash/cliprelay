# Isolated GUI testing

The normal GUI check runs the release Rust/GPUI binary inside Docker Engine on
an Xvfb display owned by the container. It does not use or mount the host
desktop, input devices, browser data, or Docker socket.

## Automated path

Requirements are Docker Engine and Docker Compose v2.20 or newer. Run:

```sh
make ui-test
```

The command validates the Compose model, builds the Rust 1.96.0 image, and
launches five app-level states: library, history, settings, command center, and
an exercised library workflow (paging, scroll recovery, hover preview, Random,
GPUI playback, and source reveal).
State selection uses ClipRelay's boot environment hooks; the runner never sends
screen-coordinate input. Each state must create a visible `ClipRelay` X11
window of the requested size and remain alive through capture. Screenshots are
read back from GPUI's rendered Blade surface rather than from the X11 drawable,
so Vulkan presentation remains visible under Xvfb. The resulting
history, settings, command-center, and workflow images must also differ
materially from the library image, so ignored boot hooks cannot pass as five
duplicate tests.

Set `GUI_TEST_THEME_MODE=frosted_glass` or
`GUI_TEST_THEME_MODE=graphite_glass` to run the same state matrix through the
native backdrop material path. Those variants additionally verify that all
eight captured Library, History, Settings, command, Prepare, and Studio states
retain alpha for whole-window compositor blur; Studio media apertures remain
independently opaque.

For a bounded Random Sources review, run
`GUI_TEST_ONLY_RANDOM_SOURCES=1 make ui-test`. It captures the popup with a
long source tree at normal and compact sizes, its empty state, and a real
keyboard selection. Combine it with `GUI_TEST_THEME_MODE=graphite_glass` for
the matching glass-material pass.

The terminal result is intentionally short. Every run writes:

```text
artifacts/ui-test/<UTC timestamp>-<pid>/
  summary.txt
  compose-config.yaml
  build.log
  container.log
  environment.txt
  metrics.tsv
  logs/
  screenshots/
  diffs/
```

Agents should read those files only after a failure. `make ui-test-clean`
removes only this harness's inspection containers and network; it never prunes
Docker globally or deletes artifacts.

## Optional visual baselines

The suite always saves screenshots. If a reviewed PNG with the same case name
exists under `tools/ui-test/baselines/`, it also writes an amplified visual diff
and enforces `GUI_TEST_MAX_DIFF_PIXELS` (default `0`) after a 2% per-pixel fuzz.
No baseline is accepted automatically. Create or update one only after a human
reviews the image produced by the pinned container.

The checkout did not contain Linux reference images when this harness was
added, so the first Docker-capable run is a launch-and-capture gate rather than
a pre-approved pixel regression gate. The next useful app-specific hook would
be a small test-mode state/ready endpoint that reports the active page and
settled async generation; that would replace the bounded settle interval with
an explicit readiness assertion.

## Human inspection with noVNC

For a temporary browser view of the isolated display:

```sh
make ui-inspect
```

The helper generates an ephemeral VNC password unless
`GUI_TEST_VNC_PASSWORD` is already set, prints it, and binds noVNC only to
`127.0.0.1:6080` (override the port with `GUI_TEST_NOVNC_PORT`). Stop it with
Ctrl-C or `make ui-inspect-down`. The `inspect` Compose profile is disabled by
default, its bridge network is internal, and the VNC server itself listens only
inside the container.

## Reproducibility and boundaries

- Rust is pinned to the 1.96.0 Bookworm multi-platform image digest, the runtime
  uses the dated Debian Bookworm 20260803 image, and the repository lockfile is
  built with `--locked`; GPUI 0.2.2 resolves to the checked-in `vendor/gpui`
  patch.
- Debian Bookworm, a fixed 1440x900x24 Xvfb screen, 96 DPI, DejaVu/Liberation
  fonts, `C.UTF-8`, UTC, Openbox, and Mesa software rendering are used.
- The automated runtime has no network, drops Linux capabilities, enables
  `no-new-privileges`, uses a read-only root filesystem, and gets only a
  run-specific artifact bind mount plus private tmpfs mounts.
- The image build needs normal package and Cargo registry network access. The
  running app does not.
- The app binary is built in a multi-stage image. BuildKit caches Cargo registry,
  Git, and target data between builds without copying the toolchain into the
  runtime image.
