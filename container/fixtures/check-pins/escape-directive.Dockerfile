# escape=`
# Adversarial: the escape directive makes ` the continuation character, so a
# check that joins on "\" sees "gi`" and "t fetch", never "git fetch".
# expect: escape directive
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN true `
 && gi`
t fetch
