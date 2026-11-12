//! `aethelgard mcp`: the command tree served as MCP tools on stdio (`RUL-10`).
//!
//! The tool list is computed from the unbuilt `clap` command tree by [`tools`]; nothing about a
//! tool is written here. A call runs this very binary as a child with an argument vector built
//! only from the subcommand path and that subcommand's declared long arguments ([`argv`]), so a
//! call gets exactly the CLI's sandbox, exit codes and JSON verdict (`INV-01`). Raw client text is
//! never forwarded as an argument vector, and no path leads to the internal first arguments
//! (`architecture.md` §6).

use std::collections::HashSet;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{ExitCode, Stdio};
use std::sync::Arc;

use clap::{ArgAction, Command, CommandFactory};
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::{RequestContext, RoleServer, ServerInitializeError};
use rmcp::{ServiceExt, transport};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command as Child;
use tokio::sync::Notify;

use crate::{Cli, run};

/// A child's stdout is one JSON line; more than this is a tool error (`INV-06`).
const MAX_STDOUT: usize = 1 << 20;
/// Stderr is kept for the diagnostic only.
const MAX_STDERR: usize = 64 << 10;
/// Exit statuses of a signalled `mcp`: 128 + the signal number.
const EXIT_SIGHUP: u8 = 129;
const EXIT_SIGINT: u8 = 130;
const EXIT_SIGTERM: u8 = 143;

/// One tool: the subcommand path it runs and the long names it accepts, in declared order.
struct Leaf {
    name: String,
    about: String,
    path: Vec<String>,
    args: Vec<Arg>,
    /// The subcommand prints one JSON line per record and then its verdict
    /// ([`crate::LINE_PER_RECORD`]); every other prints exactly one line.
    lines: bool,
}

struct Arg {
    long: String,
    help: String,
    required: bool,
    /// Repeatable (`ArgAction::Append`): a non-empty array of strings, one token each.
    many: bool,
}

/// Every leaf subcommand of `root` (an **unbuilt** command: a built one grows `help`
/// subcommands and a `help` argument everywhere). Panics on anything it cannot map, so a future
/// positional or boolean flag fails a unit test, never a client.
fn leaves(root: &Command) -> Vec<Leaf> {
    fn walk(cmd: &Command, path: &mut Vec<String>, out: &mut Vec<Leaf>) {
        for sub in cmd.get_subcommands() {
            let name = sub.get_name();
            // `mcp` is this server; `help` is clap's. Neither is a tool.
            if name == "help" || name == "mcp" {
                continue;
            }
            path.push(name.to_owned());
            if sub
                .get_subcommands()
                .any(|s| !matches!(s.get_name(), "help" | "mcp"))
            {
                walk(sub, path, out);
            } else {
                out.push(leaf(sub, path));
            }
            path.pop();
        }
    }
    let mut out = Vec::new();
    walk(root, &mut Vec::new(), &mut out);
    let mut seen = HashSet::new();
    for l in &out {
        assert!(
            seen.insert(l.name.clone()),
            "duplicate tool name {}",
            l.name
        );
    }
    out
}

fn leaf(cmd: &Command, path: &[String]) -> Leaf {
    let name = path.join("_");
    // A name an argument vector could mistake for a flag, such as the internal first arguments.
    assert!(
        path.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        }) && !path[0].starts_with('-'),
        "tool path {path:?} is not a plain subcommand path"
    );
    let mut args = Vec::new();
    for arg in cmd.get_arguments() {
        if matches!(
            arg.get_action(),
            ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
        ) {
            continue;
        }
        let id = arg.get_id();
        let long = arg
            .get_long()
            .unwrap_or_else(|| panic!("tool {name}: argument `{id}` has no long name"));
        assert!(
            matches!(arg.get_action(), ArgAction::Set | ArgAction::Append),
            "tool {name}: argument `{id}` is not a plain value (action {:?})",
            arg.get_action()
        );
        args.push(Arg {
            long: long.to_owned(),
            help: arg.get_help().map(ToString::to_string).unwrap_or_default(),
            required: arg.is_required_set(),
            many: matches!(arg.get_action(), ArgAction::Append),
        });
    }
    Leaf {
        about: cmd.get_about().map(ToString::to_string).unwrap_or_default(),
        lines: crate::LINE_PER_RECORD.contains(&name.as_str()),
        name,
        path: path.to_vec(),
        args,
    }
}

