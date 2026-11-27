# Adversarial (AET-WO-1802): the commit ARG before FROM is refused; it is
# admitted bare, after FROM, only.
# expect: ARG other than a bare
ARG AETHELGARD_COMMIT
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
