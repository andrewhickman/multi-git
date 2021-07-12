mod setup;

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

macro_rules! status_test {
    ($name:ident, $expected:expr) => {
        #[test]
        fn $name() {
            run_status_test(stringify!($name), $expected);
        }
    };
}

status_test!(
    empty,
    r#"{"kind":"status","head":{"name":"main","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    empty_branch,
    r#"{"kind":"status","head":{"name":"topic","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    on_main,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    on_branch,
    r#"{"kind":"status","head":{"name":"topic","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    detached,
    r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    detached_branch,
    r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    detached_branch_ahead,
    r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    detached_tag,
    r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    detached_tag_ahead,
    r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    index_changed,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":true},"default_branch":null}"#
);
status_test!(
    index_added,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":true},"default_branch":null}"#
);
status_test!(
    working_tree_changed,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":true,"index_changed":false},"default_branch":null}"#
);
status_test!(
    working_tree_added,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    upstream,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"upstream","ahead":0,"behind":0},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":"main"}"#
);
status_test!(
    upstream_behind,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"upstream","ahead":0,"behind":1},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":"main"}"#
);
status_test!(
    upstream_ahead,
    r#"{"kind":"status","head":{"name":"main","kind":"branch"},"upstream":{"state":"upstream","ahead":1,"behind":0},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":"main"}"#
);
status_test!(
    upstream_empty,
    r#"{"kind":"status","head":{"name":"main","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":null}"#
);
status_test!(
    upstream_local_empty,
    r#"{"kind":"status","head":{"name":"main","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":"main"}"#
);
status_test!(
    upstream_local_empty_on_branch,
    r#"{"kind":"status","head":{"name":"topic","kind":"unborn"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":"main"}"#
);
status_test!(
    upstream_detached,
    r#"{"kind":"status","head":{"name":"*","kind":"detached"},"upstream":{"state":"none"},"working_tree":{"working_changed":false,"index_changed":false},"default_branch":"main"}"#
);

fn run_status_test(name: &str, expected: &str) {
    let context = setup::run(
        &fs_err::read_to_string(Path::new("tests/setup").join(name).with_extension("setup"))
            .unwrap(),
    );

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("status")
        .current_dir(context.working_dir())
        .assert()
        .success()
        .stdout(output_pred(expected));
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
