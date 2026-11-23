# Adversarial (AET-WO-4407): a fullwidth 4 ends the digest; under en_US.UTF-8
# [0-9a-f] matched it until the check set LC_ALL=C.
# expect: unpinned base
FROM debian@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa４
