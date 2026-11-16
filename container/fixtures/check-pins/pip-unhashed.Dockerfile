# A pip install without --require-hashes, refused by name, by its full path.
# expect: pip install without --require-hashes
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN /opt/venv/bin/pip install --no-cache-dir empy==3.3.4
