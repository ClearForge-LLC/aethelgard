# Adversarial: CRLF line ends, so a continuation is "\" then a carriage return.
# expect: a carriage return
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN true \
t fetch
