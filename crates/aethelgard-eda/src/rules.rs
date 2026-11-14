//! The design rules DRC judges by, and every rule-bearing file the gate
//! writes into its scratch copy (`RUL-03` as amended, `INV-07`). None of it
//! is read from the board under judgement.
//!
//! What the gate writes, and why each piece (`AET-WO-1300` S3, run
//! 42428065833, ruled D5 on PR #40):
//! - a `(setup …)` block with no clearance token, replacing the board's. A
//!   board's own `clearance_min` there sets the board minimum and beats the
//!   project's, so it must never survive;
//! - a `.kicad_pro` whose Default net class and board minimum carry the
//!   rules. With the gate's setup token-free, its `min_clearance` is the only
//!   board minimum;
//! - a `.kicad_dru` with the same minimums as custom rules, the only file
//!   that catches a looser net class or rule.
//!
//! The project shape is the one the spike measured KiCad 10.0.7 reading in
//! full (net class, `min_clearance` and `rule_severities`).

use aethelgard_ir::Specification;

use crate::sexpr::mm;

/// The engine's floor: KiCad's default net-class clearance and track width,
/// 0.2 mm each (`aethelgard_ir::board::FLOOR_UM`). A specification declares
/// its own rules at or above it (`board.rules`, `AET-WO-4100`); `validate`
/// refuses one below it. Spacing between voltage domains is bounded by each
/// domain's spacing to ground, so one minimum covers isolation as well.
pub const FLOOR_NM: i64 = aethelgard_ir::board::FLOOR_UM as i64 * 1000;

/// Where a rule set came from, as the verdict reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Specification,
    KicadDefault,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Specification => "specification",
            Source::KicadDefault => "kicad-default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub source: Source,
    /// Every copper-to-copper clearance, and the board minimum.
    pub clearance_nm: i64,
    /// The narrowest track; `None` where KiCad's default sets no minimum.
    pub min_track_width_nm: Option<i64>,
}

impl Rules {
    /// The rules a specification declares (`board.rules`): its clearance and
    /// track width, each at or above the floor once `validate` has passed.
    pub fn from_specification(spec: &Specification) -> Rules {
        let stack = crate::stack::Stack::of(spec);
        Rules {
            source: Source::Specification,
            clearance_nm: stack.clearance_nm,
            min_track_width_nm: Some(stack.width_nm),
        }
    }

    /// KiCad's own defaults, written by the gate rather than left to the
    /// board: a 0.2 mm Default net class and no track-width minimum.
    pub fn kicad_default() -> Rules {
        Rules {
            source: Source::KicadDefault,
            clearance_nm: FLOOR_NM,
            min_track_width_nm: None,
        }
    }

    /// The board minimum: the specification's clearance, or KiCad's `0` for
    /// its defaults, which set none.
    fn board_minimum_nm(&self) -> i64 {
        match self.source {
            Source::Specification => self.clearance_nm,
            Source::KicadDefault => 0,
        }
    }

