"""AET-WO-1219: the PDF page of a fact in a held document's text, read from
the "=== page N ===" markers PyMuPDF wrote into the held .txt (PDF page
indices, 1-based; ~/.cache/aet-1211/ds/PROVENANCE.md), or "=====PAGE N=====". Usage:
pagefind.py <doc.txt> <regex> [<regex> ...]: pages where every regex
matches within one page."""
import re, sys
text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
pages = re.split(r"(?:=== page |=====PAGE )(\d+)(?: ===|=====)", text)
hits = []
for i in range(1, len(pages) - 1, 2):
    n, body = int(pages[i]), pages[i + 1]
    if all(re.search(p, body, re.S) for p in sys.argv[2:]):
        hits.append(n)
print(" ".join(map(str, hits)) if hits else "none")
