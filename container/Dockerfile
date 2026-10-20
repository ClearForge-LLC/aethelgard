# The pinned tools image (AET-WO-0200): KiCad, Renode, ngspice, Kani and Rust,
# headless. Every base is pinned by digest, every apt package by exact version,
# and every download by a sha256 written here. container/check-pins.sh refuses
# this file if any of that slips. Ruled on PR #7: base candidate a, default
# user root, tool state under /opt.
#
# KiCad 10.0.7, official image, index digest; Debian 13 (trixie), amd64 only.
# It already carries ngspice and libngspice0 44.2+ds-1 and ca-certificates.
FROM kicad/kicad:10.0.7@sha256:3224d1bf143185ecc1bd3a1082a36f37f0f4e59d707983a16515a845e3737fc6

LABEL org.opencontainers.image.source="https://github.com/ClearForge-LLC/aethelgard" \
      org.opencontainers.image.description="Aethelgard tools: KiCad, Renode, ngspice, Kani, Rust" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"

USER root
ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    KANI_HOME=/opt/kani \
    PATH=/opt/cargo/bin:$PATH \
    DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1

# Exact versions from the trixie archive. A pin that leaves the archive fails
# this step by name; the published image digest is what consumers pin.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates=20250419 \
      curl=8.14.1-2+deb13u7 \
      gcc=4:14.2.0-1 \
      libc6-dev=2.41-12+deb13u4 \
      ngspice=44.2+ds-1 \
      libngspice0=44.2+ds-1 \
 && rm -rf /var/lib/apt/lists/*

# Renode 1.17.0, the official GitHub release asset; the sha256 is GitHub's
# digest of that asset.
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

# Kani 0.68.0: the driver from crates.io (cargo checks the registry checksum)
# and its backend from the release bundle checked here, installed now so
# nothing is fetched on first run (architecture.md §5).
RUN cargo install --locked kani-verifier --version 0.68.0 \
 && curl -fsSLo /tmp/kani.tar.gz https://github.com/model-checking/kani/releases/download/kani-0.68.0/kani-0.68.0-x86_64-unknown-linux-gnu.tar.gz \
 && echo "32e2b484d73ede0bbf64a2cf0879c4259422497d8aec4ee67d448de9ae7843d3  /tmp/kani.tar.gz" | sha256sum -c - \
 && cargo kani setup --use-local-bundle /tmp/kani.tar.gz \
 && rm /tmp/kani.tar.gz \
 && chmod -R a+rX /opt/rustup /opt/cargo /opt/kani
