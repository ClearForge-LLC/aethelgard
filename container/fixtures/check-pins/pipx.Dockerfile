# A7: pipx installs from an index with no hash check.
# expect: pipx
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN pipx install empy==3.3.4
