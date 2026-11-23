# Adversarial: the sources file beside names Debian's moving archive.
# expect: not a snapshot.debian.org source line
# expect: no snapshot sources beside this file
# expect: no snapshot source COPYed
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
COPY debian.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      make=4.4.1-2 \
 && rm -rf /var/lib/apt/lists/*
