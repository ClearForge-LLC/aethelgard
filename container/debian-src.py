"""container/debian-src.py: the mirror of the GPLv2-only Debian sources next to
each public image (AET-WO-4403 D5, ruled by Scotty 2026-11-16).

One OCI artifact per (source package, version), in the image's own ghcr
package, under a tag derived from the two and never moved. Each artifact has
the GCC mirror's shape (gcc-source.manifest.json): the empty config, one
application/octet-stream layer per file of the Debian source package (.dsc,
.orig.tar.*, .debian.tar.*, ...), and three annotations. Its manifest is
derived from the committed table byte for byte, so every digest is known
before anything is pushed, and the table carries it.

The table (container/<KIND>-DEBIAN-GPL2.tsv), one line per file, sorted:

    source  version  file  sha1  sha256  size  tag  manifest_sha256

`sha1` is snapshot.debian.org's key for the file; `sha256` and `size` are
what is pushed and checked. The last two columns repeat on every line of a
package and are derived; `check` re-derives them.

    debian-src.py table FLAGGED.tsv CACHE > TABLE    (once, on a workstation)
    debian-src.py check TABLE
    debian-src.py manifest TABLE TAG > FILE
    debian-src.py publish REPO TABLE DIR       (the main publish job)
    debian-src.py fetch REPO TABLE DIR         (anonymous, via oci-mirror.sh)

A refusal is printed by name with exit 1 (INV-06).
"""
import hashlib
import json
import os
import re
import subprocess
import sys
import time
import urllib.parse
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
SNAP = "https://snapshot.debian.org"
EMPTY = {"mediaType": "application/vnd.oci.empty.v1+json",
         "digest": "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a",
         "size": 2}
COLS = ("source", "version", "file", "sha1", "sha256", "size", "tag", "manifest_sha256")


def die(msg):
    print(f"debian-src: {msg}", file=sys.stderr)
    sys.exit(1)


# OCI tags are [A-Za-z0-9_][A-Za-z0-9._-]{0,127}. Debian source names are
# [a-z0-9][a-z0-9.+-]* and versions [0-9A-Za-z.+~:-]; neither has "_", so
# "_" is free to escape the three characters a tag cannot hold, and "__"
# separates the name from the version. The map is injective over every name
# and version the checks below admit, so two packages never share a tag.
ESC = {"+": "_p", "~": "_t", ":": "_e"}


def tag_of(source, version):
    enc = lambda s: "".join(ESC.get(c, c) for c in s)
    t = f"debian-src.{enc(source)}__{enc(version)}"
    if not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9._-]{0,127}", t):
        die(f"no OCI tag for {source} {version}: {t}")
    return t


def manifest_bytes(source, version, files):
    """files: [(name, sha256, size)] in table order. Deterministic bytes."""
    m = {
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "artifactType": "application/vnd.clearforge.aethelgard.corresponding-source.v1",
        "config": EMPTY,
        "layers": [{"mediaType": "application/octet-stream", "digest": "sha256:" + h, "size": n,
                    "annotations": {"org.opencontainers.image.title": name}} for name, h, n in files],
        "annotations": {
            "org.opencontainers.image.description": f"Debian source package {source} {version}, byte for byte, the corresponding source of GPL-2.0-only code in this image",
            "org.opencontainers.image.url": f"{SNAP}/package/{source}/{version}/",
            "org.opencontainers.image.source": "https://github.com/ClearForge-LLC/aethelgard",
        },
    }
    return (json.dumps(m, indent=2) + "\n").encode()


def read_table(path):
    try:
        lines = open(path, encoding="ascii").read().split("\n")
    except (OSError, UnicodeDecodeError):
        die(f"{path} missing, unreadable or not ASCII")
    if lines[-1] != "":
        die(f"{path} does not end with a newline")
    rows = []
    for i, l in enumerate(lines[:-1], 1):
        f = l.split("\t")
        if len(f) != len(COLS):
            die(f"{path}:{i} has {len(f)} columns, not {len(COLS)}")
        rows.append(dict(zip(COLS, f)))
    if not rows:
        die(f"{path} is empty")
    return rows


