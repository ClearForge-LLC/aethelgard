//! The libngspice 44.2 shared-library interface, declared by hand against the
//! vendored header `vendor/ngspice-44.2/sharedspice.h` (sha256 in
//! `vendor/ngspice-44.2/SOURCE.md`; ruled L3 on PR #66). The line numbers
//! below are that file's, and `header::every_declaration_matches_the_vendored_header`
//! checks each signature against it.
//!
//! This is the only module of the bridge that may contain `unsafe` (`RUL-17`).
//! Every function it exports is safe to call:
//! - nothing but this module calls into the library;
//! - the library is initialised once, and every wrapper refuses before that;
//! - no command runs in ngspice's background thread (`bg_` is refused), so
//!   every callback runs on the caller's thread, inside the call that caused it;
//! - the callbacks receive no pointer to Rust state: they reach the host
//!   through `crate::host`, whose state is behind a `Mutex` that no wrapper
//!   holds across a call into the library.

use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

/// L162-165 `struct ngcomplex`.
#[repr(C)]
pub struct NgComplex {
    pub cx_real: c_double,
    pub cx_imag: c_double,
}

/// L176-183 `vector_info`.
#[repr(C)]
pub struct VectorInfo {
    pub v_name: *mut c_char,
    pub v_type: c_int,
    pub v_flags: i16,
    pub v_realdata: *mut c_double,
    pub v_compdata: *mut NgComplex,
    pub v_length: c_int,
}

/// L185-191 `vecvalues`. Declared for `SendData`'s signature only: the
/// bridge passes no `SendData` and never reads one.
#[repr(C)]
pub struct VecValues {
    pub name: *mut c_char,
    pub creal: c_double,
    pub cimag: c_double,
    pub is_scale: bool,
    pub is_complex: bool,
}

/// L193-197 `vecvaluesall`, for `SendData`'s signature only.
#[repr(C)]
pub struct VecValuesAll {
    pub veccount: c_int,
    pub vecindex: c_int,
    pub vecsa: *mut *mut VecValues,
}

/// L200-207 `vecinfo`, for `SendInitData`'s signature only.
#[repr(C)]
pub struct VecInfo {
    pub number: c_int,
    pub vecname: *mut c_char,
    pub is_real: bool,
    pub pdvec: *mut c_void,
    pub pdvecscale: *mut c_void,
}

/// L210-222 `vecinfoall`, for `SendInitData`'s signature only.
#[repr(C)]
pub struct VecInfoAll {
    pub name: *mut c_char,
    pub title: *mut c_char,
    pub date: *mut c_char,
    pub r#type: *mut c_char,
    pub veccount: c_int,
    pub vecs: *mut *mut VecInfo,
}

/// L247 `SendChar`.
pub type SendChar = extern "C" fn(*mut c_char, c_int, *mut c_void) -> c_int;
/// L254 `SendStat`.
pub type SendStat = extern "C" fn(*mut c_char, c_int, *mut c_void) -> c_int;
/// L261 `ControlledExit`; `NG_BOOL` is C `bool` (L171).
pub type ControlledExit = extern "C" fn(c_int, bool, bool, c_int, *mut c_void) -> c_int;
/// L270 `SendData`.
pub type SendData = extern "C" fn(*mut VecValuesAll, c_int, c_int, *mut c_void) -> c_int;
/// L279 `SendInitData`.
pub type SendInitData = extern "C" fn(*mut VecInfoAll, c_int, *mut c_void) -> c_int;
/// L287 `BGThreadRunning`.
pub type BgThreadRunning = extern "C" fn(bool, c_int, *mut c_void) -> c_int;
/// L299 `GetVSRCData`.
pub type GetVsrcData =
    extern "C" fn(*mut c_double, c_double, *mut c_char, c_int, *mut c_void) -> c_int;
/// L309 `GetISRCData`.
pub type GetIsrcData =
    extern "C" fn(*mut c_double, c_double, *mut c_char, c_int, *mut c_void) -> c_int;
/// L319 `GetSyncData`: seven parameters, not the manual's five (desk spike).
pub type GetSyncData =
    extern "C" fn(c_double, *mut c_double, c_double, c_int, c_int, c_int, *mut c_void) -> c_int;
unsafe extern "C" {
    /// L371-373.
    fn ngSpice_Init(
        printfcn: Option<SendChar>,
        statfcn: Option<SendStat>,
        ngexit: Option<ControlledExit>,
        sdata: Option<SendData>,
        sinitdata: Option<SendInitData>,
        bgtrun: Option<BgThreadRunning>,
        user_data: *mut c_void,
    ) -> c_int;
    /// L385-386.
    fn ngSpice_Init_Sync(
        vsrcdat: Option<GetVsrcData>,
        isrcdat: Option<GetIsrcData>,
        syncdat: Option<GetSyncData>,
        ident: *mut c_int,
        user_data: *mut c_void,
    ) -> c_int;
    /// L390-391.
    fn ngSpice_Command(command: *mut c_char) -> c_int;
    /// L394-395.
    fn ngGet_Vec_Info(vecname: *mut c_char) -> *mut VectorInfo;
    /// L426-427: a NULL-terminated array whose last string is `.end`.
    fn ngSpice_Circ(circarray: *mut *mut c_char) -> c_int;
    /// L451-452.
    fn ngSpice_SetBkpt(time: c_double) -> bool;
}

static STARTED: AtomicBool = AtomicBool::new(false);

/// Text ngspice hands to the caller; lossy, since ngspice promises no encoding.
fn text(s: *mut c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    // SAFETY: ngspice passes a NUL-terminated string that is valid for the
    // duration of the callback, and it is copied before the callback returns.
    let s = unsafe { CStr::from_ptr(s) };
    s.to_string_lossy().into_owned()
}

