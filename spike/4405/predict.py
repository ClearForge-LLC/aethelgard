# predict.py FILE...: the SERIALn rows AP_SerialManager::var_info should hold,
# predicted from a board's hwdef.h (or its hwdef.dat, whose defines ArduPilot
# copies verbatim) by the source rules of Copter-4.7.2 (fb773567), before any
# ELF is decoded. One line per board: NAME n_PROTOCOL=v n_BAUD=v->baud n_OPTIONS=v ...
#   - rows exist for n where HAL_HAVE_SERIALn_PARAMS is 1 (.h) or n < len(SERIAL_ORDER) (.dat)
#     (chibios_hwdef.py:1700-1709);
#   - 0_BAUD = DEFAULT_SERIAL0_BAUD/1000, C integer division, else 115200/1000
#     (AP_SerialManager.cpp:172, AP_SerialManager_config.h:78-81); 0_PROTOCOL = 2 (:180);
#   - n_PROTOCOL, n_BAUD, n_OPTIONS: the define, else the #ifndef default (:47-162);
#     7_OPTIONS and 8_OPTIONS are a literal 0 (:357, :376);
#   - the baud ArduPilot opens at is map_baudrate(value) (:737-770).
# Values are read as C integer constants or SerialProtocol names; anything else prints '?'.
import os, re, sys
PROTO = {"SerialProtocol_None": -1, "SerialProtocol_MAVLink": 1, "SerialProtocol_MAVLink2": 2,
         "SerialProtocol_GPS": 5, "SerialProtocol_GPS2": 6, "SerialProtocol_RCIN": 23}
DPROTO = {1: 2, 2: 2, 3: 5, 4: 5}
DBAUD = {1: 57, 2: 57, 3: 230, 4: 230}
def cint(s):
    s = s.strip().rstrip("uUlL")
    try:
        if s.lower().startswith("0x"): return int(s, 16)
        if len(s) > 1 and s.startswith("0"): return int(s, 8)
        return int(s)
    except ValueError:
        return None
def mapb(r):
    r = 57 if r <= 0 else r
    t = {1: 1200, 2: 2400, 4: 4800, 9: 9600, 19: 19200, 38: 38400, 57: 57600, 100: 100000, 111: 111100,
         115: 115200, 230: 230400, 256: 256000, 460: 460800, 500: 500000, 921: 921600, 1500: 1500000, 2000: 2000000}
    return t.get(r, r if r > 2000 else r * 1000)
for path in sys.argv[1:]:
    text = open(path).read()
    d, n_ports = {}, 0
    for line in text.splitlines():
        line = line.split("#", 1)[0] if path.endswith(".dat") else line
        m = re.match(r"\s*#?\s*define\s+(\w+)\s+(.*?)\s*$", line)
        if m: d[m.group(1)] = m.group(2)
        m = re.match(r"\s*SERIAL_ORDER\s+(.*)$", line)
        if m: n_ports = len(m.group(1).split())
    if path.endswith(".h"):
        ports = [n for n in range(10) if d.get(f"HAL_HAVE_SERIAL{n}_PARAMS") == "1"]
    else:
        ports = list(range(n_ports))
    name = os.path.basename(os.path.dirname(path)) if path.endswith(".h") else os.path.basename(path).split(".")[0]
    out = []
    for n in ports:
        if n == 0:
            b0 = cint(d.get("DEFAULT_SERIAL0_BAUD", "115200"))
            out.append("0_PROTOCOL=2")
            out.append(f"0_BAUD={b0 // 1000}->{mapb(b0 // 1000)}" if b0 is not None else "0_BAUD=?")
            continue
        p = d.get(f"DEFAULT_SERIAL{n}_PROTOCOL")
        p = DPROTO.get(n, -1) if p is None else PROTO.get(p, cint(p))
        b = d.get(f"DEFAULT_SERIAL{n}_BAUD")
        b = DBAUD.get(n, 57) if b is None else cint(b)
        o = 0 if n in (7, 8) else cint(d.get(f"DEFAULT_SERIAL{n}_OPTIONS", "0"))
        out += [f"{n}_PROTOCOL={'?' if p is None else p}",
                f"{n}_BAUD={b}->{mapb(b)}" if b is not None else f"{n}_BAUD=?",
                f"{n}_OPTIONS={'?' if o is None else o}"]
    print(name, " ".join(out))
