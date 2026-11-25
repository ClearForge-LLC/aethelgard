"""AET-WO-4400i (1) P3: each request of an idle-* run against its reply (PARAM_VALUE or PARAM_ERROR by id, the FTP
NAK by sequence + 1), from out/inj/RUN/{injected.tsv,mav.tsv}. Usage: python3 -I latency.py OUT RUN... -> TSV."""
import json, sys
out = sys.argv[1]
print("run\trequest\tsent_s\treplied_s\tlatency_ms")
for run in sys.argv[2:]:
    inj = [l.rstrip("\n").split("\t") for l in open(f"{out}/inj/{run}/injected.tsv")][1:]
    mav = [l.rstrip("\n").split("\t") for l in open(f"{out}/inj/{run}/mav.tsv")][1:]
    for t, msg, *_ in inj:
        if msg.startswith("monitor"):
            continue
        f = json.loads(msg.split(" ", 1)[1]); t = int(t)
        if "id" in f:
            key, ok = f["id"], lambda r: r[1] in ("PARAM_VALUE", "PARAM_ERROR") and json.loads(r[3])["param_id"] == f["id"]
        else:
            key, ok = "FTP seq %d" % f["seq"], lambda r: r[1] == "FILE_TRANSFER_PROTOCOL" and json.loads(r[3])["seq"] == f["seq"] + 1
        rep = next((int(r[0]) for r in mav if int(r[0]) >= t and ok(r)), None)
        print(f"{run}\t{key}\t{t / 1e9:.3f}\t{'' if rep is None else f'{rep / 1e9:.4f}'}\t{'none' if rep is None else f'{(rep - t) / 1e6:.1f}'}")
