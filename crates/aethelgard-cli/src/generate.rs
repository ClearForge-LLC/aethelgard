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
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Emit(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
            Self::Twin(e) => e.fmt(f),
        }
    }
}

impl GenerateError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Emit(e) => e.kind(),
            Self::Board(e) => e.kind(),
            Self::Twin(e) => e.kind(),
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Emit(e) => e.id(),
            Self::Board(_) | Self::Twin(_) => None,
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
            Self::Board(_) | Self::Twin(_) => false,
        }
    }
}

/// The board crate's name for a project.
pub fn crate_name(project_name: &str) -> String {
    format!("{project_name}-board")
}

/// Every file `aethelgard generate` writes for `spec`, solved as `topology`
/// with `table`, keyed by its `/`-separated path under `--out`.
pub fn tree(
    spec: &Specification,
    topology: &SolvedTopology,
    table: &PinTable,
) -> Result<BTreeMap<String, String>, GenerateError> {
    let project = &spec.project_name;
    let emission =
        aethelgard_eda::schematic::emit(spec, topology, table).map_err(GenerateError::Emit)?;
    // The devices are the topology's; the firmware reads the ones the twin can
    // answer for, and the platform declares the same ones (`AET-WO-2202`).
    let answers = |part: &str| aethelgard_twin::model::model(part).is_some();
    let board =
        aethelgard_svd_hal::board::generate_with(&crate_name(project), topology, table, &answers)
            .map_err(GenerateError::Board)?;
    let platform = aethelgard_twin::platform::emit(topology).map_err(GenerateError::Twin)?;

    let mut files = BTreeMap::new();
    for (suffix, text) in [
        ("kicad_sch", emission.schematic),
        ("bom.csv", emission.bom),
        ("kicad_pcb", emission.board),
    ] {
        files.insert(format!("{KICAD_DIR}/{project}.{suffix}"), text);
    }
    for (path, text) in board {
        files.insert(format!("{BOARD_DIR}/{path}"), text);
    }
    files.insert(format!("{BOARD_DIR}/{project}.repl"), platform);
    Ok(files)
}
