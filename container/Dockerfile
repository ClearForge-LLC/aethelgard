# The pinned tools image (AET-WO-0200): KiCad, Renode, ngspice, Kani and Rust,
# headless. Every base is pinned by digest, every apt package by exact version,
# and every download by a sha256 written here. container/check-pins.sh refuses
# this file if any of that slips. Ruled on PR #7: base candidate a, default
# user root, tool state under /opt.
#
# KiCad 10.0.7, official image, index digest; Debian 13 (trixie), amd64 only.
# It already carries ngspice and libngspice0 44.2+ds-1 and ca-certificates.
# Its vendor, url and authors labels describe KiCad's image, not this one, so
# the LABEL below replaces them; a Dockerfile cannot delete an inherited label,
# so authors is set empty (AET-WO-4403 D6, ruled on #176).
FROM kicad/kicad:10.0.7@sha256:3224d1bf143185ecc1bd3a1082a36f37f0f4e59d707983a16515a845e3737fc6

LABEL org.opencontainers.image.source="https://github.com/ClearForge-LLC/aethelgard" \
      org.opencontainers.image.description="Aethelgard tools: KiCad, Renode, ngspice, Kani, Rust" \
      org.opencontainers.image.licenses="GPL-3.0-or-later AND CC-BY-SA-4.0" \
      org.opencontainers.image.documentation="https://github.com/ClearForge-LLC/aethelgard/blob/main/container/TOOLS-SOURCES.md" \
      org.opencontainers.image.vendor="ClearForge LLC" \
      org.opencontainers.image.url="https://github.com/ClearForge-LLC/aethelgard" \
      org.opencontainers.image.authors=""

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
 && tar -xzf /tmp/renode.tar.gz -C /opt/renode --strip-components=1 --no-same-owner \
 && chmod -R go-w /opt/renode \
 && ln -s /opt/renode/renode /usr/local/bin/renode \
 && rm /tmp/renode.tar.gz

# Rust 1.99.1 (rust-toolchain.toml) through rustup-init 1.29.1 from the
# versioned archive, checked against static.rust-lang.org's published sha256.
# The thumbv7em-none-eabihf target links the twin's firmware (AET-WO-2100, R4).
RUN curl -fsSLo /tmp/rustup-init https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init \
 && echo "dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.99.1 --component rustfmt,clippy --target thumbv7em-none-eabihf \
 && rm /tmp/rustup-init

# Kani 0.68.0: the driver from crates.io (cargo checks the registry checksum)
# and its backend from the release bundle checked here, installed now so
# nothing is fetched on first run (architecture.md §5). The bundle unpacks with
# its builder's uid (1001), so every toolchain tree is made root's and writable
# by no one else in this same step (AET-WO-1601 A1: a tool's uid boundary means
# nothing if another uid can replace the tool; smoke.sh check 10).
RUN cargo install --locked kani-verifier --version 0.68.0 \
 && curl -fsSLo /tmp/kani.tar.gz https://github.com/model-checking/kani/releases/download/kani-0.68.0/kani-0.68.0-x86_64-unknown-linux-gnu.tar.gz \
 && echo "32e2b484d73ede0bbf64a2cf0879c4259422497d8aec4ee67d448de9ae7843d3  /tmp/kani.tar.gz" | sha256sum -c - \
 && cargo kani setup --use-local-bundle /tmp/kani.tar.gz \
 && rm /tmp/kani.tar.gz \
 && chown -R root:root /opt/rustup /opt/cargo /opt/kani \
 && chmod -R a+rX,go-w /opt/rustup /opt/cargo /opt/kani

# KiCad's library licence text (CC-BY-SA-4.0 with KiCad's design exception),
# beside the libraries it covers. The file is byte-identical to the copy
# vendored in aethelgard-kicad-lib (AET-WO-1300 D10, AET-WO-1900).
COPY KICAD-LICENSE.md /usr/share/kicad/LICENSE.md

# The written source offer (AET-WO-4403): KiCad, Renode's LGPL parts and the
# rest, and every Debian package by source and version, read from dpkg last so
# it lists exactly what the image holds. check-sources.sh judges both against
# the built image. The licences label names the principal components (KiCad
# and its libraries); the documentation label points at the full list, and the
# publish job sets it to this file at the commit it publishes.
COPY TOOLS-SOURCES.md /SOURCES.md
# D5 (ruled by Scotty on #176): the GPL-2.0-only Debian sources, mirrored
# next to this image, one row per file with its sha256 and the artifact's digest.
COPY TOOLS-DEBIAN-GPL2.tsv /SOURCES-debian-gpl2.tsv
RUN dpkg-query -W -f '${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\thttps://snapshot.debian.org/package/${source:Package}/${source:Version}/\n' > /tmp/dpkg.tsv \
 && LC_ALL=C sort -o /SOURCES-debian.tsv /tmp/dpkg.tsv \
 && rm /tmp/dpkg.tsv
