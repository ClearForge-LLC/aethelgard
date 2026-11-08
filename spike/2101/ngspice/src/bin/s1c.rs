//! S1 route c / S2 route c. Usage:
//!   s1c s1 <mode> <N> <out.tsv>       N transients, R=2.2k, byte=i%256, ack 0
//!   s1c s2 <mode> <R> <byte-hex> <ack> [trace-file]   one transient + rise measurement
use ng_probe::pattern::{ByteWindow, Mode, circuit};
use ng_probe::sha256::sha256_hex;
use ng_probe::Spice;
use std::time::Instant;

fn mode(s: &str) -> Mode {
    match s { "none" => Mode::None, "clamp" => Mode::Clamp, "bkpt" => Mode::Bkpt, "bkpt1" => Mode::Bkpt1, _ => panic!("mode") }
}

fn run_one(sp: &mut Spice<ByteWindow>, m: Mode, first: bool) -> (Vec<f64>, Vec<f64>) {
    sp.hooks().begin();
    sp.command("destroy all");
    if m == Mode::Bkpt || (m == Mode::Bkpt1 && first) {
        for e in ByteWindow::edges() { sp.set_bkpt(e); }
    }
    let rc = sp.command("run");
    assert_eq!(rc, 0, "run rc");
    let t = sp.vec("time").expect("time");
    let v = sp.vec("line").expect("line");
    (t, v)
}

fn bytes(t: &[f64], v: &[f64]) -> Vec<u8> {
    let mut b = Vec::with_capacity(16 * t.len());
    for x in t { b.extend_from_slice(&x.to_le_bytes()); }
    for x in v { b.extend_from_slice(&x.to_le_bytes()); }
    b
}

/// 30%-70% rise with linear interpolation of the first crossings.
pub fn rise(t: &[f64], v: &[f64]) -> Option<(f64, f64, f64)> {
    let cross = |lvl: f64, from: usize| -> Option<(usize, f64)> {
        (from..v.len() - 1).find(|&k| v[k] < lvl && v[k + 1] >= lvl)
            .map(|k| (k, t[k] + (lvl - v[k]) * (t[k + 1] - t[k]) / (v[k + 1] - v[k])))
    };
    let (k, a) = cross(0.99, 0)?;
    let (_, b) = cross(2.31, k)?;
    Some((b - a, a, b))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let m = mode(&a[2]);
    let r: f64 = if a[1] == "s1" { 2200.0 } else { a[3].parse().unwrap() };
    let t0 = Instant::now();
    let mut sp = Spice::init(ByteWindow::new(m)).expect("init");
    sp.circ(&circuit(r)).expect("circ");
    let startup = t0.elapsed();
    sp.take_log();
    if a[1] == "s1" {
        let n: usize = a[3].parse().unwrap();
        let mut f = String::new();
        f += &format!("# startup_ns\t{}\n", startup.as_nanos());
        for i in 0..n {
            sp.hooks().set_byte((i % 256) as u8, 0);
            let t1 = Instant::now();
            let (t, v) = run_one(&mut sp, m, i == 0);
            let ns = t1.elapsed().as_nanos();
            let h = sha256_hex(&bytes(&t, &v));
            let w = sp.hooks();
            f += &format!("{i}\t{ns}\t{h}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\n", t.len(), w.vsrc_calls, w.sync_calls, w.sync_redo_flag, w.time_regress, w.clamps, w.loc_hist);
        }
        std::fs::write(&a[4], f).unwrap();
    } else {
        let byte = u8::from_str_radix(&a[4], 16).unwrap();
        let ack: u8 = a[5].parse().unwrap();
        sp.hooks().set_byte(byte, ack);
        if a.len() > 6 { sp.hooks().trace_cap = 60; }
        let (t, v) = run_one(&mut sp, m, true);
        let peak = v.iter().cloned().fold(f64::MIN, f64::max);
        let w = sp.hooks();
        println!("mode={:?} R={r} byte={byte:02x} ack={ack} points={} vsrc_calls={} sync_calls={} redo_flag={} time_regress={} clamps={} loc_hist={:?}", m, t.len(), w.vsrc_calls, w.sync_calls, w.sync_redo_flag, w.time_regress, w.clamps, w.loc_hist);
        println!("peak_v={peak:.6}");
        match rise(&t, &v) {
            Some((d, x, y)) => println!("vector_rise_ns={:.4} t30={:.6e} t70={:.6e}", d * 1e9, x, y),
            None => println!("vector_rise=none"),
        }
        let tr = w.trace.clone();
        if a.len() > 6 {
            let mut s = String::from("# time delta old redo loc\n");
            for x in tr { s += &format!("{:.9e} {:.9e} {:.9e} {} {}\n", x.0, x.1, x.2, x.3, x.4); }
            std::fs::write(&a[6], s).unwrap();
        }
        sp.take_log();
        let rc = sp.command("meas tran trise TRIG v(line) VAL=0.99 RISE=1 TARG v(line) VAL=2.31 RISE=1");
        println!("meas rc={rc}");
        for l in sp.take_log() { println!("log: {}", l.trim_end()); }
    }
}
