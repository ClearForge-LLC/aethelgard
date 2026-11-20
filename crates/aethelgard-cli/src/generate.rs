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
    /// The specification declares `mcu.hse` and solves to a tree other than
    /// H1 ([`generable`]), named.
    HseNotGenerated {
        tree: String,
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
/// `HseNotGenerated` (`AET-WO-4240a` §1.7, lifted for H1 by `AET-WO-4240b`
/// §1.7): a board whose `mcu.hse` is declared solves to the HSE tree
/// (`clock.hse`). The generator writes H1's bring-up: `source` PLL1,
/// `sysclk_hz` 64 MHz and every prescaler 1 (`d1cpre`, `hpre`, `d1ppre`,
/// `d2ppre1`, `d2ppre2`, `d3ppre`), `timpre` off, at VOS Scale 2. Any other
/// HSE tree is refused, the refusal naming it.
pub fn generable(topology: &SolvedTopology) -> Result<(), GenerateError> {
    if topology.core_supply == aethelgard_ir::vos::CoreSupplyMode::Bypass {
        return Err(GenerateError::CoreSupplyNotGenerated);
    }
    let c = &topology.clock;
    let h1 = c.source == "PLL1"
        && c.sysclk_hz == 64_000_000
        && [c.d1cpre, c.hpre, c.d1ppre, c.d2ppre1, c.d2ppre2, c.d3ppre] == [1; 6]
        && !c.timpre
        && c.vos == aethelgard_ir::vos::Vos::Scale2;
    if c.hse.is_some() && !h1 {
        return Err(GenerateError::HseNotGenerated {
            tree: format!(
                "source {}, sysclk_hz {}, d1cpre {}, hpre {}, d1ppre {}, d2ppre1 {}, d2ppre2 {}, d3ppre {}, timpre {}, vos {}",
                c.source,
                c.sysclk_hz,
                c.d1cpre,
                c.hpre,
                c.d1ppre,
                c.d2ppre1,
                c.d2ppre2,
                c.d3ppre,
                c.timpre,
                c.vos
            ),
        });
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
            Self::HseNotGenerated { tree } => write!(
                f,
                "mcu.hse is declared and the board solves to the HSE tree {tree}, which is not H1 (PLL1 at 64 MHz, every prescaler 1, timpre off, VOS Scale 2), the only HSE tree the generator writes (AET-WO-4240b); validate and solve accept it, generate, emit and simulate do not"
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
            Self::HseNotGenerated { .. } => "HseNotGenerated",
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Emit(e) => e.id(),
            Self::Board(_)
            | Self::Twin(_)
            | Self::CoreSupplyNotGenerated
            | Self::HseNotGenerated { .. } => None,
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
            | Self::HseNotGenerated { .. } => false,
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
    generable(topology)?;
    let project = &spec.project_name;
    let emission =
        aethelgard_eda::schematic::emit(spec, topology, table).map_err(GenerateError::Emit)?;
    // The devices are the topology's; the firmware reads every one's identity,
    // and a scripted part's further steps only where the twin can answer
    // them, as the platform declares (`AET-WO-2202`, `AET-WO-4241`).
    let answers = |part: &str| aethelgard_twin::model::model(part).is_ok_and(|m| m.is_some());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hse(fixture: &str) -> SolvedTopology {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(format!("{fixture}.json"));
        let spec: Specification = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap()
    }

    /// `AET-WO-4240b` §1.7: both HSE fixtures solve to H1 and are generable;
    /// any other HSE tree is refused as `HseNotGenerated`, naming the tree.
    #[test]
    fn only_h1_is_generable_among_hse_trees() {
        for fixture in ["hse-crystal-16", "hse-external-8"] {
            let t = hse(fixture);
            assert!(t.clock.hse.is_some(), "{fixture}");
            assert!(generable(&t).is_ok(), "{fixture}");
        }
        let mut t = hse("hse-crystal-16");
        t.clock.sysclk_hz = 200_000_000;
        t.clock.d2ppre1 = 2;
        let err = generable(&t).unwrap_err();
        assert_eq!(err.kind(), "HseNotGenerated");
        assert!(
            err.to_string().contains(
                "the HSE tree source PLL1, sysclk_hz 200000000, d1cpre 1, hpre 1, d1ppre 1, d2ppre1 2, d2ppre2 1, d3ppre 1, timpre false, vos scale2, which is not H1"
            ),
            "{err}"
        );
        // A null board with the same off-H1 fields is not this refusal's.
        t.clock.hse = None;
        assert!(generable(&t).is_ok());
    }
}
