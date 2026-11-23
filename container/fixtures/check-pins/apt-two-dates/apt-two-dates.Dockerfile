# Adversarial: two sources files beside, at two timestamps.
# expect: b.sources:3: snapshot 20261105T120000Z, not 20261117T120000Z
# expect: b.sources:10: snapshot 20261105T120000Z, not 20261117T120000Z
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
COPY b.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      make=4.4.1-2 \
 && rm -rf /var/lib/apt/lists/*
