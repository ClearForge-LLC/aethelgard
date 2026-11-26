"""AET-WO-4412 prototype: USART1 TX from a replay console, decoded as MAVLink 2 with the stamp of each message's last
byte. Run in the ArduPilot image (pymavlink). Usage: python -I mav.py console.txt.gz
Prints HEARTBEAT base_mode changes, COMMAND_ACKs, STATUSTEXTs and SERVO_OUTPUT_RAW changes, with virtual time."""
import gzip, re, sys
from pymavlink.dialects.v20 import ardupilotmega as mav
TX = re.compile(rb'^\[(\d\d):(\d\d):(\d\d\.\d{9})\] \[\w+\] usart1: \[cpu: 0x[0-9A-F]+\] Write\w+ to 0x28 \(\w+\), value 0x([0-9A-F]+)')
m = mav.MAVLink(None); m.robust_parsing = True
last_mode = None; last_servo = None; n = 0
for line in gzip.open(sys.argv[1]):
    x = TX.match(line)
    if not x: continue
    t = int(x.group(1)) * 3600 + int(x.group(2)) * 60 + float(x.group(3))
    msg = m.parse_char(bytes([int(x.group(4), 16) & 0xFF]))
    if msg is None: continue
    n += 1
    ty = msg.get_type()
    if ty == 'HEARTBEAT' and msg.base_mode != last_mode:
        print("%.6f HEARTBEAT base_mode %d custom_mode %d system_status %d" % (t, msg.base_mode, msg.custom_mode, msg.system_status)); last_mode = msg.base_mode
    elif ty == 'COMMAND_ACK':
        print("%.6f COMMAND_ACK command %d result %d" % (t, msg.command, msg.result))
    elif ty == 'STATUSTEXT':
        print("%.6f STATUSTEXT %s" % (t, msg.text))
    elif ty == 'SERVO_OUTPUT_RAW':
        s = (msg.servo1_raw, msg.servo2_raw, msg.servo3_raw, msg.servo4_raw)
        if s != last_servo:
            print("%.6f SERVO_OUTPUT_RAW %s" % (t, s)); last_servo = s
print("messages", n)