extern "C" fn send_char(s: *mut c_char, _id: c_int, _user: *mut c_void) -> c_int {
    crate::host::print(&text(s));
    0
}

extern "C" fn send_stat(_s: *mut c_char, _id: c_int, _user: *mut c_void) -> c_int {
    0
}

extern "C" fn controlled_exit(
    status: c_int,
    _unload: bool,
    _quit: bool,
    _id: c_int,
    _user: *mut c_void,
) -> c_int {
    crate::host::exited(status);
    0
}

extern "C" fn get_vsrc(
    value: *mut c_double,
    time: c_double,
    name: *mut c_char,
    _id: c_int,
    _user: *mut c_void,
) -> c_int {
    let Some(v) = crate::host::source(&text(name), time) else {
        // An external source the host does not drive: ngspice keeps its own
        // value, and the host records the miss, which fails the measurement.
        return 1;
    };
    if !value.is_null() {
        // SAFETY: `value` points at the source's value slot, valid for this
        // call (the header's L292-299 contract).
        unsafe {
            *value = v;
        }
    }
    0
}

extern "C" fn get_isrc(
    _value: *mut c_double,
    _time: c_double,
    name: *mut c_char,
    _id: c_int,
    _user: *mut c_void,
) -> c_int {
    crate::host::source_missing(&text(name));
    1
}

/// Every step is accepted as ngspice proposes it: for a static fault the
/// edges are breakpoints, so no step needs rewriting (`AET-WO-2101` S1).
extern "C" fn get_sync(
    _time: c_double,
    _delta: *mut c_double,
    _old_delta: c_double,
    _redo: c_int,
    _id: c_int,
    _location: c_int,
    _user: *mut c_void,
) -> c_int {
    0
}

fn started() -> Result<(), String> {
    if STARTED.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err("libngspice is not initialised".to_owned())
    }
}

fn c_string(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|_| "a NUL byte in text sent to ngspice".to_owned())
}

/// `ngSpice_Init`, then `ngSpice_Init_Sync`, the order the header requires.
/// A second call refuses: ngspice's state is the process's.
pub fn init() -> Result<(), String> {
    if STARTED.swap(true, Ordering::SeqCst) {
        return Err("libngspice is already initialised".to_owned());
    }
    // SAFETY: the callbacks are `extern "C"` with the header's signatures;
    // the three unused data callbacks and both user pointers are null, which
    // the header allows (L362-370, L375-384).
    let r = unsafe {
        ngSpice_Init(
            Some(send_char),
            Some(send_stat),
            Some(controlled_exit),
            None,
            None,
            None,
            std::ptr::null_mut(),
        )
    };
    if r != 0 {
        return Err(format!("ngSpice_Init returned {r}"));
    }
    // `ident` is read during the call only; a local outlives it.
    let mut ident: c_int = 0;
    // SAFETY: as above; `ident` is valid for the call.
    let r = unsafe {
        ngSpice_Init_Sync(
            Some(get_vsrc),
            Some(get_isrc),
            Some(get_sync),
            &mut ident,
            std::ptr::null_mut(),
        )
    };
    if r != 0 {
        return Err(format!("ngSpice_Init_Sync returned {r}"));
    }
    Ok(())
}

/// One command, run to its end on this thread. A background command is
/// refused, since its callbacks would run on ngspice's own thread.
pub fn command(cmd: &str) -> Result<c_int, String> {
    started()?;
    if cmd.trim_start().starts_with("bg_") {
        return Err(format!("`{cmd}` would run in ngspice's background thread"));
    }
    let c = c_string(cmd)?;
    // SAFETY: a NUL-terminated string that outlives the call; ngspice reads
    // it and does not keep the pointer.
    let r = unsafe { ngSpice_Command(c.as_ptr().cast_mut()) };
    Ok(r)
}

/// Loads a circuit, one string per line. The caller's last line is `.end`.
pub fn circ(lines: &[String]) -> Result<c_int, String> {
    started()?;
    let owned = lines
        .iter()
        .map(|l| c_string(l))
        .collect::<Result<Vec<_>, _>>()?;
    let mut array: Vec<*mut c_char> = owned.iter().map(|c| c.as_ptr().cast_mut()).collect();
    array.push(std::ptr::null_mut());
    // SAFETY: a NULL-terminated array of NUL-terminated strings, all alive
    // for the call; ngspice copies the lines.
    let r = unsafe { ngSpice_Circ(array.as_mut_ptr()) };
    Ok(r)
}

/// A breakpoint at `time` seconds in the next transient.
pub fn set_breakpoint(time: f64) -> Result<bool, String> {
    started()?;
    // SAFETY: a plain value; ngspice stores it.
    let r = unsafe { ngSpice_SetBkpt(time) };
    Ok(r)
}

/// A copy of a real vector of the current plot, or `None` when there is no
/// such vector or it is complex.
pub fn vector(name: &str) -> Result<Option<Vec<f64>>, String> {
    started()?;
    let c = c_string(name)?;
    // SAFETY: a NUL-terminated string alive for the call. The returned
    // struct and its data belong to ngspice and stay valid until the next
    // command; both are read here, null-checked, and copied before return.
    let copied = unsafe {
        let p = ngGet_Vec_Info(c.as_ptr().cast_mut());
        if p.is_null() || (*p).v_realdata.is_null() || (*p).v_length < 0 {
            None
        } else {
            let len = usize::try_from((*p).v_length).unwrap_or(0);
            Some(std::slice::from_raw_parts((*p).v_realdata, len).to_vec())
        }
    };
    Ok(copied)
}
