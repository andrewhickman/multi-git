mod setup;

use std::path::Path;

use assert_cmd::Command;
use assert_fs::{TempDir, prelude::*};
use predicates::prelude::*;

macro_rules! pull_test {
    ($name:ident, $expected:expr) => {
        pull_test!($name, $expected, |_| {});
    };
    ($name:ident, $expected:expr, $fs_asserts:expr) => {
        #[test]
        fn $name() {
            run_pull_test(stringify!($name), $expected, $fs_asserts);
        }
    };
}

pull_test!(
    empty,
    r#"{"kind":"error","message":"no remotes","source":null}"#
);
pull_test!(
    upstream_working_tree_added,
    r#"{"kind":"pull","state":"fast_forwarded","branch":"main"}"#,
    |path| {
        path.child("local/file.txt").assert("changed");
    }
);
pull_test!(
    upstream_working_tree_overwrite,
    r#"{"kind":"error","message":"1 conflict prevents checkout","source":null}"#,
    |path| {
        path.child("local/file.txt").assert("original");
    }
);

fn run_pull_test(name: &str, expected: &str, fs_asserts: impl FnOnce(&TempDir)) {
    let context = setup::run(
        &fs_err::read_to_string(Path::new("tests/setup").join(name).with_extension("setup"))
            .unwrap(),
    );

    Command::cargo_bin("mgit")
        .unwrap()
        .arg("--json")
        .arg("pull")
        .current_dir(context.working_dir())
        .assert()
        .success()
        .stdout(output_pred(expected));

    fs_asserts(context.temp_dir());
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
