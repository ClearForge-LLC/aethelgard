//! The bridge's solver interface, in the shape of an FMI 3.0 Co-Simulation
//! FMU, without the dependency (`RUL-17`, the research pass of 2026-11-09):
//!
//! | here | FMI 3.0 |
//! |---|---|
//! | [`CoSimulation::instantiate`] | `fmi3InstantiateCoSimulation`, then entering and leaving initialisation |
//! | [`CoSimulation::set_input`] | `fmi3SetFloat64` on an input |
//! | [`CoSimulation::do_step`] | `fmi3DoStep(currentCommunicationPoint, communicationStepSize)`, with `earlyReturn` and `lastSuccessfulTime` |
//! | [`CoSimulation::get_output`] | `fmi3GetFloat64` on an output |
//!
//! An input here is a waveform over the step rather than one value held
//! across it, because a static pass runs the whole window as one step.
//! [`Ngspice`] takes one step from 0 per instance: in-run stepping, by
//! inversion of control inside `GetSyncData`, is a later work order's
//! (`AET-WO-2200` "It is NOT").

use crate::request::Pwl;
use crate::{ffi, host};

/// What `do_step` reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// The solver stopped before `current + step`.
    pub early_return: bool,
    /// The last time the solver reached, in seconds.
    pub last_successful_time: f64,
}

pub trait CoSimulation {
    /// Loads one model, replacing any earlier instance.
    fn instantiate(&mut self, model: &[String]) -> Result<(), String>;
    /// Drives the input `name` with `waveform` over the next step.
    fn set_input(&mut self, name: &str, waveform: Pwl) -> Result<(), String>;
    /// Advances from `current` by `step` seconds.
    fn do_step(&mut self, current: f64, step: f64) -> Result<Step, String>;
    /// The output `name` at every point the solver accepted.
    fn get_output(&mut self, name: &str) -> Result<Vec<f64>, String>;
}

/// The ngspice release the bridge is pinned to: the header it was declared
/// against, and the library the image ships (`AET-WO-2101` F1). It is only
/// compared with the banner; what the bridge reports is the banner's.
const NGSPICE: &str = "44.2";

/// What the library printed of itself when it was initialised: the banner
/// line, verbatim, and the release it names (`AET-WO-1606`).
#[derive(Clone, Debug, PartialEq)]
pub struct Banner {
    pub line: String,
    pub release: String,
}

/// The one line of `log` that starts `** ngspice-`, which must read
/// `** ngspice-<release> shared library` with a release of digits and dots.
/// No such line, more than one, or one that does not read so is refused, by
/// what was read.
pub fn banner(log: &[String]) -> Result<Banner, String> {
    let lines: Vec<&str> = log
        .iter()
        .map(|l| l.strip_prefix("stdout ").unwrap_or(l).trim())
        .filter(|l| l.starts_with("** ngspice-"))
        .collect();
    let [line] = lines.as_slice() else {
        return Err(format!(
            "the library printed {} banner lines, not one: {log:?}",
            lines.len()
        ));
    };
    let release = line
        .strip_prefix("** ngspice-")
        .and_then(|r| r.strip_suffix(" shared library"))
        .filter(|r| {
            r.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        })
        .ok_or_else(|| format!("the library's banner `{line}` names no release"))?;
    Ok(Banner {
        line: (*line).to_owned(),
        release: release.to_owned(),
    })
}

/// `banner`, then the pin: a library that is not 44.2 is refused by its own
/// banner.
fn pinned(log: &[String]) -> Result<Banner, String> {
    let b = banner(log)?;
    if b.release != NGSPICE {
        return Err(format!(
            "the library's banner `{}` names ngspice {}, not the {NGSPICE} this bridge is pinned to",
            b.line, b.release
        ));
    }
    Ok(b)
}

