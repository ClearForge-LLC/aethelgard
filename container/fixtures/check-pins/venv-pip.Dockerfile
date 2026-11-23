# A7: a venv whose pip is the unpinned python3-pip-whl Debian pulls in.
# expect: pin python3-pip-whl
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
COPY snapshot.sources /etc/apt/sources.list.d/debian.sources
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      python3-venv=3.13.5-1 \
 && rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /opt/venv
