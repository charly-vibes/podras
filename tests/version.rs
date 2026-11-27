//! Purpose: integration tests for the podras version/help CLI surface.
//!
//! Responsibilities:
//! - assert `podras --version` matches the crate version;
//! - assert `podras --version --json` emits a genesis envelope;
//! - assert `podras --help` advertises the v1 subcommands.
//!
//! Rationale: the version contract is the smallest observable behavior of the
//! binary, so it drives the TDD red stage before any command logic exists.

use assert_cmd::Command;

fn podras() -> Command {
    Command::cargo_bin("podras").expect("podras binary is built for tests")
}

#[test]
fn plain_version_prints_name_and_version() {
    let stdout = podras()
        .arg("--version")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(stdout).expect("version output is UTF-8");
    assert!(
        stdout.contains(concat!("podras ", env!("CARGO_PKG_VERSION"))),
        "version output is missing the name and version: {stdout}"
    );
}

#[test]
fn version_json_emits_genesis_envelope() {
    let output = podras()
        .args(["--version", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("version envelope is UTF-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("version envelope is JSON");
    assert_eq!(value["cli_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["envelope_kind"], "version");
}

#[test]
fn help_lists_v1_subcommands() {
    let help = podras()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(help).expect("help output is UTF-8");
    for subcommand in ["brief", "focus", "park", "init", "schema"] {
        assert!(
            help.contains(subcommand),
            "--help output is missing `{subcommand}`:\n{help}"
        );
    }
}
