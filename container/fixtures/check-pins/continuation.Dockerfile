# Adversarial: a line continuation splits the command word itself. Docker
# joins "gi\" and "t fetch" into "git fetch" with no space between, and
# allows spaces and tabs after the backslash (as on the "gi\" line here).
# expect: git fetch
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN true \
 && gi\ 	
t fetch
