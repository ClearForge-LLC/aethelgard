"""AET-WO-4400f: derive a spike Dockerfile from container/Dockerfile's renode-nvic stage (python3 -I make.py OUT):
the same pinned inputs plus upstream b14469bba, f2ac99657 and our channel patch, the two pinned DLL
hash checks dropped (the bytes are meant to change), then overlay Infrastructure.dll on the pinned tools
image. A spike image, tagged localhost/aethelgard-tools:spike-4400f-b, never published."""
import sys, re
src = open("container/Dockerfile").read()
start = src.index("FROM mcr.microsoft.com/dotnet/sdk")
end = src.index("# KiCad 10.0.7")
stage = src[start:end]
stage = stage.replace("COPY renode-nvic/nuget.sha256 /ci/nuget.sha256",
    "COPY spike-patches/b14469bba68dfa40218050eb1cbeae156b1495c8.patch /ci/patches/4.patch\n"
    "COPY spike-patches/f2ac99657469f0dfaf6e7c3ece8e2cc1a52d09ea.patch /ci/patches/5.patch\n"
    "COPY spike-patches/stm32-timer-channels-follow-frequency.patch /ci/patches/6.patch\n"
    "COPY renode-nvic/nuget.sha256 /ci/nuget.sha256", 1)
old = " && patch -d src/Infrastructure -p1 -i /ci/patches/3.patch \\\n"
assert stage.count(old) == 1
stage = stage.replace(old, old + "".join(f" && patch -d src/Infrastructure -p1 -i /ci/patches/{i}.patch \\\n" for i in (4, 5, 6)))
stage = re.sub(r' && echo "[0-9a-f]{64}  /out/(Infrastructure|Renode)\.dll" \| sha256sum -c - \\\n', "", stage)
assert "sha256sum -c - \\\n && chmod" not in stage and "/out/Infrastructure.dll\" | sha256sum" not in stage
image = open("container/IMAGE").read().strip()
out = stage + f"\nFROM {image}\nCOPY --from=renode-nvic --chown=0:0 --chmod=0644 /out/Infrastructure.dll /opt/renode/\nRUN sha256sum /opt/renode/Infrastructure.dll /opt/renode/Renode.dll\n"
open(sys.argv[1], "w").write(out)
