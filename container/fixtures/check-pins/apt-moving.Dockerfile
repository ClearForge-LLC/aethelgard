# AET-WO-4407: apt-get update with Debian's moving index, no snapshot source.
# expect: no snapshot source COPYed
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      make=4.4.1-2 \
 && rm -rf /var/lib/apt/lists/*
