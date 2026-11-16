#!/bin/bash
# probe2.sh: (1) injection scheduled from the boot-word hook (static injection off), echo timed by TDR/RDR hooks;
# (2) a DShot droop under a 100 ms window: does the CEN hook fire again after machine Reset, and does a pending
# ScheduleAction from the first boot fire in the second?
set -u
f=spike/2302/fixtures/sentinel-02-shaped.json; n=sentinel-02-shaped
US2="0x6f 0xb0 0x0b 0xe1 0x8e 0x51 0xfc 0x23"; US3="0x5f 0xc9 0x08 0x61 0x1b 0x58 0xd1 0xe4"
S=""; k=0; for b in $US2; do S="$S m.ScheduleAction(TimeInterval.FromMicroseconds($((100 + 200*k))), lambda t, c='sysbus.usart2 WriteChar $b': monitor.Parse(c));"; k=$((k+1)); done
k=0; for b in $US3; do S="$S m.ScheduleAction(TimeInterval.FromMicroseconds($((100 + 100*k))), lambda t, c='sysbus.usart3 WriteChar $b': monitor.Parse(c));"; k=$((k+1)); done
INJ="sysbus AddWatchpointHook 0x20000000 DoubleWord Write \"if value == 0x2100B007: from Antmicro.Renode.Time import TimeInterval; m = self.Machine; self.Log(LogLevel.Warning, 'AETBWI %d' % cpu.ExecutedInstructions); $S\""
REST=$(tr ';' '\n' < /cache/m/$n.hooks | sed -n '/^$/d;p' | grep -v "0x20000000 " | paste -sd';' | sed 's/;/;;/g')
echo "== (1) hook-scheduled injection"
AET2302_TAG=hinj AET2302_NO_INJECT=1 AET2302_WINDOW_NS=100000000 AET2302_WALL_S=60 AET2302_EXTRA="$INJ;;$REST" /cache/m.sh $f $n 100ms
grep -E "AETBWI|xception|rror\]" /cache/m/$n/hinj.log | cut -c1-160 | head -5
python3 -I /repo/spike/2302/phases.py /cache/m/$n/hinj.log /cache/m/$n/hinj.verdict.json 2>&1 | python3 -I -c "import json,sys; d=json.load(sys.stdin); print(json.dumps(d['serial'])); print('boot word', d['boot_word_ns'], 'findings', d['findings'])" 2>&1 | tail -3
grep -E "AETR_USART[23]|AETX_USART[23]" /cache/m/$n/hinj.log | awk '{print $4, $7*10}' | tail -40 > /cache/m/$n/hinj-echo.txt
echo "== (2) DShot droop, 100 ms window, CEN and boot-word hooks"
H="sysbus AddWatchpointHook 0x40010000 DoubleWord Write \"if (value & 1): from Antmicro.Renode.Time import TimeInterval; m = self.Machine; self.Log(LogLevel.Warning, 'AETCEN %d' % cpu.ExecutedInstructions); m.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t: self.Log(LogLevel.Warning, 'AETSCHED %d' % cpu.ExecutedInstructions))\";;sysbus AddWatchpointHook 0x20000000 DoubleWord Write \"self.Log(LogLevel.Warning, 'AETBW %x %d' % (value, cpu.ExecutedInstructions))\""
rm -rf /cache/m/arch/droop
AET2302_DUMP=/cache/m/motors-dshot600/droop.log AET2302_WINDOW_NS=100000000 AET2302_NO_OBS=1 AET2302_WALL_S=60 AET2302_EXTRA="$H" /cache/target-spike/debug/aethelgard campaign --scenarios scenarios/motors/dshot600-droop.json --firmware /cache/m/motors-dshot600/fw.elf --archive /cache/m/arch/droop > /cache/m/arch/droop.out 2>&1; echo "campaign rc=$?"; tail -1 /cache/m/arch/droop.out | cut -c1-300
grep -E "AETCEN|AETSCHED|AETBW|RELEASE|PROBE-BEGIN|machine Reset" /cache/m/motors-dshot600/droop.log | grep -v "^(monitor)" | cut -c1-140 | head -20
