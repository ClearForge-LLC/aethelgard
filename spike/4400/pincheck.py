"""The pinout check (INV-07), a spike probe.

Expected values come only from the SolvedTopology. Observed values come from what
ArduPilot's own chibios_hwdef.py computed with its own AF table:
  (1) the bytes of `gpio_default_config` in the built ELF, which board.c's
      gpio_init() writes to MODER/OTYPER/OSPEEDR/PUPDR/ODR/AFRL/AFRH at start-up;
  (2) the VAL_GPIOx_MODER/AFRL/AFRH initialisers in hwdef.h, the source of (1).
The check never reads the hwdef.dat the generator emits.

usage: python3 -I pincheck.py TOPOLOGY.json BUILD_DIR TOOLCHAIN_BIN
exit 0 = every topology line matches in both decodes and the decodes agree; 1 otherwise.
"""
import json, re, subprocess, sys

PORTS = "ABCDEFGHIJK"           # STM32_HAS_GPIOA..K on the H753: 11 x 7 words = 308 B
FIELDS = ["moder", "otyper", "ospeedr", "pupdr", "odr", "afrl", "afrh"]
MODES = {0: "input", 1: "output", 2: "alternate", 3: "analog"}


def elf_config(elf, tc):
    nm = subprocess.run([f"{tc}/arm-none-eabi-nm", "-S", elf], capture_output=True, text=True, check=True).stdout
    m = [l.split() for l in nm.splitlines() if l.endswith(" gpio_default_config")]
    if len(m) != 1:
        sys.exit(f"gpio_default_config: {len(m)} symbols in {elf}")
    addr, size = int(m[0][0], 16), int(m[0][1], 16)
    if size != 4 * len(FIELDS) * len(PORTS):
        sys.exit(f"gpio_default_config is {size} B, expected {4 * len(FIELDS) * len(PORTS)}")
    dump = subprocess.run([f"{tc}/arm-none-eabi-objdump", "-s", f"--start-address={addr:#x}",
                           f"--stop-address={addr + size:#x}", elf], capture_output=True, text=True, check=True).stdout
    data = b""
    for line in dump.splitlines():
        # " 813731c 01a80000 00000000 aaaaaaaa 01000000  ascii": address, up to four
        # groups of hex, two spaces, the ASCII column
        mm = re.match(r"^ ([0-9a-f]+) ((?:[0-9a-f]{2,8}(?: |$)){1,4})", line)
        if mm:
            data += bytes.fromhex(mm.group(2).replace(" ", ""))
    if len(data) != size:
        sys.exit(f"read {len(data)} B of gpio_default_config, expected {size}")
    words = [int.from_bytes(data[i:i + 4], "little") for i in range(0, size, 4)]
    return {p: dict(zip(FIELDS, words[k * 7:(k + 1) * 7])) for k, p in enumerate(PORTS)}


def header_config(hwdef_h):
    text = open(hwdef_h).read()
    out = {}
    for p in PORTS:
        regs = {}
        for f in ("MODER", "AFRL", "AFRH"):
            m = re.search(rf"#define VAL_GPIO{p}_{f}\s+\((.*?)\)\n\n", text, re.S)
            if not m:
                sys.exit(f"VAL_GPIO{p}_{f} not found in {hwdef_h}")
            v = 0
            for kind, n in re.findall(r"PIN_MODE_(INPUT|OUTPUT|ALTERNATE|ANALOG)\((\d+)U\)", m.group(1)):
                v |= {"INPUT": 0, "OUTPUT": 1, "ALTERNATE": 2, "ANALOG": 3}[kind] << (int(n) * 2)
            for n, af in re.findall(r"PIN_AFIO_AF\((\d+)U, (\d+)U\)", m.group(1)):
                v |= int(af) << ((int(n) % 8) * 4)
            regs[f.lower()] = v
        out[p] = regs
    return out


def pin_state(cfg, pin):
    port, n = pin[1], int(pin[2:])
    r = cfg[port]
    mode = (r["moder"] >> (2 * n)) & 3
    af = ((r["afrl"] if n < 8 else r["afrh"]) >> ((n % 8) * 4)) & 0xF
    return MODES[mode], af


def check(topo, cfg, source):
    findings = []
    expected_af = {}
    for net, a in sorted(topo["assignments"].items()):
        expected_af[a["pin"]] = net
        mode, af = pin_state(cfg, a["pin"])
        if mode != "alternate":
            findings.append(f"{source}: {net} ({a['signal']}): solved pin {a['pin']} is {mode}, not alternate AF{a['alternate_function']}")
        elif af != a["alternate_function"]:
            findings.append(f"{source}: {net} ({a['signal']}): {a['pin']} is on AF{af}, the topology solved AF{a['alternate_function']}")
    for net, pin in sorted(topo["gpio"].items()):
        mode, _ = pin_state(cfg, pin)
        if mode != "output":
            findings.append(f"{source}: {net}: solved GPIO {pin} is {mode}, not output")
    extra = []
    for p in PORTS:
        for n in range(16):
            pin = f"P{p}{n}"
            mode, af = pin_state(cfg, pin)
            if mode == "alternate" and pin not in expected_af:
                extra.append(f"{pin} AF{af}")
    return findings, extra


def main():
    topo = json.load(open(sys.argv[1]))
    build, tc = sys.argv[2], sys.argv[3]
    elf = elf_config(f"{build}/bin/arducopter", tc)
    hdr = header_config(f"{build}/hwdef.h")
    bad = False
    for name, cfg in (("elf", elf), ("hwdef.h", hdr)):
        findings, extra = check(topo, cfg, name)
        for f in findings:
            print("MISMATCH", f)
        print(f"{name}: {len(topo['assignments'])} AF lines, {len(topo['gpio'])} GPIO lines checked; "
              f"{len(findings)} mismatches; alternate pins outside the topology: {', '.join(extra) or 'none'}")
        bad |= bool(findings)
    agree = all(elf[p][f] == hdr[p][f] for p in PORTS for f in ("moder", "afrl", "afrh"))
    print("elf and hwdef.h MODER/AFRL/AFRH agree:", agree)
    sys.exit(1 if bad or not agree else 0)


main()
