"""AET-WO-1213 measurement: every serial port in every tracked JSON file,
its role and declared baud, and whether map_baudrate(b // 1000) == b.

map_baudrate is transcribed from AP_SerialManager.cpp:737-770 at Copter-4.7.2
(sha256 fe7cfce0...56d9b). Run from the repository root:
    git ls-files -z '*.json' | python3 -I spike/1213/measure.py
"""
import json
import sys

CODES = {1: 1200, 2: 2400, 4: 4800, 9: 9600, 19: 19200, 38: 38400, 57: 57600,
         100: 100000, 111: 111100, 115: 115200, 230: 230400, 256: 256000,
         460: 460800, 500: 500000, 921: 921600, 1500: 1500000, 2000: 2000000}


def map_baudrate(rate):
    if rate <= 0:
        rate = 57
    if rate in CODES:
        return CODES[rate]
    if rate > 2000:
        return rate
    return rate * 1000


def ports(node, path):
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "serial_ports" and isinstance(v, list):
                # a specification's: a list in document order
                for i, p in enumerate(v):
                    yield f"{path}.serial_ports[{i}]", p
            elif k == "serial_ports" and isinstance(v, dict):
                # a solved topology's: keyed by label
                for label, p in v.items():
                    yield f"{path}.serial_ports.{label}", p
            else:
                yield from ports(v, f"{path}.{k}")
    elif isinstance(node, list):
        for i, v in enumerate(node):
            yield from ports(v, f"{path}[{i}]")


def main():
    names = [n for n in sys.stdin.read().split("\0") if n]
    rows, unread = [], []
    for name in names:
        try:
            with open(name, encoding="utf-8") as f:
                doc = json.load(f)
        except (OSError, ValueError) as e:
            unread.append((name, type(e).__name__))
            continue
        for where, p in ports(doc, "$"):
            if not isinstance(p, dict):
                rows.append((name, where, "?", None, "not an object"))
                continue
            role = p.get("role", "?")
            baud = (p.get("framing") or {}).get("baud", p.get("baud"))
            if not isinstance(baud, int):
                rows.append((name, where, role, baud, "no integer baud"))
                continue
            zero = map_baudrate(baud // 1000)
            n = map_baudrate(baud)
            rows.append((name, where, role, baud,
                         f"0_BAUD:{'ok' if zero == baud else f'-> {zero}'} "
                         f"n_BAUD:{'ok' if n == baud else f'-> {n}'}"))
    for r in rows:
        print("\t".join(str(x) for x in r))
    print(f"# files {len(names)}, unread (not JSON) {len(unread)}, ports {len(rows)}")
    consoles = [r for r in rows if r[2] == "console"]
    bad = [r for r in consoles if "0_BAUD:ok" not in r[4]]
    print(f"# console ports {len(consoles)}, console bauds {sorted({r[3] for r in consoles})}, "
          f"refused {len(bad)}")
    roles = {}
    for r in rows:
        roles.setdefault(r[2], set()).add(r[3])
    for role, bauds in sorted(roles.items(), key=lambda x: str(x[0])):
        print(f"# role {role}: bauds {sorted(bauds, key=str)}")
    for name, kind in unread:
        print(f"# unread {name}: {kind}")


main()
