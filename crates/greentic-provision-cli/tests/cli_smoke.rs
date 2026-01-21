use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

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

#[test]
fn pack_inspect_cbor_manifest_aliases() {
    let dir = tempdir().expect("tempdir");
    let manifest_path = dir.path().join("manifest.cbor");
    let manifest_value = serde_json::json!({
        "pack_id": "noop-pack",
        "pack_version": "0.1.0",
        "meta": { "entry_flows": { "setup": "setup" } },
        "flows": [{ "id": "setup", "entry": "setup" }]
    });
    let mut file = std::fs::File::create(&manifest_path).expect("manifest");
    ciborium::ser::into_writer(&manifest_value, &mut file).expect("write cbor");

    let bin = assert_cmd::cargo::cargo_bin!("greentic-provision");
    Command::new(bin)
        .args([
            "pack",
            "inspect",
            "--pack",
            dir.path().to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup entry flow"));
}

#[test]
fn pack_inspect_cbor_nested_pack_manifest() {
    let dir = tempdir().expect("tempdir");
    let manifest_path = dir.path().join("manifest.cbor");
    let manifest_value = serde_json::json!({
        "pack": {
            "pack_id": "nested-pack",
            "pack_version": "0.1.0",
            "meta": { "entry_flows": { "setup": "setup" } },
            "flows": [{ "id": "setup", "entry": "setup" }]
        }
    });
    let mut file = std::fs::File::create(&manifest_path).expect("manifest");
    ciborium::ser::into_writer(&manifest_value, &mut file).expect("write cbor");

    let bin = assert_cmd::cargo::cargo_bin!("greentic-provision");
    Command::new(bin)
        .args([
            "pack",
            "inspect",
            "--pack",
            dir.path().to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup entry flow"));
}

#[test]
fn pack_inspect_cbor_pack_id_indexed_symbols() {
    let dir = tempdir().expect("tempdir");
    let manifest_path = dir.path().join("manifest.cbor");
    let manifest_value = serde_json::json!({
        "pack_id": 0,
        "version": "0.4.18",
        "symbols": { "pack_ids": ["messaging-telegram"] },
        "flows": [
            { "id": "setup_default", "entrypoints": ["setup"] }
        ]
    });
    let mut file = std::fs::File::create(&manifest_path).expect("manifest");
    ciborium::ser::into_writer(&manifest_value, &mut file).expect("write cbor");

    let bin = assert_cmd::cargo::cargo_bin!("greentic-provision");
    Command::new(bin)
        .args([
            "pack",
            "inspect",
            "--pack",
            dir.path().to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup entry flow"));
}

#[test]
fn pack_inspect_cbor_prefers_id_over_pack_id() {
    let dir = tempdir().expect("tempdir");
    let manifest_path = dir.path().join("manifest.cbor");
    let manifest_value = serde_json::json!({
        "id": "explicit-pack",
        "pack_id": "legacy-pack",
        "version": "0.1.0",
        "pack_version": "0.0.1",
        "meta": { "entry_flows": { "setup": "setup" } },
        "flows": [{ "id": "setup", "entrypoints": ["setup"] }]
    });
    let mut file = std::fs::File::create(&manifest_path).expect("manifest");
    ciborium::ser::into_writer(&manifest_value, &mut file).expect("write cbor");

    let bin = assert_cmd::cargo::cargo_bin!("greentic-provision");
    Command::new(bin)
        .args([
            "pack",
            "inspect",
            "--pack",
            dir.path().to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup entry flow"));
}
