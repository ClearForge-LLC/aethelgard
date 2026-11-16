"""container/debian-gpl2.py: which installed Debian source packages carry code
under the GPL version 2 only, or the LGPL version 2.0 or 2.1 only (AET-WO-4403
D5, ruled by Scotty 2026-11-16; LGPL added by the architect's ruling on #181,
Q2). GPLv2 §3 and LGPL-2.1 §4 count source only from the same place, with no
third-party-server clause, so these sources are mirrored next to the image,
not only pointed at on snapshot.debian.org. Code under "or later" can be taken
under GPLv3 or LGPLv3, whose §6(d) allows the third-party route.

Run inside an image as `python3 -I debian-gpl2.py`. It prints, sorted and
tab-separated, one line per (source, source version) that it flags:

    source  version  reason

A package is flagged, conservatively, when
- its machine-readable (DEP-5) copyright file has a Files paragraph whose
  licence has a conjunct that is exactly GPL-2 or LGPL-2 / LGPL-2.1 (with
  ".0", "-only" or neither; no "+" and no "or" alternative), or
- its copyright file is free text that names a GNU (Lesser or Library)
  General Public License and version 2 at all ("review": free text cannot say
  which files are version 2 only, so it is mirrored rather than judged).
Debian's own debian/* packaging counts too: it is part of the source package.
"""
import re
import subprocess

GPL2 = re.compile(r"(gpl-?2(\.0)?|lgpl-?2(\.[01])?)(-only)?", re.I)


def conjuncts(expr):
    """The conjuncts of a DEP-5 licence expression that offer no alternative."""
    out = []
    for c in re.split(r"\s+and\s+|,", expr):
        c = c.strip().strip("()")
        if c and not re.search(r"\sor\s", c):
            out.append(c.split()[0])
    return out


def reason(text):
    if text.lstrip().lower().startswith("format:"):
        for para in re.split(r"\n\s*\n", text):
            if not re.search(r"^Files:", para, re.M):
                continue
            m = re.search(r"^License:\s*(.+)$", para, re.M)
            if m and any(GPL2.fullmatch(t) for t in conjuncts(m.group(1))):
                return "dep5:" + m.group(1).strip()
        return None
    if "General Public License" in text and re.search(r"version 2|GPL-?2|\bv2\b", text, re.I):
        return "free-text:GPL version 2"
    return None


def main():
    rows = subprocess.run(
        ["dpkg-query", "-W", "-f", "${binary:Package}\t${source:Package}\t${source:Version}\n"],
        capture_output=True, text=True, check=True).stdout.splitlines()
    flagged = {}
    for r in rows:
        b, s, v = r.split("\t")
        try:
            with open(f"/usr/share/doc/{b.split(':')[0]}/copyright", encoding="utf-8", errors="replace") as f:
                why = reason(f.read())
        except OSError:
            continue
        if why and (s, v) not in flagged:
            flagged[(s, v)] = why
    for (s, v), why in sorted(flagged.items()):
        print(f"{s}\t{v}\t{why}")


if __name__ == "__main__":
    main()
