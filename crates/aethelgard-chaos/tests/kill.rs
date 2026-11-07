//! §3.4: the writer is killed (`SIGKILL`) at four points and the reader's
//! verdict on what is left is checked. The test binary re-executes itself as
//! the writer; an environment variable names the role.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use aethelgard_chaos::mirror::{mirror_step, mirror_step_observed};
use aethelgard_chaos::synthetic;
use aethelgard_chaos::{ArchiveError, CommitStage, TraceWriter, replay_dir};

use common::*;

const ROLE: &str = "AET_KILL_ROLE";
const DIR: &str = "AET_KILL_DIR";
const POINT: &str = "AET_KILL_POINT";
const READY: &str = "AET-KILL-READY";
const EVENTS: usize = 1000;

/// The writer role. In an ordinary test run it does nothing.
#[test]
fn kill_child_entry() {
    let (Some(role), Some(dir), Some(point)) = (
        std::env::var_os(ROLE),
        std::env::var_os(DIR),
        std::env::var(POINT).ok(),
    ) else {
        return;
    };
    assert_eq!(role, "writer");
    let dir = PathBuf::from(dir);
    let announce = |what: &str| {
        println!("{READY} {what}");
        std::io::stdout().flush().unwrap();
    };
    if point == "mid" {
        // A tiny buffer, so the file grows while the writer is running, and
        // an endless stream, so the kill lands wherever the writer is.
        let mut w =
            TraceWriter::create_with_capacity(&dir, &synthetic::header(0x0400), 64).unwrap();
        for (i, event) in synthetic::events(synthetic::GOLDEN_SEED).enumerate() {
            w.append(&event).unwrap();
            if i == 5_000 {
                announce("running");
            }
            assert!(i < 50_000_000, "the parent never killed the writer");
        }
        return;
    }
    let mut w = TraceWriter::create(&dir, &synthetic::header(0x0400)).unwrap();
    for event in synthetic::events(synthetic::GOLDEN_SEED).take(EVENTS) {
        w.append(&event).unwrap();
    }
    if point == "mirror-tmp" {
        w.commit().unwrap();
        // No credentials, so the mirror step writes its record: stop it once
        // the temporary file is written and synced.
        mirror_step_observed(
            &dir,
            |_| None,
            &mut std::io::sink(),
            |stage| {
                if stage == CommitStage::ManifestTempWritten {
                    announce("stopped");
                    loop {
                        std::thread::sleep(Duration::from_secs(3600));
                    }
                }
            },
        )
        .unwrap();
        panic!("the mirror step finished without reaching {point}");
    }
    let target = match point.as_str() {
        "synced" => CommitStage::TraceSynced,
        "tmp" => CommitStage::ManifestTempWritten,
        "renamed" => CommitStage::ManifestRenamed,
        other => panic!("unknown point {other}"),
    };
    w.commit_observed(|stage| {
        if stage == target {
            announce("stopped");
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
    })
    .unwrap();
    panic!("the writer finished without reaching {point}");
}

/// Run the writer to `point`, SIGKILL it there, and return the run directory.
fn killed_at(point: &str) -> PathBuf {
    let dir = scratch(&format!("kill-{point}")).join("run");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "kill_child_entry",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ROLE, "writer")
        .env(DIR, &dir)
        .env(POINT, point)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let reached = lines.any(|l| l.is_ok_and(|l| l.contains(READY)));
    child.kill().unwrap(); // SIGKILL
    let status = child.wait().unwrap();
    assert!(
        reached,
        "the writer exited before reaching {point}: {status}"
    );
    assert!(!status.success());
    dir
}

#[test]
fn killed_mid_event_is_uncommitted() {
    let dir = killed_at("mid");
    assert!(std::fs::metadata(dir.join("trace.cbor")).unwrap().len() > 0);
    assert!(matches!(replay_dir(&dir), Err(ArchiveError::Uncommitted)));
}

#[test]
fn killed_after_the_trace_sync_is_uncommitted() {
    let dir = killed_at("synced");
    assert!(matches!(replay_dir(&dir), Err(ArchiveError::Uncommitted)));
}

#[test]
fn killed_after_the_temporary_manifest_is_refused() {
    let dir = killed_at("tmp");
    assert!(dir.join("manifest.cbor.tmp").exists());
    assert!(
        matches!(replay_dir(&dir), Err(ArchiveError::UnexpectedEntry(name)) if name == "manifest.cbor.tmp")
    );
}

#[test]
fn killed_after_the_rename_is_committed_and_replays() {
    let dir = killed_at("renamed");
    let replayed = replay_dir(&dir).unwrap();
    let events: Vec<_> = synthetic::events(synthetic::GOLDEN_SEED)
        .take(EVENTS)
        .collect();
    let (_, expected) = write_run("kill-reference", 0x0400, &events);
    assert_eq!(replayed.manifest, expected);
    assert_eq!(replayed.manifest.event_count, EVENTS as u64);
}

#[test]
fn killed_in_the_mirror_step_leaves_the_run_replayable() {
    let dir = killed_at("mirror-tmp");
    assert!(dir.join("mirror.cbor.tmp").exists());
    assert!(!dir.join("mirror.cbor").exists());
    let before = replay_dir(&dir).unwrap();
    assert_eq!(before.manifest.event_count, EVENTS as u64);
    // The mirror can be retried: it clears the stale file and the run still replays.
    mirror_step(&dir, |_| None, &mut std::io::sink()).unwrap();
    assert!(!dir.join("mirror.cbor.tmp").exists());
    assert!(dir.join("mirror.cbor").exists());
    assert_eq!(replay_dir(&dir).unwrap(), before);
}
