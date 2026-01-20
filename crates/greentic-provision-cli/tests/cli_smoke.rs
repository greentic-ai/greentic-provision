use assert_cmd::Command;
use predicates::prelude::*;

fn fixture_pack() -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests/fixtures/packs/noop-provision.gtpack");
    path.to_string_lossy().to_string()
}

#[test]
fn pack_inspect_smoke() {
    let pack = fixture_pack();
    let bin = assert_cmd::cargo::cargo_bin!("greentic-provision");
    Command::new(bin)
        .args(["pack", "inspect", "--pack", &pack])
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup entry flow"));
}

#[test]
fn dry_run_noop_smoke() {
    let pack = fixture_pack();
    let bin = assert_cmd::cargo::cargo_bin!("greentic-provision");
    Command::new(bin)
        .args([
            "dry-run",
            "setup",
            "--pack",
            &pack,
            "--executor",
            "noop",
            "--provider-id",
            "noop",
            "--install-id",
            "noop",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry-run completed"));
}
