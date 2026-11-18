# The pinned tools image (AET-WO-0200): KiCad, Renode, ngspice, Kani and Rust,
# headless. Every base is pinned by digest, every apt package by exact version,
# and every download by a sha256 written here. container/check-pins.sh refuses
# this file if any of that slips. Ruled on PR #7: base candidate a, default
# user root, tool state under /opt.
#
# Renode 1.17.0's NVIC fixes (AET-WO-1605, ruled on #223). No Renode release
# carries them, so this stage rebuilds the two assemblies they touch from the
# source Antmicro built the portable asset from: renode f1dd1b4af (two commits
# before the v1.17.0 tag), whose src/Infrastructure is cd4b002a, every
# submodule at the commit f1dd1b4af records, each a GitHub commit tarball
# checked here. renode-nvic/ holds upstream's three renode-infrastructure
# commits as patches: e7a8584c (ICSR.RETTOBASE counts external interrupts),
# c8b2075c and 7b687540 (cache maintenance ignored, not logged). Every NuGet
# package restored is checked against renode-nvic/nuget.sha256, and nothing
# else may be restored. Infrastructure.dll is built as Antmicro built it, with
# the source revision the asset's own carries; Renode.dll is published as
# build.sh publishes it, with the build string 1.17.0+20260907gitf1dd1b4af.aet1605,
# so `renode --version` and every archive header name this build. Unpatched,
# each reproduces Antmicro's assembly but for the 72 bytes of deterministic
# identity, and both outputs are checked against the sha256 measured (FEEDBACK).
FROM mcr.microsoft.com/dotnet/sdk:8.0.405@sha256:da1f981a47b9017065a702f054f186316366af92dfaea76377fe4f2da2a042bc AS renode-nvic
ENV DOTNET_CLI_TELEMETRY_OPTOUT=1 \
    DOTNET_NOLOGO=1 \
    NUGET_PACKAGES=/ci/src/renode/.nuget
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      patch=2.7.6-7 \
 && rm -rf /var/lib/apt/lists/*
RUN curl -fsSLo /tmp/renode.tar.gz https://github.com/renode/renode/archive/f1dd1b4af7838b45a925c17603cdfed0a583844a.tar.gz \
 && echo "8106ab990a8c2ec8c832e58c6c51ea37e8b199eb0574e5c834c84cde23160285  /tmp/renode.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode \
 && tar -xzf /tmp/renode.tar.gz -C /ci/src/renode --strip-components=1 --no-same-owner \
 && rm /tmp/renode.tar.gz \
 && curl -fsSLo /tmp/renode-infrastructure.tar.gz https://github.com/renode/renode-infrastructure/archive/cd4b002aac2398994c4a61a34a8b31b5700facf2.tar.gz \
 && echo "83e6af97c9911e089cf914cccf89334de7f711f3a8d8c16977c045f5311c6fa8  /tmp/renode-infrastructure.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/src/Infrastructure \
 && tar -xzf /tmp/renode-infrastructure.tar.gz -C /ci/src/renode/src/Infrastructure --strip-components=1 --no-same-owner \
 && rm /tmp/renode-infrastructure.tar.gz \
 && curl -fsSLo /tmp/migrant.tar.gz https://github.com/antmicro/Migrant/archive/9e135403b5e1612d9c6bed7d3498bf39ada0a95c.tar.gz \
 && echo "2680aa7d0aa5281d038d20c4012d4a655079eed0ef5151429fe103517648f380  /tmp/migrant.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/Migrant \
 && tar -xzf /tmp/migrant.tar.gz -C /ci/src/renode/lib/Migrant --strip-components=1 --no-same-owner \
 && rm /tmp/migrant.tar.gz \
 && curl -fsSLo /tmp/elfsharp.tar.gz https://github.com/antmicro/elfsharp/archive/bf797c5bbec983578ded3081a8e17d1c646e7c32.tar.gz \
 && echo "ee5eff750aad71316e0dac6a3680415847d161a8768878dda1ca429bbd716342  /tmp/elfsharp.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/ELFSharp \
 && tar -xzf /tmp/elfsharp.tar.gz -C /ci/src/renode/lib/ELFSharp --strip-components=1 --no-same-owner \
 && rm /tmp/elfsharp.tar.gz \
 && curl -fsSLo /tmp/antshell.tar.gz https://github.com/antmicro/AntShell/archive/ca49bd325b6943802578d70ae575baddc7005a49.tar.gz \
 && echo "22f9cbe6c79b0c4a355d176ca4a131c87e04edd93c4e45d824f4c429994bcf14  /tmp/antshell.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/AntShell \
 && tar -xzf /tmp/antshell.tar.gz -C /ci/src/renode/lib/AntShell --strip-components=1 --no-same-owner \
 && rm /tmp/antshell.tar.gz \
 && curl -fsSLo /tmp/fdtsharp.tar.gz https://github.com/antmicro/FdtSharp/archive/fb43c1507c2c7680d386fcef2dcb75bbe1fae273.tar.gz \
 && echo "8c28c44f44f22b88203ebbcb2dfa90770929ff63a561ada995baa11d9de35a83  /tmp/fdtsharp.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/FdtSharp \
 && tar -xzf /tmp/fdtsharp.tar.gz -C /ci/src/renode/lib/FdtSharp --strip-components=1 --no-same-owner \
 && rm /tmp/fdtsharp.tar.gz \
 && curl -fsSLo /tmp/termsharp.tar.gz https://github.com/antmicro/termsharp/archive/c13c96b95e92c1a8e29705f0884f690ea2f1a876.tar.gz \
 && echo "ab88196ba1de3edd8acdbd7d8057aebb8a44bcd6a57d9df6001e7471f1bd6662  /tmp/termsharp.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/termsharp \
 && tar -xzf /tmp/termsharp.tar.gz -C /ci/src/renode/lib/termsharp --strip-components=1 --no-same-owner \
 && rm /tmp/termsharp.tar.gz \
 && curl -fsSLo /tmp/xwt.tar.gz https://github.com/antmicro/xwt/archive/edb7793688c83448b1eb5336ab2fc9e98905eaa3.tar.gz \
 && echo "c263adba3106720b9b40a8fcae8988266e19a3c3f033f380387884534bb3290e  /tmp/xwt.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/termsharp/xwt \
 && tar -xzf /tmp/xwt.tar.gz -C /ci/src/renode/lib/termsharp/xwt --strip-components=1 --no-same-owner \
 && rm /tmp/xwt.tar.gz \
 && curl -fsSLo /tmp/packet.net.tar.gz https://github.com/antmicro/Packet.Net/archive/20e0f2fe40743fed4e6a81bb8fe9523c257f168b.tar.gz \
 && echo "6f60f56d7d3eeae7f8e20d764e6bd9c1f8df07e4c31514034fd2fadf6104bdc0  /tmp/packet.net.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/Packet.Net \
 && tar -xzf /tmp/packet.net.tar.gz -C /ci/src/renode/lib/Packet.Net --strip-components=1 --no-same-owner \
 && rm /tmp/packet.net.tar.gz \
 && curl -fsSLo /tmp/bc-csharp.tar.gz https://github.com/antmicro/bc-csharp/archive/f2e740be178845f5abb7e475ea0490d29637f4d7.tar.gz \
 && echo "9b97a926bcb8d91f27851ddc481fabc4a503a419b90f9e8f616867c94bf10ce7  /tmp/bc-csharp.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/bc-csharp \
 && tar -xzf /tmp/bc-csharp.tar.gz -C /ci/src/renode/lib/bc-csharp --strip-components=1 --no-same-owner \
 && rm /tmp/bc-csharp.tar.gz \
 && curl -fsSLo /tmp/cxxdemangler.tar.gz https://github.com/antmicro/CxxDemangler/archive/71eb9736623081a09712a8cb08bcd322ce17c7b5.tar.gz \
 && echo "9bb247e0c56a77c1e54711940a006436f8a6aee027b1d5b1e7c2e5cd4480a528  /tmp/cxxdemangler.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/CxxDemangler \
 && tar -xzf /tmp/cxxdemangler.tar.gz -C /ci/src/renode/lib/CxxDemangler --strip-components=1 --no-same-owner \
 && rm /tmp/cxxdemangler.tar.gz \
 && curl -fsSLo /tmp/biggustave.tar.gz https://github.com/antmicro/BigGustave/archive/6858c7e4e529e6bc6543fbaaea0f3523a2663a55.tar.gz \
 && echo "6455c7da8c91d14329c8491c12d674b583770bef731d4c0cb5ed29892353e7ee  /tmp/biggustave.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/BigGustave \
 && tar -xzf /tmp/biggustave.tar.gz -C /ci/src/renode/lib/BigGustave --strip-components=1 --no-same-owner \
 && rm /tmp/biggustave.tar.gz \
 && curl -fsSLo /tmp/options-parser.tar.gz https://github.com/antmicro/options-parser/archive/2b2b67a0b8f9ec93e1ba2cf2cfadded38d2855a4.tar.gz \
 && echo "9e242a3ef2642fa4561858e618683ceb807e8f4aa91c34b67b22fca4788a1cde  /tmp/options-parser.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/options-parser \
 && tar -xzf /tmp/options-parser.tar.gz -C /ci/src/renode/lib/options-parser --strip-components=1 --no-same-owner \
 && rm /tmp/options-parser.tar.gz \
 && curl -fsSLo /tmp/inplitftpserver.tar.gz https://github.com/antmicro/InpliTftpServer/archive/2a4877dce99fd7a30b85b5bf217c631538cd7086.tar.gz \
 && echo "e5310e93531e0efd227db94d16c28d89e5ad54871c86490112610f5212591004  /tmp/inplitftpserver.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/InpliTftpServer \
 && tar -xzf /tmp/inplitftpserver.tar.gz -C /ci/src/renode/lib/InpliTftpServer --strip-components=1 --no-same-owner \
 && rm /tmp/inplitftpserver.tar.gz \
 && curl -fsSLo /tmp/renode-resources.tar.gz https://github.com/renode/renode-resources/archive/14b80cde0a136b684f316eb7f6a31aeaae0684bf.tar.gz \
 && echo "9298a8968f68f9c149e142f8a14cce17f2e533e22bfdba37135024bc6c5b64d3  /tmp/renode-resources.tar.gz" | sha256sum -c - \
 && mkdir -p /ci/src/renode/lib/resources \
 && tar -xzf /tmp/renode-resources.tar.gz -C /ci/src/renode/lib/resources --strip-components=1 --no-same-owner \
 && rm /tmp/renode-resources.tar.gz
COPY renode-nvic/Directory.Build.targets /ci/src/renode/Directory.Build.targets
COPY renode-nvic/e7a8584cac829e060e98fc2850dcc9df61ddd594.patch /ci/patches/1.patch
COPY renode-nvic/c8b2075c042d1da6ae07745a63c8bd3edea31c4f.patch /ci/patches/2.patch
COPY renode-nvic/7b68754055b87ab3cc64c929eaac4be2b8060fef.patch /ci/patches/3.patch
COPY renode-nvic/nuget.sha256 /ci/nuget.sha256
WORKDIR /ci/src/renode
RUN patch -d src/Infrastructure -p1 -i /ci/patches/1.patch \
 && patch -d src/Infrastructure -p1 -i /ci/patches/2.patch \
 && patch -d src/Infrastructure -p1 -i /ci/patches/3.patch \
 && dotnet restore -r linux-x64 -p:Configuration=Release -p:GenerateFullPaths=true -p:Platform=AnyCPU -p:Architecture=x64 -p:PORTABLE=true -p:SatelliteResourceLanguages=en -p:RemoveWPF=true -p:BuildDate=20260907 -p:CommitString=gitf1dd1b4af.aet1605 src/Renode/Renode.csproj \
 && find .nuget -name '*.nupkg' -printf '%P\n' > /tmp/restored \
 && sort -o /tmp/restored /tmp/restored \
 && cut -c67- /ci/nuget.sha256 > /tmp/pinned \
 && cmp /tmp/restored /tmp/pinned \
 && cd /ci/src/renode/.nuget \
 && sha256sum -c --quiet --strict /ci/nuget.sha256 \
 && cd /ci/src/renode \
 && dotnet build --no-restore src/Infrastructure/src/Infrastructure.csproj -c Release -p:WPFPath=/ci/src/renode/lib/resources/libraries/WPF/linux-x64 -p:SourceRevisionId=cd4b002aac2398994c4a61a34a8b31b5700facf2 \
 && mkdir -p /out \
 && cp src/Infrastructure/src/bin/Release/net8.0/Infrastructure.dll /out/ \
 && dotnet publish --no-restore -maxcpucount:1 -r linux-x64 -f net8.0 -p:Configuration=Release -p:GenerateFullPaths=true -p:Platform=AnyCPU -p:Architecture=x64 -p:PORTABLE=true -p:SatelliteResourceLanguages=en -p:RemoveWPF=true -p:BuildDate=20260907 -p:CommitString=gitf1dd1b4af.aet1605 --output /ci/publish src/Renode/Renode.csproj \
 && cp /ci/publish/Renode.dll /out/ \
 && echo "ddb19d7bf7224525677cbcf77e7890901412eed4ea014e23618dd94f6cd524b5  /out/Infrastructure.dll" | sha256sum -c - \
 && echo "5734eb0cd95a2e6b84f2c0d449cf51e73a63a301db4098cbc40d2fcbe789f633  /out/Renode.dll" | sha256sum -c - \
 && chmod 0644 /out/Infrastructure.dll /out/Renode.dll

# KiCad 10.0.7, official image, index digest; Debian 13 (trixie), amd64 only.
# It already carries ngspice and libngspice0 44.2+ds-1 and ca-certificates.
# Its vendor, url and authors labels describe KiCad's image, not this one, so
# the LABEL below replaces them; a Dockerfile cannot delete an inherited label,
# so authors is set empty (AET-WO-4403 D6, ruled on #176).
FROM kicad/kicad:10.0.7@sha256:3224d1bf143185ecc1bd3a1082a36f37f0f4e59d707983a16515a845e3737fc6

LABEL org.opencontainers.image.source="https://github.com/ClearForge-LLC/aethelgard" \
      org.opencontainers.image.description="Aethelgard tools: KiCad, Renode, ngspice, Kani, Rust" \
      org.opencontainers.image.licenses="GPL-3.0-or-later AND CC-BY-SA-4.0" \
      org.opencontainers.image.documentation="https://github.com/ClearForge-LLC/aethelgard/blob/main/container/TOOLS-SOURCES.md" \
      org.opencontainers.image.vendor="ClearForge LLC" \
      org.opencontainers.image.url="https://github.com/ClearForge-LLC/aethelgard" \
      org.opencontainers.image.authors=""

USER root
ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    KANI_HOME=/opt/kani \
    PATH=/opt/cargo/bin:$PATH \
    DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1

# Exact versions from the trixie archive. A pin that leaves the archive fails
# this step by name; the published image digest is what consumers pin.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates=20250419 \
      curl=8.14.1-2+deb13u7 \
      gcc=4:14.2.0-1 \
      libc6-dev=2.41-12+deb13u4 \
      ngspice=44.2+ds-1 \
      libngspice0=44.2+ds-1 \
 && rm -rf /var/lib/apt/lists/*

# Renode 1.17.0, the official GitHub release asset; the sha256 is GitHub's
# digest of that asset.
RUN curl -fsSLo /tmp/renode.tar.gz https://github.com/renode/renode/releases/download/v1.17.0/renode-1.17.0.linux-portable.tar.gz \
 && echo "4ba7c68b59e2447f188ef4b4b112fcccd2802582c460beedceb63b84e5605a5f  /tmp/renode.tar.gz" | sha256sum -c - \
 && mkdir -p /opt/renode \
 && tar -xzf /tmp/renode.tar.gz -C /opt/renode --strip-components=1 --no-same-owner \
 && chmod -R go-w /opt/renode \
 && ln -s /opt/renode/renode /usr/local/bin/renode \
 && rm /tmp/renode.tar.gz
# The two assemblies the renode-nvic stage rebuilt, over the checked asset
# (AET-WO-1605): Renode 1.17.0 with upstream's NVIC fixes, named by its build
# string. The asset's sha256 above still checks every other byte.
COPY --from=renode-nvic --chown=0:0 --chmod=0644 /out/Infrastructure.dll /out/Renode.dll /opt/renode/

# Rust 1.99.1 (rust-toolchain.toml) through rustup-init 1.29.1 from the
# versioned archive, checked against static.rust-lang.org's published sha256.
# The thumbv7em-none-eabihf target links the twin's firmware (AET-WO-2100, R4).
RUN curl -fsSLo /tmp/rustup-init https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init \
 && echo "dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain 1.99.1 --component rustfmt,clippy --target thumbv7em-none-eabihf \
 && rm /tmp/rustup-init

# Kani 0.68.0: the driver from crates.io (cargo checks the registry checksum)
# and its backend from the release bundle checked here, installed now so
# nothing is fetched on first run (architecture.md §5). The bundle unpacks with
# its builder's uid (1001), so every toolchain tree is made root's and writable
# by no one else in this same step (AET-WO-1601 A1: a tool's uid boundary means
# nothing if another uid can replace the tool; smoke.sh check 10).
RUN cargo install --locked kani-verifier --version 0.68.0 \
 && curl -fsSLo /tmp/kani.tar.gz https://github.com/model-checking/kani/releases/download/kani-0.68.0/kani-0.68.0-x86_64-unknown-linux-gnu.tar.gz \
 && echo "32e2b484d73ede0bbf64a2cf0879c4259422497d8aec4ee67d448de9ae7843d3  /tmp/kani.tar.gz" | sha256sum -c - \
 && cargo kani setup --use-local-bundle /tmp/kani.tar.gz \
 && rm /tmp/kani.tar.gz \
 && chown -R root:root /opt/rustup /opt/cargo /opt/kani \
 && chmod -R a+rX,go-w /opt/rustup /opt/cargo /opt/kani

# KiCad's library licence text (CC-BY-SA-4.0 with KiCad's design exception),
# beside the libraries it covers. The file is byte-identical to the copy
# vendored in aethelgard-kicad-lib (AET-WO-1300 D10, AET-WO-1900).
COPY KICAD-LICENSE.md /usr/share/kicad/LICENSE.md

# The written source offer (AET-WO-4403): KiCad, Renode's LGPL parts and the
# rest, and every Debian package by source and version, read from dpkg last so
# it lists exactly what the image holds. check-sources.sh judges both against
# the built image. The licences label names the principal components (KiCad
# and its libraries); the documentation label points at the full list, and the
# publish job sets it to this file at the commit it publishes.
COPY TOOLS-SOURCES.md /SOURCES.md
# D5 (ruled by Scotty on #176): the GPL-2.0-only Debian sources, mirrored
# next to this image, one row per file with its sha256 and the artifact's digest.
COPY TOOLS-DEBIAN-GPL2.tsv /SOURCES-debian-gpl2.tsv
RUN dpkg-query -W -f '${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\thttps://snapshot.debian.org/package/${source:Package}/${source:Version}/\n' > /tmp/dpkg.tsv \
 && LC_ALL=C sort -o /SOURCES-debian.tsv /tmp/dpkg.tsv \
 && rm /tmp/dpkg.tsv
