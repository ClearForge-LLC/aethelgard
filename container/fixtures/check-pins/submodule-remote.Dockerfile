# A7: a submodule moved to its branch tip, not the commit the tree records.
# expect: git submodule update --remote
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN git -C /opt/src submodule update --init --remote