def groups(path, rows):
    """{(source, version): [row, ...]} after checking every field and order."""
    keys = [(r["source"], r["version"], r["file"]) for r in rows]
    if keys != sorted(keys) or len(set(keys)) != len(keys):
        die(f"{path} is not sorted by source, version, file, or repeats a file")
    out = {}
    for i, r in enumerate(rows, 1):
        where = f"{path}:{i}"
        if not re.fullmatch(r"[a-z0-9][a-z0-9.+-]+", r["source"]):
            die(f"{where} source is not a Debian source name")
        if not re.fullmatch(r"([0-9]+:)?[0-9A-Za-z.+~-]+", r["version"]):
            die(f"{where} version is not a Debian version")
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+~_-]*", r["file"]):
            die(f"{where} file is not a plain file name")
        if not re.fullmatch(r"[0-9a-f]{40}", r["sha1"]) or not re.fullmatch(r"[0-9a-f]{64}", r["sha256"]):
            die(f"{where} sha1 or sha256 is malformed")
        if not re.fullmatch(r"[1-9][0-9]*", r["size"]):
            die(f"{where} size is not a positive integer")
        out.setdefault((r["source"], r["version"]), []).append(r)
    for (s, v), rs in out.items():
        if sum(r["file"].endswith(".dsc") for r in rs) != 1:
            die(f"{path}: {s} {v} does not have exactly one .dsc")
        t = tag_of(s, v)
        d = hashlib.sha256(manifest_bytes(s, v, [(r["file"], r["sha256"], int(r["size"])) for r in rs])).hexdigest()
        for r in rs:
            if r["tag"] != t:
                die(f"{path}: {s} {v} {r['file']} names tag {r['tag']}, not {t}")
            if r["manifest_sha256"] != d:
                die(f"{path}: {s} {v} {r['file']} names manifest sha256:{r['manifest_sha256']}, but its rows derive sha256:{d}")
    return out


def check(path):
    g = groups(path, read_table(path))
    n = sum(len(rs) for rs in g.values())
    size = sum(int(r["size"]) for rs in g.values() for r in rs)
    print(f"ok   {path}: {len(g)} source packages, {n} files, {size} bytes")
    return g


def get(url, out=None):
    req = urllib.request.Request(url, headers={"User-Agent": "aethelgard-debian-src"})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=300) as r:
                if out is None:
                    return r.read()
                h1, h256, n = hashlib.sha1(), hashlib.sha256(), 0
                with open(out, "wb") as f:
                    while chunk := r.read(1 << 20):
                        f.write(chunk); h1.update(chunk); h256.update(chunk); n += len(chunk)
                return h1.hexdigest(), h256.hexdigest(), n
        except OSError as e:
            if attempt == 3:
                die(f"{url}: {e}")
            time.sleep(5 * (attempt + 1))


def dsc_sha256(path):
    """{name: (sha256, size)} from a .dsc's Checksums-Sha256 field."""
    text = open(path, encoding="utf-8", errors="replace").read()
    m = re.search(r"^Checksums-Sha256:\n((?: .+\n)+)", text, re.M)
    if not m:
        die(f"{path} has no Checksums-Sha256")
    out = {}
    for line in m.group(1).splitlines():
        h, n, name = line.split()
        out[name] = (h, int(n))
    return out


def table(flagged, cache):
    """Build the table from snapshot.debian.org; every file is downloaded and
    its sha1 must be snapshot's and its sha256 and size the .dsc's."""
    os.makedirs(cache, exist_ok=True)
    rows, t0, total = [], time.time(), 0
    for line in open(flagged):
        s, v = line.rstrip("\n").split("\t")[:2]
        info = json.loads(get(f"{SNAP}/mr/package/{urllib.parse.quote(s)}/{urllib.parse.quote(v, safe='')}/srcfiles?fileinfo=1"))
        # A file can sit under two names when two source packages share an
        # orig tarball; the .dsc says which name is this package's.
        names_of = {h1: {e["name"] for e in entries} for h1, entries in info.get("fileinfo", {}).items()}
        dscs = [(n, h1) for h1, ns in names_of.items() for n in ns if n.endswith(".dsc")]
        if len(dscs) != 1:
            die(f"{s} {v}: snapshot lists {len(dscs)} .dsc files")
        dsc_name, dsc_h1 = dscs[0]
        p = os.path.join(cache, dsc_h1)
        if not os.path.exists(p):
            get(f"{SNAP}/file/{dsc_h1}", p)
        files = {dsc_name: dsc_h1}
        for name in dsc_sha256(p):
            hs = [h1 for h1, ns in names_of.items() if name in ns]
            if len(hs) != 1:
                die(f"{s} {v}: snapshot has {len(hs)} files named {name}")
            files[name] = hs[0]
        got = {}
        for name, h1 in sorted(files.items()):
            p = os.path.join(cache, h1)
            if not os.path.exists(p):
                got[name] = get(f"{SNAP}/file/{h1}", p)
            else:
                b = open(p, "rb").read()
                got[name] = (hashlib.sha1(b).hexdigest(), hashlib.sha256(b).hexdigest(), len(b))
            if got[name][0] != h1:
                die(f"{s} {v} {name}: snapshot served sha1 {got[name][0]}, not {h1}")
            total += got[name][2]
        listed = dsc_sha256(os.path.join(cache, dsc_h1))
        for name, (h, n) in listed.items():
            if got[name][1:] != (h, n):
                die(f"{s} {v} {name}: sha256/size {got[name][1:]} is not the .dsc's {(h, n)}")
        fl = sorted(files)
        t = tag_of(s, v)
        d = hashlib.sha256(manifest_bytes(s, v, [(n, got[n][1], got[n][2]) for n in fl])).hexdigest()
        for n in fl:
            rows.append((s, v, n, files[n], got[n][1], str(got[n][2]), t, d))
        print(f"{s} {v}: {len(fl)} files, {sum(got[n][2] for n in fl)} bytes", file=sys.stderr)
    for r in sorted(rows):
        print("\t".join(r))
    dt = time.time() - t0
    print(f"table: {total} bytes in {dt:.0f} s ({total / max(dt, 1) / 1e6:.2f} MB/s)", file=sys.stderr)


