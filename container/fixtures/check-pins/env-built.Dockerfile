# Adversarial: the subcommand comes from a variable (ARG is already refused).
# expect: git subcommand
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
ENV G=fetch
RUN git -C /opt/src $G
