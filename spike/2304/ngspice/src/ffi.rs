//! Hand-declared externs from the pinned header
//! ~/.cache/aet-2101/src/sharedspice.h (ngspice-44.2 tag, sha256 4041c4e2...).
//! Line numbers below are that file's.
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::sync::Mutex;

#[repr(C)]
pub struct NgComplex {
    pub re: c_double,
    pub im: c_double,
}
/// header L176-183 `vector_info`
#[repr(C)]
pub struct VectorInfo {
    pub v_name: *mut c_char,
    pub v_type: c_int,
    pub v_flags: i16,
    pub v_realdata: *mut c_double,
    pub v_compdata: *mut NgComplex,
    pub v_length: c_int,
}

// header L247 SendChar, L254 SendStat, L261 ControlledExit (NG_BOOL = C bool, L171)
pub type SendChar = extern "C" fn(*mut c_char, c_int, *mut c_void) -> c_int;
pub type SendStat = extern "C" fn(*mut c_char, c_int, *mut c_void) -> c_int;
pub type ControlledExit = extern "C" fn(c_int, bool, bool, c_int, *mut c_void) -> c_int;
// header L299 GetVSRCData, L309 GetISRCData, L319 GetSyncData (7 params, not the manual's 5)
pub type GetVSRCData = extern "C" fn(*mut c_double, c_double, *mut c_char, c_int, *mut c_void) -> c_int;
pub type GetISRCData = extern "C" fn(*mut c_double, c_double, *mut c_char, c_int, *mut c_void) -> c_int;
pub type GetSyncData = extern "C" fn(c_double, *mut c_double, c_double, c_int, c_int, c_int, *mut c_void) -> c_int;

unsafe extern "C" {
    // header L372-373
    fn ngSpice_Init(
        printfcn: Option<SendChar>,
        statfcn: Option<SendStat>,
        ngexit: Option<ControlledExit>,
        sdata: *const c_void,
        sinitdata: *const c_void,
        bgtrun: *const c_void,
        user: *mut c_void,
    ) -> c_int;
    // header L386
    fn ngSpice_Init_Sync(
        v: Option<GetVSRCData>,
        i: Option<GetISRCData>,
        s: Option<GetSyncData>,
        ident: *mut c_int,
        user: *mut c_void,
    ) -> c_int;
    // header L391
    fn ngSpice_Command(cmd: *mut c_char) -> c_int;
    // header L395
    fn ngGet_Vec_Info(name: *mut c_char) -> *mut VectorInfo;
    // header L427 (array of C strings, NULL-terminated)
    fn ngSpice_Circ(circ: *mut *mut c_char) -> c_int;
    // header L452
    fn ngSpice_SetBkpt(t: c_double) -> bool;
}

/// Everything the lock-step needs from the host, called on ngspice's thread inside `run`.
pub trait Hooks {
    /// A `V... external` source asks for its value (GetVSRCData, header L299).
    fn vsrc(&mut self, time: f64, name: &str) -> f64;
    /// Per accepted/rejected step (GetSyncData, header L319). May rewrite `*delta`;
    /// return 1 to redo the step, 0 to go on.
    fn sync(&mut self, time: f64, delta: &mut f64, old_delta: f64, redo: i32, loc: i32) -> i32;
}

static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

extern "C" fn send_char<H: Hooks>(s: *mut c_char, _id: c_int, _u: *mut c_void) -> c_int {
    // SAFETY: ngspice passes a NUL-terminated string valid for the call.
    let st = unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned();
    if let Ok(mut l) = LOG.lock() {
        l.push(st);
    }
    0
}
extern "C" fn send_stat(_s: *mut c_char, _id: c_int, _u: *mut c_void) -> c_int {
    0
}
extern "C" fn ctrl_exit(_s: c_int, _unload: bool, _quit: bool, _id: c_int, _u: *mut c_void) -> c_int {
    0
}
extern "C" fn vsrc_cb<H: Hooks>(val: *mut c_double, t: c_double, name: *mut c_char, _id: c_int, u: *mut c_void) -> c_int {
    // SAFETY: `u` is the `*mut H` handed to Init_Sync; `name` is NUL-terminated; ngspice is
    // single-threaded for a non-bg_ `run`, so no aliasing with the owner's `&mut`.
    unsafe {
        let h = &mut *(u as *mut H);
        let n = CStr::from_ptr(name).to_string_lossy();
        *val = h.vsrc(t, &n);
    }
    0
}
extern "C" fn isrc_cb(_v: *mut c_double, _t: c_double, _n: *mut c_char, _id: c_int, _u: *mut c_void) -> c_int {
    0
}
extern "C" fn sync_cb<H: Hooks>(t: c_double, d: *mut c_double, old: c_double, redo: c_int, _id: c_int, loc: c_int, u: *mut c_void) -> c_int {
    // SAFETY: as vsrc_cb; `d` points at ckt->CKTdelta.
    unsafe {
        let h = &mut *(u as *mut H);
        h.sync(t, &mut *d, old, redo, loc)
    }
}

