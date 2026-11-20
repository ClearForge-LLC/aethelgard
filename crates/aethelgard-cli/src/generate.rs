//! Everything one specification produces, as relative path to text
//! (`AET-WO-3301` §1.1):
//!
//! - `kicad/<project>.kicad_sch`, `kicad/<project>.bom.csv` and
//!   `kicad/<project>.kicad_pcb`, exactly as `aethelgard emit` writes them;
//! - `board/`, the board crate `<project>-board` and its firmware crate under
//!   `board/firmware/`, from `aethelgard_svd_hal::board::generate_with` with
//!   the twin's model table as `answers`;
//! - `board/<project>.repl`, the Renode platform from
//!   `aethelgard_twin::platform::emit`.
//!
//! `board/` is what `boards/<project>/` holds. The output is a function of
//! the specification, its solved topology, the pin table and the twin's model
//! table alone (`INV-01`); nothing here reads a file, writes one or starts a
//! process.

use std::collections::BTreeMap;

use aethelgard_eda::schematic::EmitError;
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use aethelgard_svd_hal::board::BoardError;
use aethelgard_svd_hal::cmsis::RegisterError;
use aethelgard_twin::TwinError;

/// The KiCad files' directory in the tree.
pub const KICAD_DIR: &str = "kicad";
/// The board crate's directory in the tree.
pub const BOARD_DIR: &str = "board";

/// A generator's refusal. Each keeps its own kind; [`GenerateError::internal`]
/// says which only a corrupted build can cause.
#[derive(Debug)]
pub enum GenerateError {
    Emit(EmitError),
    Board(BoardError),
    Twin(TwinError),
    /// The specification declares `mcu.core_supply.mode` `bypass` ([`generable`]).
    CoreSupplyNotGenerated,
    /// The specification declares `mcu.hse` ([`generable`]).
    HseNotGenerated,
    /// The topology holds a pin device ([`generable`], `AET-WO-1211a`).
    PinDeviceNotGenerated {
        reference: String,
        part: String,
    },
    /// A motor output idles high ([`generable`], `AET-WO-1211a`).
    IdleHighNotGenerated {
        net: String,
    },
}

/// The declarations this engine validates and solves but does not generate,
/// each refused by name by `generate`, `emit` and `simulate` (and so by
/// `verify` and `campaign`, which pass through them).
///
/// `CoreSupplyNotGenerated` (`AET-WO-4240v` D-1): `generate`, `emit` and
/// `simulate` refuse a board whose core supply is declared `bypass`. It is a
/// true declaration (`validate` and `solve` accept it), but no board this
/// engine emits draws an external VCORE, and under bypass the voltage scale
/// is the external supply's (`VOSRDY` "forced by hardware to 1 when the
/// Bypass mode is selected", RM0433 Rev 9 §6.8.6, p. 310), which the
/// generated bring-up does not set.
///
/// `HseNotGenerated` (`AET-WO-4240a` §1.7, `4240` D10): a board whose
/// `mcu.hse` is declared solves to the HSE tree (`clock.hse`), whose
/// bring-up, `RCC` words, Kani clause and twin rule are `AET-WO-4240b`'s; the
/// generator today writes only the reset tree.
///
/// `PinDeviceNotGenerated` (`AET-WO-1211a`): a topology with a pin device
/// holds gpio inputs and ADC inputs no generator, simulation or board writes
/// or models until its own work order; each is refused by the device's
/// reference and part. `IdleHighNotGenerated` (`AET-WO-1211a`): a motor
/// output whose `pwm.idle` is `Idle::Level(High)` implies a pull-up
/// (`silicon::implied_pupd`), and the generator's own motor pull is the
/// pull-down until `AET-WO-1211b`.
pub fn generable(topology: &SolvedTopology) -> Result<(), GenerateError> {
    if topology.core_supply == aethelgard_ir::vos::CoreSupplyMode::Bypass {
        return Err(GenerateError::CoreSupplyNotGenerated);
    }
    if topology.clock.hse.is_some() {
        return Err(GenerateError::HseNotGenerated);
    }
    if let Some((reference, d)) = topology.pin_devices.iter().next() {
        return Err(GenerateError::PinDeviceNotGenerated {
            reference: reference.clone(),
            part: d.part.clone(),
        });
    }
    let high = |m: &aethelgard_ir::SolvedMotorOutput| {
        matches!(
            m.pwm.as_ref().map(|p| p.idle),
            Some(aethelgard_ir::motor::Idle::Level(
                aethelgard_ir::motor::Level::High
            ))
        )
    };
    if let Some((net, _)) = topology.motor_outputs.iter().find(|(_, m)| high(m)) {
        return Err(GenerateError::IdleHighNotGenerated { net: net.clone() });
    }
    Ok(())
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Emit(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Twin(e) => e.fmt(f),
            Self::CoreSupplyNotGenerated => f.write_str(
                "mcu.core_supply.mode is `bypass`: the core is supplied from an external VCORE, which no board this engine emits draws, and its voltage scale is that supply's, which the generated bring-up does not set (AET-WO-4240v D-1); validate and solve accept it, generate, emit and simulate do not",
            ),
            Self::HseNotGenerated => f.write_str(
                "mcu.hse is declared: the board solves to the HSE clock tree, whose bring-up the generator does not write until AET-WO-4240b; validate and solve accept it, generate, emit and simulate do not",
            ),
            Self::PinDeviceNotGenerated { reference, part } => write!(
                f,
                "`{reference}` ({part}) is a pin device: the board, firmware and twin this engine generates drive and read none of its pins (AET-WO-1211a); validate and solve accept it, generate, emit and simulate do not"
            ),
            Self::IdleHighNotGenerated { net } => write!(
                f,
                "motor output `{net}` idles high: the generator's motor pull is the pull-down until AET-WO-1211b (AET-WO-1211a); validate and solve accept it, generate, emit and simulate do not"
            ),
        }
    }
}

