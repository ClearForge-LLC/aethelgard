#!/bin/bash
# AET-WO-4412 prototype: a capture dir for sh/replay3.sh, made from a kit capture.
# mk.sh BASE NAME aet|aet-nopre|stock   (aet-nopre: timer1 built with preload: false, the stock CCR rule; inc: the models compiled, the stock platform)   -> /cache/proto/cap/NAME (host: ~/.cache/aet-4412/proto/cap/NAME)
#  aet:   the two models copied into tool/models/, included in -e after the CLI's own models, and
#         the platform's `using` pointed at /cache/proto/platforms/stm32h753-aet.repl (stock chain, timer1/dma1/dma2 retyped). stock: unchanged tool dir.
#  both:  "sysbus.timer1 Stats" (aet only) is printed before the CLI's final "peripherals; quit".
set -eu
P=$HOME/.cache/aet-4412/proto; base=$1; name=$2; kind=$3
W=$HOME/worktrees/aethelgard/4412-models/models/renode
o=$P/cap/$name; rm -rf "$o"; cp -r "$base" "$o"; chmod -R u+w "$o"
if [ "$kind" != stock ]; then
  cp $W/AethelgardSTM32Timer.cs $W/AethelgardSTM32DMA.cs "$o/tool/models/"
  f="$o/tool/platform-booted.repl"; [ "$kind" = inc ] || grep -q '^using "platforms/cpus/stm32h753.repl"$' "$f"
  [ "$kind" = inc ] || sed -i "s#^using \"platforms/cpus/stm32h753.repl\"\$#using \"/cache/proto/platforms/stm32h753-$kind.repl\"#" "$f"
fi
python3 -I - "$o/argv.bin" "$kind" <<'PY'
import sys
p, kind = sys.argv[1], sys.argv[2]
a = open(p, 'rb').read().split(b'\0')
i = a.index(b'-e') + 1
e = a[i].decode()
icm = [x for x in e.split('; ') if x.startswith('include @') and x.endswith('/models/ICM42688P.cs')]
assert len(icm) == 1
d = icm[0][len('include @'):-len('ICM42688P.cs')]
if kind != 'stock':
    e = e.replace(icm[0], icm[0] + '; include @%sAethelgardSTM32Timer.cs; include @%sAethelgardSTM32DMA.cs' % (d, d), 1)
    assert e.endswith('; peripherals; quit')
    if kind not in ('inc', 'dmaonly'):
        e = e[:-len('peripherals; quit')] + 'echo "AET-PROTO-STATS"; sysbus.timer1 Stats; peripherals; quit'
a[i] = e.encode()
open(p, 'wb').write(b'\0'.join(a))
PY
echo "made $o ($kind)"
