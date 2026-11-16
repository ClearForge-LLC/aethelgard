# Adversarial: a clone split over a continuation, so the physical shape
# check never sees "git clone" on one line.
# expect: git clone outside the checked shape
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN git \
    clone --depth 1 -b v1 --remote-submodules https://example.org/x.git /opt/x