/// libngspice, in this process, stepped on this thread.
pub struct Ngspice {
    banner: Banner,
    loaded: bool,
    stepped: bool,
    inputs: Vec<Pwl>,
    /// Breakpoints for a deck's own sources, in seconds (`--chain`).
    breaks: Vec<f64>,
}

impl Ngspice {
    /// Initialises the library once, and refuses one that is not 44.2 by its
    /// own banner.
    pub fn start() -> Result<Self, String> {
        ffi::init()?;
        let (log, _) = host::take_log();
        Ok(Self {
            banner: pinned(&log)?,
            loaded: false,
            stepped: false,
            inputs: Vec::new(),
            breaks: Vec::new(),
        })
    }

    /// The banner the library printed when this instance started it.
    pub fn banner(&self) -> &Banner {
        &self.banner
    }

    /// Breakpoints, in seconds, that the next step places besides its inputs'.
    pub fn set_breakpoints(&mut self, times: &[f64]) {
        self.breaks = times.to_vec();
    }

    /// One command; any nonzero return, `stderr` line or exit is an error.
    fn run(&mut self, cmd: &str) -> Result<Vec<String>, String> {
        let r = ffi::command(cmd)?;
        clean(&format!("`{cmd}`"), r)
    }

    /// ngspice's own `meas` of the same rise, read back as the vector it
    /// leaves in the plot; `None` when ngspice reports it failed.
    pub fn meas(
        &mut self,
        result: &str,
        probe: &str,
        low: &str,
        high: &str,
        from: &str,
    ) -> Result<Option<f64>, String> {
        let cmd = format!(
            "meas tran {result} TRIG v({probe}) VAL={low} TD={from} RISE=1 TARG v({probe}) VAL={high} TD={from} RISE=1"
        );
        self.read_meas(result, &cmd)
    }

    /// ngspice's own `meas` of the time `probe` crosses `level` (a decimal in
    /// volts) for the `count`-th time in the given direction, in seconds;
    /// `None` when ngspice reports there is no such crossing.
    pub fn meas_when(
        &mut self,
        result: &str,
        probe: &str,
        level: &str,
        falling: bool,
        count: usize,
    ) -> Result<Option<f64>, String> {
        let edge = if falling { "FALL" } else { "RISE" };
        let cmd = format!("meas tran {result} WHEN v({probe})={level} {edge}={count}");
        self.read_meas(result, &cmd)
    }

    /// Runs one `meas` command and reads its result vector.
    fn read_meas(&mut self, result: &str, cmd: &str) -> Result<Option<f64>, String> {
        if ffi::vector(result)?.is_some() {
            return Err(format!("a vector `{result}` exists before the measurement"));
        }
        let r = ffi::command(cmd)?;
        let (log, _) = host::take_log();
        if r != 0 || host::exit().is_some() {
            return Err(format!("`{cmd}` returned {r}: {log:?}"));
        }
        let failed = log.iter().any(|l| l.ends_with("failed!"));
        let read = ffi::vector(result)?;
        // A failed `meas` leaves no vector, and asking for one prints an
        // error that would otherwise be read as the next command's.
        host::take_log();
        match read {
            Some(v) if v.len() == 1 && !failed && v[0].is_finite() => Ok(Some(v[0])),
            None if failed => Ok(None),
            other => Err(format!("`meas` left {other:?} with log {log:?}")),
        }
    }
}

/// The log of the call just made, refused when ngspice complained.
fn clean(what: &str, r: i32) -> Result<Vec<String>, String> {
    let (log, dropped) = host::take_log();
    if let Some(status) = host::exit() {
        return Err(format!("ngspice exited ({status}) during {what}: {log:?}"));
    }
    if r != 0 || dropped > 0 || log.iter().any(|l| l.starts_with("stderr")) {
        return Err(format!("{what} returned {r}: {log:?}"));
    }
    Ok(log)
}

