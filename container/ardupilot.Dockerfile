# The ArduPilot image (AET-WO-4400a, ruling D5 on PR #130): unmodified ArduPilot
# Copter-4.7.2 with its submodules, the GNU Arm toolchain the tag pins, and its
# Python set, so a board builds with --network none. It is not built FROM the
# tools image and nothing here enters it: this image is a GPL source
# distribution (ARDUPILOT-LICENSE.md). Every base is pinned by digest, every
# apt package by exact version, every download by a sha256 written here, the
# clone by its commit and every pip file by its hash; container/check-pins.sh
# refuses this file if any of that slips.
#
# Debian 13 (trixie), slim, by index digest.
FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2

LABEL org.opencontainers.image.source="https://github.com/ClearForge-LLC/aethelgard" \
      org.opencontainers.image.description="ArduPilot Copter-4.7.2 (fb773567) with its toolchain, for building board definitions offline" \
      org.opencontainers.image.licenses="GPL-3.0-or-later"

ENV DEBIAN_FRONTEND=noninteractive \
    PATH=/opt/venv/bin:/opt/gcc-arm-none-eabi-10-2020-q4-major/bin:$PATH

# What waf looked for in the spike's configure log (AET-WO-4400 S1): python3
# with venv, make, git and rsync. A host C compiler too, which the spike could
# not see because Furnace has one: waf builds AP_Scripting's binding generator
# (libraries/AP_Scripting/generator/src/main.c) with `gcc` (AET-WO-4400a, first
# CI run). bzip2 unpacks the toolchain; curl and ca-certificates fetch it.
# python3-pip-whl is the wheel `python3 -m venv` installs as the venv's pip; a
# dependency of python3-venv, so it is named here to pin it (AET-WO-4401,
# 4400a A7; 25.1.1+dfsg-1 read in the image published from 8030fa5).
# Exact versions from trixie and trixie-security.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates=20250419+deb13u1 \
      curl=8.14.1-2+deb13u7 \
      python3=3.13.5-1 \
      python3-venv=3.13.5-1 \
      python3-pip-whl=25.1.1+dfsg-1 \
      make=4.4.1-2 \
      git=1:2.47.3-0+deb13u1 \
      rsync=3.5.0+ds1-0+deb13u1 \
      bzip2=1.0.8-6 \
      gcc=4:14.2.0-1 \
      libc6-dev=2.41-12+deb13u4 \
 && rm -rf /var/lib/apt/lists/*

# The toolchain Copter-4.7.2's Tools/environment_install/install-prereqs-ubuntu.sh:249
# names, from ArduPilot's firmware server; the sha256 was measured in AET-WO-4400 S1.
RUN curl -fsSLo /tmp/gcc-arm.tar.bz2 https://firmware.ardupilot.org/Tools/STM32-tools/gcc-arm-none-eabi-10-2020-q4-major-x86_64-linux.tar.bz2 \
 && echo "21134caa478bbf5352e239fbc6e2da3038f8d2207e089efc96c3b55f1edcd618  /tmp/gcc-arm.tar.bz2" | sha256sum -c - \
 && tar -xjf /tmp/gcc-arm.tar.bz2 -C /opt --no-same-owner \
 && rm /tmp/gcc-arm.tar.bz2

# The pip set, every file by its hash, installed by the pinned pip above.
# setuptools first, alone, so empy's sdist builds without pip fetching an
# unhashed build backend.
COPY ardupilot-build-requirements.txt /tmp/ardupilot-build-requirements.txt
COPY ardupilot-requirements.txt /tmp/ardupilot-requirements.txt
RUN python3 -m venv /opt/venv \
 && /opt/venv/bin/pip install --no-cache-dir --require-hashes --no-deps -r /tmp/ardupilot-build-requirements.txt \
 && /opt/venv/bin/pip install --no-cache-dir --require-hashes --no-deps --no-build-isolation -r /tmp/ardupilot-requirements.txt \
 && rm /tmp/ardupilot-build-requirements.txt /tmp/ardupilot-requirements.txt

# ArduPilot at the tag, refused unless the tag still names the commit measured
# in AET-WO-4400 S1. Each submodule is checked out at the commit the tag's tree
# records, and the clean status says no submodule sits anywhere else.
RUN git clone --depth 1 -b Copter-4.7.2 --recurse-submodules --shallow-submodules https://github.com/ArduPilot/ardupilot.git /opt/ardupilot \
 && test "$(git -C /opt/ardupilot rev-parse HEAD)" = fb773567896014da007bca58358704d4a0d8516f \
 && test -z "$(git -C /opt/ardupilot status --porcelain)"

COPY ARDUPILOT-LICENSE.md /opt/ARDUPILOT-LICENSE.md
WORKDIR /opt/ardupilot
