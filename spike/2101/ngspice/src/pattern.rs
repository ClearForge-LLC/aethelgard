//! One I2C byte window as the external driver-control source, plus delta clamping.
//! Same circuit and same control waveform as deck.py (routes a/b): 9 bits x 2.5 us,
//! 1 ns edge ramps, 3.3 V = driver ON (bit 0).
use crate::ffi::Hooks;

pub const BIT: f64 = 2.5e-6;
pub const EDGE: f64 = 1e-9;
pub const VON: f64 = 3.3;
pub const TSTOP: f64 = 22.5e-6;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    /// no help: solver may step over edges
    None,
    /// clamp *delta in GetSyncData so no step crosses the next edge
    Clamp,
    /// ngSpice_SetBkpt at every edge before each run (host sets them; see main)
    Bkpt,
    /// SetBkpt only once, before the first run (tests whether breakpoints persist across runs)
    Bkpt1,
}

pub fn circuit(r: f64) -> Vec<String> {
    vec![
        "* one I2C line".into(),
        "Vdd vdd 0 dc 3.3".into(),
        format!("R1 vdd line {r}"),
        "C1 line 0 100p".into(),
        "S1 line 0 ctl 0 swm".into(),
        ".model swm sw vt=1.65 vh=0 ron=10 roff=1e12".into(),
        "Vctl ctl 0 dc 0 external".into(),
        ".options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap".into(),
        ".tran 1n 22.5u 0 5n".into(),
        ".end".into(),
    ]
}

pub struct ByteWindow {
    pub lv: [f64; 9],
    pub mode: Mode,
    // counters, reset by `begin`
    pub vsrc_calls: u64,
    pub sync_calls: u64,
    pub sync_redo_flag: u64,
    pub time_regress: u64,
    pub clamps: u64,
    pub loc_hist: [u64; 8],
    last_t: f64,
    pub trace: Vec<(f64, f64, f64, i32, i32)>,
    pub trace_cap: usize,
}

impl ByteWindow {
    pub fn new(mode: Mode) -> Self {
        let mut w = ByteWindow {
            lv: [0.0; 9], mode, vsrc_calls: 0, sync_calls: 0, sync_redo_flag: 0, time_regress: 0,
            clamps: 0, loc_hist: [0; 8], last_t: -1.0, trace: vec![], trace_cap: 0,
        };
        w.set_byte(0xFF, 1);
        w
    }
    /// Changing the byte is just host state.
    pub fn set_byte(&mut self, byte: u8, ack: u8) {
        for k in 0..8 {
            self.lv[k] = if (byte >> (7 - k)) & 1 == 1 { 0.0 } else { VON };
        }
        self.lv[8] = if ack == 1 { 0.0 } else { VON };
    }
    pub fn begin(&mut self) {
        self.vsrc_calls = 0; self.sync_calls = 0; self.sync_redo_flag = 0; self.time_regress = 0;
        self.clamps = 0; self.loc_hist = [0; 8]; self.last_t = -1.0; self.trace.clear();
    }
    /// The same PWL as deck.py: level k during bit k; 1 ns ramp from the previous level at each change.
    pub fn value(&self, t: f64) -> f64 {
        let k = ((t / BIT).floor() as i64).clamp(0, 8) as usize;
        let tk = k as f64 * BIT;
        if k > 0 && self.lv[k] != self.lv[k - 1] && t < tk + EDGE {
            let f = (t - tk) / EDGE;
            return self.lv[k - 1] + (self.lv[k] - self.lv[k - 1]) * f.max(0.0);
        }
        self.lv[k]
    }
    pub fn edges() -> impl Iterator<Item = f64> {
        (1..9).flat_map(|k| [k as f64 * BIT, k as f64 * BIT + EDGE])
    }
    fn next_edge(t: f64) -> Option<f64> {
        Self::edges().find(|e| *e > t + 1e-13)
    }
}

impl Hooks for ByteWindow {
    fn vsrc(&mut self, time: f64, _name: &str) -> f64 {
        self.vsrc_calls += 1;
        self.value(time)
    }
    fn sync(&mut self, time: f64, delta: &mut f64, old: f64, redo: i32, loc: i32) -> i32 {
        self.sync_calls += 1;
        if redo != 0 { self.sync_redo_flag += 1; }
        if time < self.last_t - 1e-18 { self.time_regress += 1; }
        self.last_t = time;
        if (0..8).contains(&loc) { self.loc_hist[loc as usize] += 1; }
        if self.trace.len() < self.trace_cap { self.trace.push((time, *delta, old, redo, loc)); }
        if self.mode == Mode::Clamp {
            if let Some(e) = Self::next_edge(time) {
                if time + *delta > e {
                    *delta = e - time;
                    self.clamps += 1;
                }
            }
        }
        0
    }
}
