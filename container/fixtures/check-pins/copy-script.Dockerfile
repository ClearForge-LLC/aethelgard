# Adversarial: a script copied in and run; the check cannot read what it runs.
# expect: script
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
COPY copy-script.sh /tmp/copy-script.sh
RUN sh /tmp/copy-script.sh
