# Adversarial (AET-WO-4407): ways to reach /etc/apt that a literal "/etc/apt" misses.
# expect: may change apt's configuration: RUN echo deb x > /etc//apt/sources.list
# expect: may change apt's configuration: RUN cd /etc && echo deb x > apt/sources.list
# expect: may change apt's configuration: WORKDIR /etc
# expect: may change apt's configuration: RUN echo deb x > /etc/ap*/sources.list
# expect: may change apt's configuration: RUN cd / && echo deb x > etc/apt/sources.list
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2
RUN echo deb x > /etc//apt/sources.list
RUN cd /etc && echo deb x > apt/sources.list
WORKDIR /etc
RUN echo deb x > /etc/ap*/sources.list
RUN cd / && echo deb x > etc/apt/sources.list