fn schema(leaf: &Leaf) -> JsonObject {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for a in &leaf.args {
        let property = if a.many {
            json!({"type": "array", "items": {"type": "string"}, "minItems": 1, "description": a.help})
        } else {
            json!({"type": "string", "description": a.help})
        };
        properties.insert(a.long.clone(), property);
        if a.required {
            required.push(Value::from(a.long.clone()));
        }
    }
    let mut s = serde_json::Map::new();
    s.insert("type".into(), "object".into());
    s.insert("properties".into(), properties.into());
    if !required.is_empty() {
        s.insert("required".into(), required.into());
    }
    s.insert("additionalProperties".into(), false.into());
    s
}

/// The tool list, one tool per leaf subcommand, from the unbuilt command tree.
pub fn tools(root: &Command) -> Vec<Tool> {
    leaves(root)
        .iter()
        .map(|l| Tool::new(l.name.clone(), l.about.clone(), Arc::new(schema(l))))
        .collect()
}

/// A call refused before any child is spawned: an MCP invalid-params error.
#[derive(Debug, PartialEq)]
struct Refused(String);

/// The child's argument vector: the subcommand path, then one `--<long>=<value>` token per
/// argument, and per item of a repeatable one, so a value starting with `-` can never be read as a
/// flag. Only declared names, only string values (a non-empty array of them for a repeatable
/// argument), every required argument present, no NUL.
fn argv(leaves: &[Leaf], tool: &str, given: &JsonObject) -> Result<Vec<OsString>, Refused> {
    let leaf = leaves
        .iter()
        .find(|l| l.name == tool)
        .ok_or_else(|| Refused(format!("unknown tool `{tool}`")))?;
    if let Some(unknown) = given
        .keys()
        .find(|k| !leaf.args.iter().any(|a| &a.long == *k))
    {
        return Err(Refused(format!("unknown argument `{unknown}`")));
    }
    let mut out: Vec<OsString> = leaf.path.iter().map(OsString::from).collect();
    for a in &leaf.args {
        match given.get(&a.long) {
            None if a.required => {
                return Err(Refused(format!("missing required argument `{}`", a.long)));
            }
            None => {}
            Some(Value::Array(items)) if a.many && !items.is_empty() => {
                for item in items {
                    out.push(token(&a.long, item)?);
                }
            }
            Some(_) if a.many => {
                return Err(Refused(format!(
                    "argument `{}` must be a non-empty array of strings",
                    a.long
                )));
            }
            Some(v) => out.push(token(&a.long, v)?),
        }
    }
    Ok(out)
}

/// One `--<long>=<value>` token from a string with no NUL.
fn token(long: &str, v: &Value) -> Result<OsString, Refused> {
    match v {
        Value::String(v) if v.contains('\0') => {
            Err(Refused(format!("argument `{long}` contains NUL")))
        }
        Value::String(v) => Ok(format!("--{long}={v}").into()),
        _ => Err(Refused(format!("argument `{long}` must be a string"))),
    }
}

/// How a child ended, or why the call stopped reading.
struct Finished {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    capped: Option<&'static str>,
}