impl CoSimulation for Ngspice {
    fn instantiate(&mut self, model: &[String]) -> Result<(), String> {
        host::clear_sources();
        self.inputs.clear();
        self.breaks.clear();
        if std::mem::take(&mut self.loaded) {
            self.run("remcirc")?;
            self.run("destroy all")?;
        }
        let r = ffi::circ(model)?;
        clean("loading the circuit", r)?;
        self.loaded = true;
        self.stepped = false;
        Ok(())
    }

    fn set_input(&mut self, name: &str, waveform: Pwl) -> Result<(), String> {
        if !self.loaded || self.stepped {
            return Err("an input is set only between instantiate and the step".to_owned());
        }
        self.inputs.push(waveform.clone());
        host::set_source(name, waveform);
        Ok(())
    }

    fn do_step(&mut self, current: f64, step: f64) -> Result<Step, String> {
        if !self.loaded || self.stepped || current != 0.0 || step.is_nan() || step <= 0.0 {
            return Err(
                "this instance takes one step, from 0, after instantiate (AET-WO-2200)".to_owned(),
            );
        }
        self.stepped = true;
        // A breakpoint at every vertex of every input, so no accepted step
        // crosses an edge (`AET-WO-2101` S1: without them the 30 % crossing
        // lands 2.1 ns late).
        let own: Vec<f64> = self.inputs.iter().flat_map(Pwl::vertices).collect();
        for t in own.into_iter().chain(self.breaks.clone()) {
            if t <= step && !ffi::set_breakpoint(t)? {
                return Err(format!("ngspice refused a breakpoint at {t} s"));
            }
        }
        self.run("run")?;
        let missing = host::take_missing();
        if !missing.is_empty() {
            return Err(format!(
                "ngspice asked for sources the bridge does not drive: {missing:?}"
            ));
        }
        let time = self.get_output("time")?;
        let last = *time.last().ok_or("the transient accepted no point")?;
        if last > step * (1.0 + 1e-9) {
            return Err(format!(
                "the transient ran to {last} s, past the {step} s step"
            ));
        }
        Ok(Step {
            early_return: last < step * (1.0 - 1e-9),
            last_successful_time: last,
        })
    }

    fn get_output(&mut self, name: &str) -> Result<Vec<f64>, String> {
        ffi::vector(name)?.ok_or_else(|| format!("ngspice has no real vector `{name}`"))
    }
}

#[cfg(test)]
mod tests {
    use super::{banner, pinned};

    fn log(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|l| (*l).to_owned()).collect()
    }

    /// `AET-WO-1606`: the release is the banner's, whatever it names; the
    /// pin is a separate refusal.
    #[test]
    fn the_release_is_read_from_the_banner_and_the_pin_refuses_another() {
        let other = log(&[
            "stdout ******",
            "stdout ** ngspice-45.0.1 shared library",
            "stdout ** Creation Date: Jan  1 2027",
        ]);
        let b = banner(&other).unwrap();
        assert_eq!(b.line, "** ngspice-45.0.1 shared library");
        assert_eq!(b.release, "45.0.1");
        let e = pinned(&other).unwrap_err();
        assert!(e.contains("names ngspice 45.0.1, not the 44.2"), "{e}");
        let ours = pinned(&log(&["stdout ** ngspice-44.2 shared library"])).unwrap();
        assert_eq!(ours.release, "44.2");
    }

    #[test]
    fn a_banner_absent_repeated_or_unreadable_is_refused() {
        for bad in [
            log(&["stdout ** Creation Date: Jan  1 2027"]),
            log(&[
                "stdout ** ngspice-44.2 shared library",
                "stdout ** ngspice-44.2 shared library",
            ]),
            log(&["stdout ** ngspice-44.2 shared"]),
            log(&["stdout ** ngspice- shared library"]),
            log(&["stdout ** ngspice-44..2 shared library"]),
            log(&["stdout ** ngspice-44.2+ shared library"]),
            log(&["stdout ** ngspice-44.2 shared library (debug)"]),
        ] {
            assert!(banner(&bad).is_err(), "{bad:?}");
        }
    }
}
