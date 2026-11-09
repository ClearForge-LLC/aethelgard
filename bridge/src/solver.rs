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
/// against, and the library the image ships (`AET-WO-2101` F1).
pub const NGSPICE: &str = "44.2";

/// libngspice, in this process, stepped on this thread.
pub struct Ngspice {
    loaded: bool,
    stepped: bool,
    inputs: Vec<Pwl>,
}

impl Ngspice {
    /// Initialises the library once, and refuses one that is not 44.2 by its
    /// own banner.
    pub fn start() -> Result<Self, String> {
        ffi::init()?;
        let (log, _) = host::take_log();
        let banner = format!("** ngspice-{NGSPICE} shared library");
        if !log
            .iter()
            .any(|l| l.trim_start_matches("stdout ").trim() == banner)
        {
            return Err(format!(
                "the library's banner does not say `{banner}`: {log:?}"
            ));
        }
        Ok(Self {
            loaded: false,
            stepped: false,
            inputs: Vec::new(),
        })
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
        if ffi::vector(result)?.is_some() {
            return Err(format!("a vector `{result}` exists before the measurement"));
        }
        let cmd = format!(
            "meas tran {result} TRIG v({probe}) VAL={low} TD={from} RISE=1 TARG v({probe}) VAL={high} TD={from} RISE=1"
        );
        let r = ffi::command(&cmd)?;
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
        for t in self.inputs.iter().flat_map(Pwl::vertices) {
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