/// Reads to the end, keeping at most `cap` bytes. Past the cap it raises `over` once, then keeps
/// draining and discarding until the pipe closes: a reader that stopped would block the child
/// forever, and `RUL-13` gives the call no timeout.
async fn drain(mut pipe: impl AsyncRead + Unpin, cap: usize, over: &Notify) -> Option<Vec<u8>> {
    let mut kept = Vec::new();
    let mut buf = [0u8; 8192];
    let mut capped = false;
    loop {
        match pipe.read(&mut buf).await {
            Ok(0) | Err(_) => return (!capped).then_some(kept),
            Ok(n) if !capped && kept.len() + n <= cap => kept.extend_from_slice(&buf[..n]),
            Ok(_) => {
                if !capped {
                    capped = true;
                    over.notify_one();
                }
            }
        }
    }
}

/// Runs the child to the end. Dropping this future (the call was cancelled) kills the child
/// (`kill_on_drop`); its reaper then ends the tool's whole process group (`architecture.md` §6).
async fn execute(exe: &PathBuf, args: &[OsString]) -> std::io::Result<Finished> {
    let mut child = Child::new(exe)
        .args(args)
        .env(run::PARENT_PID_VAR, std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let over = Notify::new();
    let out = drain(child.stdout.take().expect("piped"), MAX_STDOUT, &over);
    let err = drain(child.stderr.take().expect("piped"), MAX_STDERR, &over);
    let wait = async {
        let mut capped = None;
        let status = tokio::select! {
            status = child.wait() => status,
            () = over.notified() => {
                capped = Some("over the output cap");
                let _ = child.kill().await;
                child.wait().await
            }
        };
        (status, capped)
    };
    let (out, err, (status, capped)) = tokio::join!(out, err, wait);
    let status = status?;
    let capped = match (out.is_some(), err.is_some(), capped) {
        (false, _, _) => Some("stdout exceeded the 1 MiB cap"),
        (_, false, _) => Some("stderr exceeded the 64 KiB cap"),
        (true, true, c) => c,
    };
    Ok(Finished {
        code: status.code(),
        stdout: out.unwrap_or_default(),
        stderr: err.unwrap_or_default(),
        capped,
    })
}

/// Maps a finished child to the tool result. `isError` is false only for exit 0, 1 or `propose`'s 6
/// (a generator report) with exactly one JSON object on stdout; a fail verdict is a successful call. With `lines`, stdout may hold
/// one JSON object line per record before the verdict's: the last line is the verdict, and the
/// lines before it are `records`.
fn result(done: &Finished, lines: bool) -> CallToolResult {
    let error = |message: String| CallToolResult::error(vec![ContentBlock::text(message)]);
    let exit = done.code.map_or_else(
        || "was killed by a signal".to_owned(),
        |c| format!("exited {c}"),
    );
    if let Some(cap) = done.capped {
        // `exit` describes the kill this module sent, so the cap is named first.
        return error(format!("{cap}: the call was stopped (the child {exit})"));
    }
    let line = done
        .stdout
        .strip_suffix(b"\n")
        .filter(|l| lines || !l.contains(&b'\n'));
    let object = |l: &[u8]| {
        serde_json::from_slice::<Value>(l)
            .ok()
            .filter(Value::is_object)
    };
    let mut objects = line
        .map(|l| {
            l.split(|b| *b == b'\n')
                .map(object)
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_default();
    let verdict = objects.as_mut().and_then(Vec::pop);
    let (Some(line), Some(records), Some(verdict)) = (line, objects, verdict) else {
        let stderr = String::from_utf8_lossy(&done.stderr);
        return error(format!(
            "the child {exit} and stdout is not exactly one JSON object line; stderr: {}",
            stderr.trim_end()
        ));
    };
    let mut r = CallToolResult::success(vec![ContentBlock::text(
        String::from_utf8_lossy(line).into_owned(),
    )]);
    r.structured_content = Some(if lines {
        json!({"exit_code": done.code, "verdict": verdict, "records": records})
    } else {
        json!({"exit_code": done.code, "verdict": verdict})
    });
    let report = i32::from(crate::EXIT_GENERATOR_REPORT);
    r.is_error = Some(!matches!(done.code, Some(c) if c == 0 || c == 1 || c == report));
    r
}

struct Server {
    leaves: Vec<Leaf>,
    tools: Vec<Tool>,
    exe: PathBuf,
}

impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("aethelgard", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tools.clone()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let given = request.arguments.unwrap_or_default();
        let args = argv(&self.leaves, &request.name, &given)
            .map_err(|Refused(m)| ErrorData::invalid_params(m, None))?;
        tokio::select! {
            done = execute(&self.exe, &args) => match done {
                Ok(done) => {
                    let lines = self.leaves.iter().any(|l| l.name == request.name && l.lines);
                    Ok(result(&done, lines).into())
                }
                Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "cannot run the child: {e}"
                ))])
                .into()),
            },
            () = context.ct.cancelled() => Err(ErrorData::internal_error("cancelled", None)),
        }
    }
}