pub struct Spice<H: Hooks> {
    hooks: *mut H,
    _ident: Box<c_int>,
}

impl<H: Hooks> Spice<H> {
    /// `ngSpice_Init` THEN `ngSpice_Init_Sync` (the order the header requires).
    pub fn init(h: H) -> Result<Self, String> {
        let hooks = Box::into_raw(Box::new(h));
        let mut ident = Box::new(0 as c_int);
        // SAFETY: plain FFI calls; callbacks are `extern "C"` with the header's signatures.
        unsafe {
            let r = ngSpice_Init(
                Some(send_char::<H>),
                Some(send_stat),
                Some(ctrl_exit),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                hooks as *mut c_void,
            );
            if r != 0 {
                return Err(format!("ngSpice_Init -> {r}"));
            }
            let r = ngSpice_Init_Sync(
                Some(vsrc_cb::<H>),
                Some(isrc_cb),
                Some(sync_cb::<H>),
                &mut *ident,
                hooks as *mut c_void,
            );
            if r != 0 {
                return Err(format!("ngSpice_Init_Sync -> {r}"));
            }
        }
        Ok(Spice { hooks, _ident: ident })
    }
    /// Host state; only call between commands (never while a `run` is in flight).
    pub fn hooks(&mut self) -> &mut H {
        // SAFETY: allocated in init, freed in Drop; no run in flight (caller contract).
        unsafe { &mut *self.hooks }
    }
    pub fn circ(&mut self, lines: &[String]) -> Result<(), String> {
        let cs: Vec<CString> = lines.iter().map(|l| CString::new(l.as_str()).unwrap()).collect();
        let mut p: Vec<*mut c_char> = cs.iter().map(|c| c.as_ptr() as *mut c_char).collect();
        p.push(std::ptr::null_mut());
        // SAFETY: NULL-terminated array of NUL-terminated strings, alive for the call.
        let r = unsafe { ngSpice_Circ(p.as_mut_ptr()) };
        if r == 0 { Ok(()) } else { Err(format!("ngSpice_Circ -> {r}")) }
    }
    /// Synchronous command (never `bg_`): blocks, callbacks run on this thread.
    pub fn command(&mut self, cmd: &str) -> i32 {
        let c = CString::new(cmd).unwrap();
        // SAFETY: NUL-terminated string alive for the call.
        unsafe { ngSpice_Command(c.as_ptr() as *mut c_char) }
    }
    pub fn set_bkpt(&mut self, t: f64) -> bool {
        // SAFETY: plain FFI.
        unsafe { ngSpice_SetBkpt(t) }
    }
    /// Copy of a real vector of the current plot.
    pub fn vec(&mut self, name: &str) -> Option<Vec<f64>> {
        let c = CString::new(name).unwrap();
        // SAFETY: the returned struct and data are valid until the next plot change; copied now.
        unsafe {
            let p = ngGet_Vec_Info(c.as_ptr() as *mut c_char);
            if p.is_null() || (*p).v_realdata.is_null() {
                return None;
            }
            Some(std::slice::from_raw_parts((*p).v_realdata, (*p).v_length as usize).to_vec())
        }
    }
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut *LOG.lock().unwrap())
    }
}

/// The length and newest value of a real vector of the current plot, read
/// while a `run` is in flight (from a hook on ngspice's own thread). Added by
/// the PM for the S4 inversion-of-control lock-step.
pub fn vec_last(name: &str) -> Option<(usize, f64)> {
    let c = CString::new(name).ok()?;
    // SAFETY: called on ngspice's thread (inside a hook) or between commands;
    // the struct and its data are valid for this call; one element is read.
    unsafe {
        let p = ngGet_Vec_Info(c.as_ptr() as *mut c_char);
        if p.is_null() || (*p).v_realdata.is_null() || (*p).v_length < 1 {
            return None;
        }
        let n = (*p).v_length as usize;
        Some((n, *(*p).v_realdata.add(n - 1)))
    }
}

impl<H: Hooks> Drop for Spice<H> {
    fn drop(&mut self) {
        // SAFETY: allocated by Box::into_raw in init, dropped once.
        unsafe { drop(Box::from_raw(self.hooks)) }
    }
}
