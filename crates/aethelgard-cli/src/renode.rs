//! `simulate`: Renode loads the platform and lists what it built. Renode exits
//! 0 even when the load fails (`architecture.md` §2.4), so the exit code is
//! recorded but never trusted alone. Two witnesses in the output: a CPU in the
//! `peripherals` listing (an entry followed by a `Slot:` line) and the absence
//! of `There was an error executing command`.

use crate::run::{Output, Stop};
use crate::verdict::{Finding, Machine, Peripheral, code_of};

const TOOL: &str = "renode";
const LOAD_ERROR: &str = "There was an error executing command";

/// The monitor commands. The platform is always the call's own copy, at a
/// path this CLI chose, so no part of the caller's input reaches the monitor.
pub fn commands(copy: &std::path::Path) -> String {
    format!(
        "mach create; machine LoadPlatformDescription @{}; peripherals; quit",
        copy.display()
    )
}

pub fn judge(out: &Output) -> Result<(Vec<Finding>, Option<Machine>), Stop> {
    let code = code_of(out.ended)
        .ok_or_else(|| Stop::tool_error(TOOL, format!("renode was killed: {:?}", out.ended)))?;
    if code != 0 {
        return Err(Stop::tool_error(TOOL, format!("renode exited {code}")));
    }
    let text = out.text();
    let lines: Vec<&str> = text.lines().collect();
    let mut peripherals = Vec::new();
    let mut cpus = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(entry) = line
            .trim_start_matches([' ', '│'])
            .trim_start()
            .strip_prefix("├── ")
            .or_else(|| {
                line.trim_start_matches([' ', '│'])
                    .trim_start()
                    .strip_prefix("└── ")
            })
        else {
            continue;
        };
        let Some((name, kind)) = entry
            .trim_end()
            .strip_suffix(')')
            .and_then(|e| e.split_once(" ("))
        else {
            continue;
        };
        if lines.get(i + 1).is_some_and(|next| next.contains("Slot:")) {
            cpus.push(name.to_owned());
        }
        peripherals.push(Peripheral {
            name: name.to_owned(),
            kind: kind.to_owned(),
        });
    }
    let error = lines.iter().position(|l| l.contains(LOAD_ERROR));
    match (cpus.is_empty(), error) {
        (false, None) => Ok((Vec::new(), Some(Machine { cpus, peripherals }))),
        (true, Some(i)) => {
            let detail = lines.get(i + 1).map(|l| l.trim()).unwrap_or_default();
            Ok((
                vec![Finding::plain(
                    "platform_load_error",
                    format!("{} {detail}", lines[i].trim()).trim().to_owned(),
                )],
                None,
            ))
        }
        (false, Some(_)) => Err(Stop::tool_error(
            TOOL,
            "renode listed a CPU and also reported a load error".to_owned(),
        )),
        (true, None) => Err(Stop::tool_error(
            TOOL,
            "renode listed no CPU and reported no load error".to_owned(),
        )),
    }
}
