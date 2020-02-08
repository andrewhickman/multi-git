use std::borrow::Cow;
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::Error;
use git2::Repository;
use termcolor::StandardStream;

use crate::config::Config;
use crate::walk::walk_repos;
use crate::{alias, cli, print_utils};

pub fn run(
    stdout: &StandardStream,
    args: &cli::Args,
    pull_args: &cli::PullArgs,
    config: &Config,
) -> Result<(), Error> {
    let root = if let Some(name) = &pull_args.name {
        Cow::Owned(alias::resolve(name, args, config)?)
    } else {
        Cow::Borrowed(&config.root)
    };

    walk_repos(
        config,
        &root,
        |path, repos| visit_dir(stdout, path, repos),
        |path, (), repo| visit_repo(stdout, config, path, repo),
    );
    Ok(())
}

fn visit_dir(stdout: &StandardStream, path: &Path, repos: &[(PathBuf, Repository)]) {
    if !repos.is_empty() {
        if !path.as_os_str().is_empty() {
            if let Err(err) = print_utils::print_dir(&mut stdout.lock(), path) {
                log::error!("failed to write to stdout\ncaused by: {}", err);
            }
        }
    }
}

fn visit_repo(stdout: &StandardStream, config: &Config, path: &Path, _repo: &mut Repository) {
    let settings = config.settings(path);
    writeln!(stdout.lock(), "{} {:#?}", path.display(), settings).unwrap();
}
