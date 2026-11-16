# Adversarial: a RUN heredoc, whose body the shell joins and quotes its own way.
# expect: heredoc
# expect: git fetch
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN <<EOF
git -C /opt/src fetch
EOF
