# syntax=docker/dockerfile:1
# AET-WO-4407 (4401 Q2): BuildKit would pull this frontend by its tag.
# expect: syntax directive
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
