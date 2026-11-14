#!/usr/bin/env python3
"""AET-WO-4300 spike S3: does a CDC-ACM firmware enumerate on the H7's OTG model in Renode 1.17.0?

Third-party firmware: Antmicro's Zephyr `shell-cdc` ELF for nucleo_h753zi, named by Renode's
own test `tests/platforms/nucleo_h753zi.robot` ("Should Communicate Over USB"), downloaded on the
host to /cache/thirdparty (not committed; URL, size, SHA-1 and SHA-256 in FEEDBACK).

The run copies that test's steps into the CLI's config composition (drive.py's CONFIG, virtual
timestamps as a labelled deviation): `platforms/boards/nucleo_h753zi.repl`, `USB.USB_UART @ sysbus`,
`usb_uart Register usb2 0`, the host's bytes to a file backend, one carriage return written to the
host side after 3 s, then `demo board` and a carriage return after 4 s.

Usage: usb.py <out.jsonl>
"""
import json, os, subprocess, sys, tempfile, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from drive import CONFIG, body  # noqa: E402

ELF = "/cache/thirdparty/zephyr-shell-cdc.elf"
OUT = "/cache/raw/usb.bin"


def write(text):
    return [f"sysbus.usb_uart WriteChar {b:#04x}" for b in text.encode()]


def main():
    if os.path.exists(OUT):
        os.remove(OUT)
    cmds = ["emulation SetSeed 2100", "mach create", "machine LoadPlatformDescription @platforms/boards/nucleo_h753zi.repl",
            f"sysbus LoadELF @{ELF}",
            'machine LoadPlatformDescriptionFromString "usb_uart: USB.USB_UART @ sysbus"',
            "usb_uart Register usb2 0",
            f"sysbus.usb_uart CreateFileBackend @{OUT} true",
            "logLevel 0 sysbus.usb2", "logLevel 0 sysbus.usb_uart",
            'emulation RunFor "3.0"'] + write("\r") + ['emulation RunFor "1.0"'] + write("demo board\r") + [
            'emulation RunFor "1.0"',
            'echo "AETHELGARD-BOOT-BEGIN"', "sysbus ReadDoubleWord 0x40080800", "sysbus ReadDoubleWord 0x40080804",
            "sysbus ReadDoubleWord 0x40080014", 'echo "AETHELGARD-BOOT-END"', "quit"]
    home = tempfile.mkdtemp(prefix="aet4300-home-", dir="/cache/homes")
    os.makedirs(f"{home}/.config/renode")
    cfg = f"{home}/renode.config"
    open(cfg, "w").write(CONFIG.format(home=home, ts="Virtual"))
    env = {"HOME": home, "PATH": "/usr/local/bin:/usr/bin:/bin", "DOTNET_EnableDiagnostics": "0",
           "DOTNET_EnableWriteXorExecute": "0"}
    t0 = time.monotonic()
    p = subprocess.run(["renode", "--disable-gui", "--console", "--plain", "--config", cfg, "-e", "; ".join(cmds)],
                       stdin=subprocess.DEVNULL, capture_output=True, env=env, timeout=900)
    out = p.stdout.decode("utf-8", "replace")
    open("/cache/raw/usb.stdout", "w").write(out[-400_000:])
    lines = [l.strip("\r") for l in out.split("\n")]
    logs = [body(l) for l in lines if body(l) is not None]
    got = open(OUT, "rb").read() if os.path.exists(OUT) else b""
    b = lines.index("AETHELGARD-BOOT-BEGIN") if "AETHELGARD-BOOT-BEGIN" in lines else -1
    row = {"case": "usb-zephyr-cdc", "exit": p.returncode, "wall_s": round(time.monotonic() - t0, 3),
           "host_bytes": len(got), "host_text": got.decode("utf-8", "replace")[:2000],
           "reads": lines[b + 1:b + 4] if b >= 0 else None,
           "usb_log": [l for l in logs if "usb" in l][:80], "warnings": [l for l in logs if "[WARNING]" in l][:30],
           "errors": [l for l in logs if "[ERROR]" in l][:30], "log_lines": len(logs)}
    with open(sys.argv[1], "a") as f:
        f.write(json.dumps(row) + "\n")


if __name__ == "__main__":
    main()
