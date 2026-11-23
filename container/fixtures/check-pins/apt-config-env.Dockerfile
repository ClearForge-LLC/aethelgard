# Adversarial: apt reads its configuration from APT_CONFIG.
# expect: may change apt's configuration
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
ENV APT_CONFIG=/x.conf
COPY snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      make=4.4.1-2 \
 && rm -rf /var/lib/apt/lists/*
