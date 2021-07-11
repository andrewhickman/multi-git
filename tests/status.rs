mod setup;

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn empty() {
    let context = setup::run(include_str!("setup/empty.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"main","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn empty_branch() {
    let context = setup::run(include_str!("setup/empty_branch.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"topic","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn on_main() {
    let context = setup::run(include_str!("setup/on_main.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn on_branch() {
    let context = setup::run(include_str!("setup/on_branch.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"topic","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn detached() {
    let context = setup::run(include_str!("setup/detached.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn detached_branch() {
    let context = setup::run(include_str!("setup/detached_branch.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn detached_branch_ahead() {
    let context = setup::run(include_str!("setup/detached_branch_ahead.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn detached_tag() {
    let context = setup::run(include_str!("setup/detached_tag.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

#[test]
fn detached_tag_ahead() {
    let context = setup::run(include_str!("setup/detached_tag_ahead.setup"));

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.temp_dir())
        .assert()
        .success()
        .stdout(output_pred(r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#));
}

fn output_pred(expected: &str) -> impl Predicate<[u8]> {
    let regex = format!(
        "^{}$",
        regex::escape(&expected.replace("*", "__WILDCARD__")).replace("__WILDCARD__", ".*")
    );

    predicates::str::is_match(&regex)
        .unwrap()
        .trim()
        .from_utf8()
}
