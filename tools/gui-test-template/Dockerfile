# syntax=docker/dockerfile:1.7

ARG RUST_IMAGE=rust:1.96.0-bookworm@sha256:5e2214abe154fe26e39f64488952e5c991eeed1d6d6da7cc8381ae83927f0cfc
ARG RUNTIME_IMAGE=debian:bookworm-20260803-slim

FROM ${RUST_IMAGE} AS builder
ARG APP_PACKAGE
ARG APP_BINARY

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        build-essential \
        clang \
        cmake \
        git \
        libfontconfig1-dev \
        libssl-dev \
        libwayland-dev \
        libx11-xcb-dev \
        libxcb-render0-dev \
        libxcb-shape0-dev \
        libxcb-xfixes0-dev \
        libxkbcommon-x11-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
COPY . .

# ADOPT: Replace this Rust build when the target uses another toolchain. The
# only required output contract is an executable at /out/app.
RUN --mount=type=cache,id=gui-template-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=gui-template-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=gui-template-cargo-target,target=/workspace/target \
    test -n "${APP_PACKAGE}" \
    && test -n "${APP_BINARY}" \
    && cargo build --release --locked -p "${APP_PACKAGE}" \
    && install -D -m 0755 "target/release/${APP_BINARY}" /out/app

FROM ${RUNTIME_IMAGE} AS tester

# ADOPT: add the application's runtime libraries here; keep the isolated X/VNC
# packages and deterministic fonts.
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        ca-certificates \
        fonts-dejavu-core \
        fonts-liberation \
        imagemagick \
        libfontconfig1 \
        libgl1 \
        libvulkan1 \
        libwayland-client0 \
        libx11-xcb1 \
        libxcb-render0 \
        libxcb-shape0 \
        libxcb-xfixes0 \
        libxkbcommon-x11-0 \
        mesa-vulkan-drivers \
        novnc \
        openbox \
        websockify \
        x11-utils \
        x11-xserver-utils \
        x11vnc \
        xdotool \
        xvfb \
    && rm -rf /var/lib/apt/lists/* \
    && fc-cache -f

COPY --from=builder /out/app /opt/gui-test/app
COPY tools/gui-test-template/container/ /opt/gui-test/bin/
COPY tools/gui-test-template/baselines/ /opt/gui-test/baselines/
RUN chmod 0755 /opt/gui-test/app /opt/gui-test/bin/*.sh

ENV DISPLAY=:99 \
    GUI_TEST_SCREEN=1440x900x24 \
    GUI_TEST_DPI=96 \
    HOME=/tmp/gui-test-home \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    LIBGL_ALWAYS_SOFTWARE=1 \
    MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
    TZ=UTC \
    XDG_CACHE_HOME=/tmp/gui-test-home/.cache \
    XDG_CONFIG_HOME=/tmp/gui-test-home/.config \
    XDG_DATA_HOME=/tmp/gui-test-home/.local/share

USER 10001:10001
ENTRYPOINT ["/opt/gui-test/bin/entrypoint.sh"]
CMD ["/opt/gui-test/bin/run-gui-test.sh"]
