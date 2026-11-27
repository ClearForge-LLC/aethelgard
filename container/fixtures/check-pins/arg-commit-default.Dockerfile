# Adversarial (AET-WO-1802): the commit ARG with a default would state a
# commit no workflow passed. Only a bare `ARG AETHELGARD_COMMIT` after FROM.
# expect: ARG other than a bare
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
ARG AETHELGARD_COMMIT=0123456789abcdef0123456789abcdef01234567
