# AET-WO-4400f spike helpers, loaded into Renode's monitor with `include @.../clock.py`.
# Reflection reads private state (observation) and, for candidate (b), emulates Renode changes
# no release carries; each use is a labelled deviation in FEEDBACK.
import System
from System.Reflection import BindingFlags
NP = BindingFlags.NonPublic | BindingFlags.Instance
TIMERS = (1, 2, 3, 4, 5, 8, 12, 13, 14, 15, 16, 17)


def _p(name):
    return monitor.Machine["sysbus." + name]


def _channels(t):
    return t.GetType().GetField("channels", NP).GetValue(t)


def chan_freq(timer, i):
    """Observation: channel i's compare LimitTimer frequency (STM32_Timer.cs L104)."""
    ch = _channels(_p(timer))[i]
    print(hex(int(ch.GetType().GetField("Timer").GetValue(ch).Frequency)))


def propagate():
    """(b): what upstream f2ac99657 / b14469bba add, STM32H7_RCC.PropagateClock (L1501)."""
    r = _p("rcc")
    r.GetType().GetMethod("PropagateClock", NP).Invoke(r, None)


def retune():
    """(b'): each timer's capture/compare channels at the timer's own frequency (no upstream commit)."""
    for n in TIMERS:
        t = _p("timer%d" % n)
        f = t.Frequency
        for ch in _channels(t):
            lt = ch.GetType().GetField("Timer").GetValue(ch)
            if lt.Frequency != f:
                lt.Frequency = f


def follow():
    """(b) then (b'), the hook body."""
    propagate()
    retune()
