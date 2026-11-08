# The engine image (AET-WO-1501): the pinned tools image plus the aethelgard
# binary built from this repository's source inside it. The base is the digest
# in container/IMAGE; the workflow reads that file and passes it as TOOLS_IMAGE,
# and is never handed a tag. check-pins.sh refuses this file if the FROM is
# anything but ${TOOLS_IMAGE}, if the ARG has a default, or if container/IMAGE
# is missing or not a digest. Build from the repository root:
#   docker build -f container/engine.Dockerfile --build-arg TOOLS_IMAGE="$(cat container/IMAGE)" .
ARG TOOLS_IMAGE
FROM ${TOOLS_IMAGE}

LABEL org.opencontainers.image.source="https://github.com/ClearForge-LLC/aethelgard" \
      org.opencontainers.image.description="Aethelgard engine: the aethelgard CLI on the pinned tools image" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"

# The context is bind-mounted into the one RUN, never COPYed: a COPY is a layer
# of its own and ships in the image even if a later RUN deletes it. The mount is
# read-only; CARGO_HOME and the target are under /tmp. The dependency fetch is
# the only network use; the build is --offline against a throwaway CARGO_HOME,
# so the base image's /opt/cargo is not touched. The one layer holds only the
# installed binary.
RUN --mount=type=bind,target=/src \
    cd /src \
 && export CARGO_HOME=/tmp/cargo-build CARGO_TARGET_DIR=/tmp/target-build \
 && cargo fetch --locked \
 && cargo build --release --locked --offline -p aethelgard-cli \
 && install -m 0755 /tmp/target-build/release/aethelgard /usr/local/bin/aethelgard \
 && cd / \
 && rm -rf /tmp/cargo-build /tmp/target-build