impl GenerateError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Emit(e) => e.kind(),
            Self::Board(e) => e.kind(),
            Self::Twin(e) => e.kind(),
            Self::CoreSupplyNotGenerated => "CoreSupplyNotGenerated",
            Self::HseNotGenerated => "HseNotGenerated",
            Self::PinDeviceNotGenerated { .. } => "PinDeviceNotGenerated",
            Self::IdleHighNotGenerated { .. } => "IdleHighNotGenerated",
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Emit(e) => e.id(),
            Self::PinDeviceNotGenerated { reference, .. } => Some(reference),
            Self::IdleHighNotGenerated { net } => Some(net),
            Self::Board(_)
            | Self::Twin(_)
            | Self::CoreSupplyNotGenerated
            | Self::HseNotGenerated => None,
        }
    }

    /// A vendored library or header that failed its own check: a corrupted
    /// build (exit 70), not a fault of the specification (exit 2). A register
    /// the header does not define is the topology's fault, as `simulate`
    /// rules it.
    pub fn internal(&self) -> bool {
        let register = |e: &RegisterError| !matches!(e, RegisterError::Undefined { .. });
        match self {
            Self::Emit(e) => matches!(e, EmitError::Library(_)),
            Self::Board(BoardError::Register(e)) => register(e),
            Self::Twin(TwinError::Register(e)) => register(e),
            Self::Board(_)
            | Self::Twin(_)
            | Self::CoreSupplyNotGenerated
            | Self::HseNotGenerated
            | Self::PinDeviceNotGenerated { .. }
            | Self::IdleHighNotGenerated { .. } => false,
        }
    }
}

/// The board crate's name for a project.
pub fn crate_name(project_name: &str) -> String {
    format!("{project_name}-board")
}

/// Every file `aethelgard generate` writes for `spec`, solved as `topology`
/// with `table`, keyed by its `/`-separated path under `--out`. A twin
/// stand-in (`board.drawn: false`) is not drawn: no `kicad/` (`AET-WO-1210`
/// F1, ruled 2026-11-20).
pub fn tree(
    spec: &Specification,
    topology: &SolvedTopology,
    table: &PinTable,
) -> Result<BTreeMap<String, String>, GenerateError> {
    generable(topology)?;
    let project = &spec.project_name;
    let emission = spec
        .board
        .drawn
        .then(|| aethelgard_eda::schematic::emit(spec, topology, table))
        .transpose()
        .map_err(GenerateError::Emit)?;
    // The devices are the topology's; the firmware reads every one's identity,
    // and a scripted part's further steps only where the twin can answer
    // them, as the platform declares (`AET-WO-2202`, `AET-WO-4241`).
    let answers = |part: &str| aethelgard_twin::model::model(part).is_ok_and(|m| m.is_some());
    let board =
        aethelgard_svd_hal::board::generate_with(&crate_name(project), topology, table, &answers)
            .map_err(GenerateError::Board)?;
    let platform = aethelgard_twin::platform::emit(topology).map_err(GenerateError::Twin)?;

    let mut files = BTreeMap::new();
    if let Some(e) = emission {
        for (suffix, text) in [
            ("kicad_sch", e.schematic),
            ("bom.csv", e.bom),
            ("kicad_pcb", e.board),
        ] {
            files.insert(format!("{KICAD_DIR}/{project}.{suffix}"), text);
        }
    }
    for (path, text) in board {
        files.insert(format!("{BOARD_DIR}/{path}"), text);
    }
    files.insert(format!("{BOARD_DIR}/{project}.repl"), platform);
    Ok(files)
}
