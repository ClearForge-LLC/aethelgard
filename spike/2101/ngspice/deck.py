"""Shared deck builder for S1/S2 routes a and b (identical circuit in all routes).
Circuit: VDD=3.3 V, pull-up R, C=100 pF, open-drain driver = voltage-controlled switch.
Control source ctl: 3.3 V = driver ON (bit 0, line pulled low), 0 V = released (bit 1).
Bit time 2.5 us (400 kHz), 9 bits (8 data MSB first + ACK), 22.5 us. Edges 1 ns ramps.
"""
BIT = 2.5e-6
EDGE = 1e-9
VON = 3.3
TSTOP = 22.5e-6
TRAN = ".tran 1n 22.5u 0 5n"
OPTS = ".options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap"


def bits(byte, ack):
    b = [(byte >> (7 - k)) & 1 for k in range(8)] + [ack]
    return b


def levels(byte, ack):
    return [0.0 if b else VON for b in bits(byte, ack)]


def pwl_points(byte, ack):
    lv = levels(byte, ack)
    pts = [(0.0, lv[0])]
    for k in range(1, 9):
        if lv[k] != lv[k - 1]:
            t = k * BIT
            pts.append((t, lv[k - 1]))
            pts.append((t + EDGE, lv[k]))
    pts.append((TSTOP, lv[8]))
    return pts


def pwl_str(byte, ack):
    return " ".join("%.12g %.12g" % p for p in pwl_points(byte, ack))


def circuit_lines(r, byte, ack):
    return [
        "* one I2C line",
        "Vdd vdd 0 dc 3.3",
        "R1 vdd line %g" % r,
        "C1 line 0 100p",
        "S1 line 0 ctl 0 swm",
        ".model swm sw vt=1.65 vh=0 ron=10 roff=1e12",
        "Vctl ctl 0 pwl(%s)" % pwl_str(byte, ack),
        OPTS,
        TRAN,
    ]

MEAS = "meas tran trise TRIG v(line) VAL=0.99 RISE=1 TARG v(line) VAL=2.31 RISE=1"