def oci(*args):
    return subprocess.run(["bash", os.path.join(HERE, "oci-mirror.sh"), *args]).returncode


def publish(repo, path, workdir):
    g = check(path)
    os.makedirs(workdir, exist_ok=True)
    pushed = 0
    for (s, v), rs in sorted(g.items()):
        man = os.path.join(workdir, rs[0]["tag"] + ".json")
        with open(man, "wb") as f:
            f.write(manifest_bytes(s, v, [(r["file"], r["sha256"], int(r["size"])) for r in rs]))
        d = rs[0]["manifest_sha256"]
        rc = oci("fetch", f"{repo}@sha256:{d}", workdir, "--manifest", man, "--head")
        if rc == 0:
            continue
        if rc != 2:
            die(f"{s} {v}: the mirror at {repo}@sha256:{d} is there but refused (above); nothing pushed")
        files = os.path.join(workdir, "files")
        os.makedirs(files, exist_ok=True)
        for r in rs:
            sha1, sha256, n = get(f"{SNAP}/file/{r['sha1']}", os.path.join(files, r["file"]))
            if (sha1, sha256, n) != (r["sha1"], r["sha256"], int(r["size"])):
                die(f"{s} {v} {r['file']}: snapshot served sha256 {sha256}, {n} bytes, not the table's")
        if oci("push", repo, rs[0]["tag"], man, files) != 0:
            die(f"{s} {v}: push refused (above)")
        for r in rs:
            os.remove(os.path.join(files, r["file"]))
        if oci("fetch", f"{repo}@sha256:{d}", workdir, "--manifest", man) != 0:
            die(f"{s} {v}: pushed, but not fetched anonymously (above)")
        pushed += 1
    print(f"ok   {repo}: {len(g)} source packages mirrored, {pushed} pushed now")


def fetch(repo, path, workdir):
    g = check(path)
    os.makedirs(workdir, exist_ok=True)
    for (s, v), rs in sorted(g.items()):
        man = os.path.join(workdir, rs[0]["tag"] + ".json")
        with open(man, "wb") as f:
            f.write(manifest_bytes(s, v, [(r["file"], r["sha256"], int(r["size"])) for r in rs]))
        if oci("fetch", f"{repo}@sha256:{rs[0]['manifest_sha256']}", workdir, "--manifest", man, "--discard") != 0:
            die(f"{s} {v}: not fetched anonymously from {repo} (above)")
    print(f"ok   {repo}: all {len(g)} source packages fetched anonymously and verified")


def main(a):
    if a[:1] == ["table"] and len(a) == 3:
        table(a[1], a[2])
    elif a[:1] == ["check"] and len(a) == 2:
        check(a[1])
    elif a[:1] == ["manifest"] and len(a) == 3:
        g = groups(a[1], read_table(a[1]))
        hit = [(k, rs) for k, rs in g.items() if rs[0]["tag"] == a[2]]
        if not hit:
            die(f"{a[1]} has no tag {a[2]}")
        (s, v), rs = hit[0]
        sys.stdout.buffer.write(manifest_bytes(s, v, [(r["file"], r["sha256"], int(r["size"])) for r in rs]))
    elif a[:1] == ["publish"] and len(a) == 4:
        publish(*a[1:])
    elif a[:1] == ["fetch"] and len(a) == 4:
        fetch(*a[1:])
    else:
        die("usage: debian-src.py table|check|manifest|publish|fetch ...")


if __name__ == "__main__":
    main(sys.argv[1:])
