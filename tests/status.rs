mod setup;

use assert_cmd::Command;

#[test]
fn empty() {
    let context = setup::run(include_str!("setup/empty.setup"));

    const EXPECTED: &str = r#"{"kind":"status","head":{"name":"main","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}
"#;
    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(EXPECTED);
}

#[test]
fn empty_branch() {
    let context = setup::run(include_str!("setup/empty_branch.setup"));

    const EXPECTED: &str = r#"{"kind":"status","head":{"name":"topic","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}
"#;
    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(EXPECTED);
}

#[test]
fn on_main() {
    let context = setup::run(include_str!("setup/on_main.setup"));

    const EXPECTED: &str = r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}
"#;
    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(EXPECTED);
}

#[test]
fn on_branch() {
    let context = setup::run(include_str!("setup/on_main.setup"));

    const EXPECTED: &str = r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}
"#;
    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(EXPECTED);
}