/// `aethelgard mcp`. Exits 0 when stdin closes.
pub fn serve() -> ExitCode {
    // Before the runtime starts, and for the same reason the trampoline checks: otherwise every
    // call would be refused later, one at a time.
    let leaked = match run::inherited_descriptors() {
        Ok(leaked) if leaked.is_empty() => None,
        Ok(leaked) => Some(format!(
            "descriptors inherited without close-on-exec: {leaked:?}"
        )),
        Err(reason) => Some(format!("descriptors {reason}")),
    };
    if let Some(message) = leaked {
        // Stdout carries protocol messages only, so the refusal goes to stderr.
        let refusal = crate::Refusal::new("SandboxUnavailable", Some("descriptors"), message);
        let _ = writeln!(
            std::io::stderr(),
            "{}",
            serde_json::to_string(&refusal).unwrap_or_default()
        );
        return ExitCode::from(crate::EXIT_TOOL_UNAVAILABLE);
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            crate::diagnose(&format!("cannot find its own executable: {e}"));
            return ExitCode::from(crate::EXIT_INTERNAL);
        }
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            crate::diagnose(&format!("cannot start the runtime: {e}"));
            return ExitCode::from(crate::EXIT_INTERNAL);
        }
    };
    let code = runtime.block_on(run_server(exe));
    // A blocked read of stdin must not hold the process open.
    runtime.shutdown_background();
    code
}