    /// The `.kicad_pro` for a copy named `<stem>`: the board minimum and the
    /// Default net class for DRC, and ERC's library checks ignored. The
    /// scratch copy holds the schematic alone, its symbols embedded, so there
    /// is no library to compare against (ruled D3). The spike measured this
    /// shape: KiCad 10.0.7 read the net class, `min_clearance` and
    /// `rule_severities` from it (runs 42427541833 and 42428065833).
    pub fn project(&self, stem: &str) -> String {
        let clearance = mm(self.clearance_nm);
        let min_clearance = mm(self.board_minimum_nm());
        let min_track = mm(self.min_track_width_nm.unwrap_or(0));
        let width = mm(self.min_track_width_nm.unwrap_or(FLOOR_NM));
        format!(
            r#"{{
  "board": {{
    "design_settings": {{
      "meta": {{ "version": 2 }},
      "rules": {{ "min_clearance": {min_clearance}, "min_track_width": {min_track} }},
      "rule_severities": {{ }}
    }}
  }},
  "erc": {{ "meta": {{ "version": 0 }}, "rule_severities": {{ "lib_symbol_issues": "ignore", "lib_symbol_mismatch": "ignore", "footprint_link_issues": "ignore" }} }},
  "meta": {{ "filename": "{stem}.kicad_pro", "version": 1 }},
  "net_settings": {{
    "classes": [
      {{ "name": "Default", "clearance": {clearance}, "track_width": {width}, "via_diameter": 0.6, "via_drill": 0.3, "microvia_diameter": 0.3, "microvia_drill": 0.1, "diff_pair_gap": 0.25, "diff_pair_via_gap": 0.25, "diff_pair_width": 0.2, "wire_width": 6, "bus_width": 12, "line_style": 0, "pcb_color": "rgba(0, 0, 0, 0.000)", "schematic_color": "rgba(0, 0, 0, 0.000)", "priority": 2147483647 }}
    ],
    "meta": {{ "version": 3 }}
  }}
}}
"#
        )
    }

    /// The `.kicad_dru`: the same minimums as custom rules.
    pub fn custom_rules(&self) -> String {
        let prefix = match self.source {
            Source::Specification => "specification",
            Source::KicadDefault => "kicad_default",
        };
        let mut out = format!(
            "(version 1)\n(rule \"{prefix}_clearance\"\n\t(constraint clearance (min {}mm)))\n",
            mm(self.clearance_nm)
        );
        if let Some(w) = self.min_track_width_nm {
            out.push_str(&format!(
                "(rule \"{prefix}_track_width\"\n\t(constraint track_width (min {}mm)))\n",
                mm(w)
            ));
        }
        out
    }
}

/// The `(setup …)` block the gate writes over the board's: no clearance
/// token, so the gate's project holds the board minimum alone (D5).
pub const SETUP: &str = "(setup\n\t\t(pad_to_mask_clearance 0)\n\t)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_specification_sets_every_minimum_and_kicad_default_only_the_net_class() {
        let spec = Rules {
            source: Source::Specification,
            clearance_nm: FLOOR_NM,
            min_track_width_nm: Some(FLOOR_NM),
        };
        assert_eq!(spec.source.name(), "specification");
        let pro = spec.project("input");
        assert!(pro.contains("\"min_clearance\": 0.2,"));
        assert!(pro.contains("\"min_track_width\": 0.2 "));
        assert!(pro.contains("\"clearance\": 0.2,"));
        assert!(pro.contains("\"filename\": \"input.kicad_pro\""));
        let dru = spec.custom_rules();
        assert!(dru.contains("(rule \"specification_clearance\""));
        assert!(dru.contains("(constraint clearance (min 0.2mm))"));
        assert!(dru.contains("(constraint track_width (min 0.2mm))"));

        let default = Rules::kicad_default();
        assert_eq!(default.source.name(), "kicad-default");
        let pro = default.project("input");
        assert!(pro.contains("\"min_clearance\": 0,"));
        assert!(pro.contains("\"clearance\": 0.2,"));
        let dru = default.custom_rules();
        assert!(dru.contains("(constraint clearance (min 0.2mm))"));
        assert!(!dru.contains("track_width"));
    }

    #[test]
    fn the_setup_block_carries_no_clearance_token() {
        let setup = crate::sexpr::parse(SETUP).unwrap();
        assert_eq!(setup.head(), Some("setup"));
        assert!(!SETUP.contains("clearance_min") && !SETUP.contains("trace_clearance"));
    }

    #[test]
    fn the_rules_file_is_balanced_s_expressions() {
        for rules in [
            Rules::kicad_default(),
            Rules {
                source: Source::Specification,
                clearance_nm: FLOOR_NM,
                min_track_width_nm: Some(FLOOR_NM),
            },
        ] {
            let wrapped = format!("(file {})", rules.custom_rules());
            assert!(crate::sexpr::parse(&wrapped).is_ok(), "{wrapped}");
        }
    }
}
