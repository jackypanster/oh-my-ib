//! FROZEN SPEC — card 01 (`omi help`: one-shot agent-parseable command surface).
//! Black-box subprocess assertions: reference ONLY the `omi` binary contract
//! (args, stdout, exit code) — never internal symbols. Deterministic offline:
//! help needs no gateway and no config file.
//!
//! RED today: `omi help` hits clap's builtin (plain text, not JSON). pipeline-impl
//! makes it GREEN via src/surface.rs + the Help variant (arch.md, ADR 0036/0037).
//! The coder must NOT edit this file (the freeze gate).

use std::collections::BTreeSet;

use assert_cmd::Command;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn help_stdout_json(cmd: &mut Command) -> serde_json::Value {
    let assert = cmd.arg("help").assert().success();
    serde_json::from_slice(&assert.get_output().stdout)
        .expect("`omi help` stdout should be valid JSON")
}

/// Happy path: one invocation, valid JSON, and every command entry carries the
/// full agent-facing field set (name/purpose/usage/example/gate, all non-empty).
#[test]
fn help_emits_the_full_surface_as_json() {
    let v = help_stdout_json(&mut omi());
    assert!(v["global"].is_object(), "top-level `global` object missing");
    let commands = v["commands"]
        .as_array()
        .expect("top-level `commands` array missing");
    assert!(!commands.is_empty(), "commands array must not be empty");
    for entry in commands {
        for key in ["name", "purpose", "usage", "example", "gate"] {
            let val = entry[key]
                .as_str()
                .unwrap_or_else(|| panic!("entry {entry} missing string `{key}`"));
            assert!(!val.is_empty(), "entry {entry}: `{key}` must be non-empty");
        }
    }
}

/// Staleness pin: the help inventory equals the exact set of implemented
/// subcommands (25 existing + help + logs). A command missing from help — or a
/// dangling help entry — fails this set equality in either direction.
#[test]
fn help_inventory_matches_the_command_set() {
    let expected: BTreeSet<&str> = [
        "account",
        "brief",
        "buy",
        "cancel",
        "completed-orders",
        "contract",
        "executions",
        "grid-tick",
        "health",
        "help",
        "history",
        "logs",
        "option-buy",
        "option-chain",
        "option-close",
        "option-combo",
        "option-quote",
        "option-sell",
        "orders",
        "pnl",
        "pnl-by-position",
        "positions",
        "quote",
        "search",
        "sell",
        "sma-signal",
        "sma-tick",
    ]
    .into_iter()
    .collect();
    let v = help_stdout_json(&mut omi());
    let actual: BTreeSet<String> = v["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .map(|e| e["name"].as_str().expect("entry name").to_string())
        .collect();
    let actual_refs: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    assert_eq!(actual_refs, expected, "help inventory drifted from the CLI surface");
}

/// Safety surface: write-gate markers per AGENTS.md hard rules — reads are
/// `read-only`, gated writes are `write`, the paper-only ticks are `write-paper-only`.
#[test]
fn help_marks_the_write_gates() {
    let v = help_stdout_json(&mut omi());
    let gate_of = |name: &str| -> String {
        v["commands"]
            .as_array()
            .expect("commands array")
            .iter()
            .find(|e| e["name"] == name)
            .unwrap_or_else(|| panic!("command `{name}` missing from help"))["gate"]
            .as_str()
            .expect("gate string")
            .to_string()
    };
    assert_eq!(gate_of("health"), "read-only");
    assert_eq!(gate_of("orders"), "read-only");
    assert_eq!(gate_of("buy"), "write");
    assert_eq!(gate_of("option-combo"), "write");
    assert_eq!(gate_of("grid-tick"), "write-paper-only");
    assert_eq!(gate_of("sma-tick"), "write-paper-only");
}

/// Help must work with NO config file and NO gateway: point HOME at an empty
/// temp dir (no ~/.config/oh-my-ib/config.toml) and it still answers.
#[test]
fn help_needs_no_config_and_no_gateway() {
    let home = std::env::temp_dir().join(format!("omi-spec-help-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp HOME");
    let v = help_stdout_json(omi().env("HOME", &home));
    assert!(v["commands"].is_array());
}