async fn run_server(exe: PathBuf) -> ExitCode {
    use tokio::signal::unix::{SignalKind, signal};
    let (Ok(mut term), Ok(mut int), Ok(mut hup)) = (
        signal(SignalKind::terminate()),
        signal(SignalKind::interrupt()),
        signal(SignalKind::hangup()),
    ) else {
        crate::diagnose("cannot install signal handlers");
        return ExitCode::from(crate::EXIT_INTERNAL);
    };
    let command = Cli::command();
    let server = Server {
        tools: tools(&command),
        leaves: leaves(&command),
        exe,
    };
    // Signals are served from the start: a handshake that never comes must not hold them off.
    let started = tokio::select! {
        started = server.serve(transport::stdio()) => started,
        _ = term.recv() => return ExitCode::from(EXIT_SIGTERM),
        _ = int.recv() => return ExitCode::from(EXIT_SIGINT),
        _ = hup.recv() => return ExitCode::from(EXIT_SIGHUP),
    };
    let running = match started {
        Ok(running) => running,
        // stdin closed before `initialize`: the same clean end as closing it later.
        Err(ServerInitializeError::ConnectionClosed(_)) => return ExitCode::SUCCESS,
        Err(e) => {
            crate::diagnose(&format!("cannot start the MCP service: {e}"));
            return ExitCode::from(crate::EXIT_INTERNAL);
        }
    };
    // A signal returns from here; `serve` then drops the runtime, which drops every running
    // call, and `kill_on_drop` kills each call's child. Its reaper ends the tool's group.
    tokio::select! {
        ended = running.waiting() => match ended {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                crate::diagnose(&format!("the MCP service failed: {e}"));
                ExitCode::from(crate::EXIT_INTERNAL)
            }
        },
        _ = term.recv() => ExitCode::from(EXIT_SIGTERM),
        _ = int.recv() => ExitCode::from(EXIT_SIGINT),
        _ = hup.recv() => ExitCode::from(EXIT_SIGHUP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Arg;

    fn names(root: &Command) -> Vec<String> {
        tools(root).iter().map(|t| t.name.to_string()).collect()
    }

    fn obj(v: Value) -> JsonObject {
        v.as_object().cloned().expect("an object")
    }

    #[test]
    fn the_tools_come_from_the_tree() {
        assert_eq!(
            names(&Cli::command()),
            [
                "validate",
                "solve",
                "emit",
                "check_drc",
                "check_erc",
                "prove",
                "simulate",
                "campaign",
                "replay",
                "attribute",
                "propose"
            ]
        );
    }

    #[test]
    fn schema_keys_are_long_names_and_required_follows_clap() {
        let all = tools(&Cli::command());
        let prove = all.iter().find(|t| t.name == "prove").unwrap();
        let s = Value::Object((*prove.input_schema).clone());
        assert_eq!(s["additionalProperties"], false);
        assert_eq!(s["type"], "object");
        let mut keys: Vec<_> = s["properties"].as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["crate", "harness"]);
        assert_eq!(s["required"], json!(["crate", "harness"]));
        assert_eq!(s["properties"]["crate"]["type"], "string");
        let validate = all.iter().find(|t| t.name == "validate").unwrap();
        assert!(validate.input_schema.get("required").is_none());
        let emit = all.iter().find(|t| t.name == "emit").unwrap();
        assert_eq!(emit.input_schema["required"], json!(["file", "out"]));
        let drc = all.iter().find(|t| t.name == "check_drc").unwrap();
        assert_eq!(drc.input_schema["required"], json!(["board"]));
        assert_eq!(drc.input_schema["properties"]["spec"]["type"], "string");
        assert!(!validate.description.as_deref().unwrap().is_empty());
    }

    /// The throwaway-subcommand clause of the P1 gate: a leaf added to the tree appears with no
    /// change to this module.
    #[test]
    fn a_throwaway_subcommand_appears_in_the_list() {
        let root = Cli::command().subcommand(
            Command::new("throwaway")
                .about("A subcommand added by a test")
                .arg(Arg::new("thing").long("thing").required(true)),
        );
        let all = tools(&root);
        let t = all.iter().find(|t| t.name == "throwaway").expect("listed");
        assert_eq!(
            t.description.as_deref(),
            Some("A subcommand added by a test")
        );
        assert_eq!(t.input_schema["required"], json!(["thing"]));
        assert_eq!(all.len(), 12);
    }

    #[test]
    fn a_nested_throwaway_is_named_by_its_path() {
        let root = Cli::command()
            .subcommand(Command::new("outer").subcommand(Command::new("inner").about("x")));
        assert!(names(&root).contains(&"outer_inner".to_owned()));
    }

    #[test]
    #[should_panic(expected = "no long name")]
    fn a_positional_argument_panics() {
        tools(&Cli::command().subcommand(Command::new("p").arg(Arg::new("file"))));
    }

    #[test]
    #[should_panic(expected = "not a plain value")]
    fn a_boolean_flag_panics() {
        tools(
            &Cli::command().subcommand(
                Command::new("b").arg(Arg::new("v").long("v").action(ArgAction::SetTrue)),
            ),
        );
    }

    #[test]
    #[should_panic(expected = "not a plain subcommand path")]
    fn a_subcommand_named_like_an_internal_argument_panics() {
        tools(&Cli::command().subcommand(Command::new(run::REAPER_ARG)));
    }

    #[test]
    fn no_tool_is_named_like_an_internal_argument_or_mcp_or_help() {
        let root = Cli::command()
            .subcommand(Command::new("mcp").about("x"))
            .subcommand(Command::new("help").about("x"))
            .subcommand(Command::new("grp").subcommand(Command::new("help")));
        for name in names(&root) {
            assert!(!name.starts_with('-'), "{name}");
            for banned in [
                run::REAPER_ARG,
                run::CONFINE_ARG,
                run::GUARD_ARG,
                "mcp",
                "help",
            ] {
                assert_ne!(name, banned);
            }
            assert!(!name.contains("internal"), "{name}");
        }
        // A group holding only `help` is a leaf itself; its `help` is never a tool.
        assert!(names(&root).contains(&"grp".to_owned()));
        assert!(!names(&root).contains(&"grp_help".to_owned()));
    }

    fn real() -> Vec<Leaf> {
        leaves(&Cli::command())
    }

    /// `AET-WO-2100` (R5): the boot is three optional flags on the same tool,
    /// so `simulate` keeps its name and only `--platform` is required.
    #[test]
    fn simulate_keeps_its_name_and_takes_the_boot_as_optional_flags() {
        let leaf = real().into_iter().find(|l| l.name == "simulate").unwrap();
        let args: Vec<(&str, bool)> = leaf
            .args
            .iter()
            .map(|a| (a.long.as_str(), a.required))
            .collect();
        assert_eq!(
            args,
            [
                ("platform", true),
                ("firmware", false),
                ("spec", false),
                ("for", false)
            ]
        );
    }

    #[test]
    fn argv_builds_one_token_per_argument() {
        let v = argv(
            &real(),
            "prove",
            &obj(json!({"harness": "h", "crate": "/c"})),
        )
        .unwrap();
        assert_eq!(
            v,
            ["prove", "--crate=/c", "--harness=h"].map(OsString::from)
        );
        let v = argv(&real(), "check_drc", &obj(json!({"board": "/b"}))).unwrap();
        assert_eq!(v, ["check", "drc", "--board=/b"].map(OsString::from));
        let v = argv(&real(), "validate", &obj(json!({}))).unwrap();
        assert_eq!(v, ["validate"].map(OsString::from));
    }

    /// `AET-WO-2300`: `--scenarios` repeats, so it is an array of strings, one token per item.
    #[test]
    fn a_repeatable_argument_is_an_array_with_one_token_per_item() {
        let all = tools(&Cli::command());
        let campaign = all.iter().find(|t| t.name == "campaign").unwrap();
        let s = &campaign.input_schema;
        assert_eq!(s["properties"]["scenarios"]["type"], "array");
        assert_eq!(s["properties"]["scenarios"]["items"]["type"], "string");
        assert_eq!(s["properties"]["scenarios"]["minItems"], 1);
        assert_eq!(s["properties"]["firmware"]["type"], "string");
        assert_eq!(s["required"], json!(["scenarios", "firmware", "archive"]));
        let v = argv(
            &real(),
            "campaign",
            &obj(json!({"scenarios": ["a.json", "--b"], "firmware": "f", "archive": "d"})),
        )
        .unwrap();
        assert_eq!(
            v,
            [
                "campaign",
                "--scenarios=a.json",
                "--scenarios=--b",
                "--firmware=f",
                "--archive=d"
            ]
            .map(OsString::from)
        );
        let l = real();
        let refuse = |v: Value| argv(&l, "campaign", &obj(v)).unwrap_err().0;
        let rest =
            |scenarios: Value| json!({"scenarios": scenarios, "firmware": "f", "archive": "d"});
        for bad in [json!("a.json"), json!([]), json!(null), json!({})] {
            assert!(
                refuse(rest(bad.clone())).contains("non-empty array"),
                "{bad}"
            );
        }
        assert!(refuse(rest(json!(["a", 1]))).contains("must be a string"));
        assert!(refuse(rest(json!(["a\0"]))).contains("NUL"));
        assert!(refuse(json!({"firmware": "f", "archive": "d"})).contains("missing required"));
        let replay = real().into_iter().find(|l| l.name == "replay").unwrap();
        assert!(!replay.lines && replay.args.iter().all(|a| !a.many));
    }

    /// Only a subcommand in `LINE_PER_RECORD` may print more than one line,
    /// and then every line must be a JSON object; the last is the verdict.
    #[test]
    fn a_line_per_record_subcommand_returns_its_records_and_its_verdict() {
        assert!(real().iter().find(|l| l.name == "campaign").unwrap().lines);
        let two = b"{\"scenario\":\"a\"}\n{\"ok\":true,\"verdict\":\"pass\"}\n";
        let r = view(&result(&done(Some(0), two), true));
        assert_eq!(r["isError"], false);
        assert_eq!(r["structuredContent"]["verdict"]["verdict"], "pass");
        assert_eq!(
            r["structuredContent"]["records"],
            json!([{"scenario": "a"}])
        );
        let one = view(&result(&done(Some(1), b"{\"ok\":false}\n"), true));
        assert_eq!(one["structuredContent"]["records"], json!([]));
        for bad in [&b"{}\nnope\n"[..], b"{}\n\n{}\n", b"{}\n[]\n", b"", b"{}"] {
            let r = view(&result(&done(Some(0), bad), true));
            assert_eq!(r["isError"], true, "{bad:?}");
            assert!(r.get("structuredContent").is_none(), "{bad:?}");
        }
        // The same two lines from any other tool are an error.
        assert_eq!(view(&result(&done(Some(0), two), false))["isError"], true);
    }

    #[test]
    fn a_value_starting_with_dashes_is_one_token_not_a_flag() {
        let v = argv(&real(), "validate", &obj(json!({"file": "--help"}))).unwrap();
        assert_eq!(v, ["validate", "--file=--help"].map(OsString::from));
        let v = argv(&real(), "validate", &obj(json!({"file": "--x=a\nb"}))).unwrap();
        assert_eq!(v[1], OsString::from("--file=--x=a\nb"));
    }

    #[test]
    fn argv_refuses_what_the_schema_forbids() {
        let l = real();
        let refuse = |tool: &str, v: Value| argv(&l, tool, &obj(v)).unwrap_err().0;
        assert!(refuse("validate", json!({"nope": "x"})).contains("unknown argument"));
        assert!(refuse("validate", json!({"help": "x"})).contains("unknown argument"));
        assert!(refuse("validate", json!({"version": "x"})).contains("unknown argument"));
        assert!(refuse("validate", json!({"file": 3})).contains("must be a string"));
        assert!(refuse("validate", json!({"file": null})).contains("must be a string"));
        assert!(refuse("validate", json!({"file": ["a"]})).contains("must be a string"));
        assert!(refuse("check_drc", json!({})).contains("missing required"));
        assert!(refuse("prove", json!({"crate": "/c"})).contains("missing required"));
        assert!(refuse("validate", json!({"file": "a\0b"})).contains("NUL"));
        assert!(refuse("nope", json!({})).contains("unknown tool"));
        for name in [
            "",
            "check",
            "check.drc",
            "check/drc",
            "Validate",
            "mcp",
            "help",
            run::REAPER_ARG,
        ] {
            assert!(refuse(name, json!({})).contains("unknown tool"), "{name}");
        }
    }

    fn done(code: Option<i32>, stdout: &[u8]) -> Finished {
        Finished {
            code,
            stdout: stdout.to_vec(),
            stderr: b"diag\n".to_vec(),
            capped: None,
        }
    }

    fn view(r: &CallToolResult) -> Value {
        serde_json::to_value(r).unwrap()
    }

    #[test]
    fn exit_0_1_and_6_are_successful_calls_and_others_are_errors() {
        let line = br#"{"ok":false,"verdict":"fail"}"#;
        let with_nl = [line.as_slice(), b"\n"].concat();
        for (code, error) in [
            (Some(0), false),
            (Some(1), false),
            (Some(2), true),
            (Some(3), true),
            (Some(4), true),
            (Some(5), true),
            (Some(6), false),
            (Some(7), true),
            (Some(64), true),
            (Some(70), true),
            (None, true),
        ] {
            let r = view(&result(&done(code, &with_nl), false));
            assert_eq!(r["isError"], error, "{code:?}");
            assert_eq!(r["content"][0]["text"], std::str::from_utf8(line).unwrap());
            assert_eq!(r["structuredContent"]["exit_code"], json!(code));
            assert_eq!(r["structuredContent"]["verdict"]["verdict"], "fail");
        }
    }

    #[test]
    fn stdout_that_is_not_one_json_object_line_is_an_error() {
        for bad in [
            &b""[..],
            b"{}",
            b"{}\n{}\n",
            b"[]\n",
            b"1\n",
            b"null\n",
            b"nope\n",
            b"\n",
            b"{} \n\n",
        ] {
            let r = view(&result(&done(Some(0), bad), false));
            assert_eq!(r["isError"], true, "{bad:?}");
            assert!(r.get("structuredContent").is_none(), "{bad:?}");
            assert!(r["content"][0]["text"].as_str().unwrap().contains("diag"));
        }
    }

    #[test]
    fn a_capped_child_is_an_error_naming_the_cap() {
        let mut d = done(Some(0), b"{}\n");
        d.capped = Some("stdout exceeded the 1 MiB cap");
        let r = view(&result(&d, false));
        assert_eq!(r["isError"], true);
        assert!(r["content"][0]["text"].as_str().unwrap().contains("1 MiB"));
    }

    fn sh(script: &str) -> Vec<OsString> {
        vec!["-c".into(), script.into()]
    }

    #[tokio::test(flavor = "current_thread")]
    async fn execute_captures_one_line_and_the_exit_code() {
        let d = execute(
            &PathBuf::from("/bin/sh"),
            &sh("echo '{\"a\":1}'; echo e >&2; exit 1"),
        )
        .await
        .unwrap();
        assert_eq!((d.code, d.capped), (Some(1), None));
        assert_eq!(d.stdout, b"{\"a\":1}\n");
        assert_eq!(d.stderr, b"e\n");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_child_that_prints_forever_is_killed_and_drained() {
        // A removed notify would hang here, so the wait is bounded.
        let d = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            execute(
                &PathBuf::from("/bin/sh"),
                &sh("while :; do echo xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; done"),
            ),
        )
        .await
        .expect("the cap must stop the child")
        .unwrap();
        assert!(d.capped.unwrap().contains("stdout"));
        assert!(d.stdout.len() <= MAX_STDOUT);
        assert!(view(&result(&d, false))["isError"].as_bool().unwrap());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_child_that_floods_stderr_is_capped_too() {
        // A removed notify would hang here, so the wait is bounded.
        let d = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            execute(
                &PathBuf::from("/bin/sh"),
                &sh("while :; do echo yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy >&2; done"),
            ),
        )
        .await
        .expect("the cap must stop the child")
        .unwrap();
        assert!(d.capped.unwrap().contains("stderr"));
        assert!(d.stderr.len() <= MAX_STDERR);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_child_killed_by_a_signal_has_no_exit_code() {
        let d = execute(&PathBuf::from("/bin/sh"), &sh("kill -9 $$"))
            .await
            .unwrap();
        assert_eq!(d.code, None);
        assert_eq!(view(&result(&d, false))["isError"], true);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_missing_executable_is_an_io_error() {
        assert!(
            execute(&PathBuf::from("/nonexistent/aethelgard"), &[])
                .await
                .is_err()
        );
    }
}
