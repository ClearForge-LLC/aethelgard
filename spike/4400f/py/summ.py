"""AET-WO-4400f: one line per verdict (python3 -I summ.py FILE...): exit-free summary of a simulate verdict."""
import json, sys
for p in sys.argv[1:]:
    try:
        v = json.load(open(p))
    except Exception as e:
        print(p, "unreadable", e); continue
    b = v.get("boot") or {}
    w = {f"{d['reference']}:{d['step'].split(' ')[0]}{'' if d['step']=='identity' else '-'+d['step'].split('(')[-1].rstrip(')')}": d["observed_ns"] for d in b.get("devices", [])}
    kinds = sorted({f["kind"] for f in v.get("findings", []) if f["severity"] != "info"})
    mem = v.get("memory") or {}
    print(p.split("/")[-2] + "/" + p.split("/")[-1], v.get("verdict"), kinds, "witness", w,
          "rss", mem.get("peak_rss_kib"), "log_kept", b.get("log_kept_bytes"), "insn", (v.get("machine") or {}).get("instructions"))
