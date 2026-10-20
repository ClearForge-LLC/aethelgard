# AET-WO-0200 spike: a candidate tools image, built in CI only to measure the
# answers to the work order's §0. The build half replaces this with
# container/Dockerfile after the architect's ruling. Every download is checked
# against a sha256 written here; the bases are pinned by digest in the workflow.
ARG BASE
FROM ${BASE}

# 1 = install KiCad from the base distribution's archive (candidate b).
ARG KICAD_FROM_APT=0

USER root
SHELL ["/bin/bash", "-euo", "pipefail", "-c"]
ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    KANI_HOME=/opt/kani \
    PATH=/usr/local/cargo/bin:$PATH \
    DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl gcc libc6-dev \
      ngspice libngspice0 $([ "$KICAD_FROM_APT" = 1 ] && echo kicad) \
 && rm -rf /var/lib/apt/lists/*

# Renode 1.17.0: the official GitHub release asset; sha256 is GitHub's digest
# of that asset.
RUN curl -fsSLo /tmp/renode.tar.gz https://github.com/renode/renode/releases/download/v1.17.0/renode-1.17.0.linux-portable.tar.gz \
 && echo "4ba7c68b59e2447f188ef4b4b112fcccd2802582c460beedceb63b84e5605a5f  /tmp/renode.tar.gz" | sha256sum -c - \
 && mkdir -p /opt/renode \
 && tar -xzf /tmp/renode.tar.gz -C /opt/renode --strip-components=1 \
 && ln -s /opt/renode/renode /usr/local/bin/renode \
 && rm /tmp/renode.tar.gz

# Rust 1.99.1 (rust-toolchain.toml) through rustup-init 1.29.1 from the
# versioned archive, checked against static.rust-lang.org's published sha256.
RUN curl -fsSLo /tmp/rustup-init https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init \
 && echo "dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.99.1 --component rustfmt,clippy \
 && rm /tmp/rustup-init

# Kani 0.68.0: the driver from crates.io (cargo checks the registry checksum),
# its backend from the release bundle checked here, installed at build time so
# nothing is fetched on first run.
RUN cargo install --locked kani-verifier --version 0.68.0 \
 && curl -fsSLo /tmp/kani.tar.gz https://github.com/model-checking/kani/releases/download/kani-0.68.0/kani-0.68.0-x86_64-unknown-linux-gnu.tar.gz \
 && echo "32e2b484d73ede0bbf64a2cf0879c4259422497d8aec4ee67d448de9ae7843d3  /tmp/kani.tar.gz" | sha256sum -c - \
 && cargo kani setup --use-local-bundle /tmp/kani.tar.gz \
 && rm /tmp/kani.tar.gz
