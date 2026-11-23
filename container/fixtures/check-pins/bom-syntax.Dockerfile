# syntax=docker/dockerfile:1
# Adversarial: BuildKit skips a UTF-8 byte-order mark; the directive behind it is hidden.
# expect: byte-order mark
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
