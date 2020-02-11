use std::borrow::Cow;
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::Error;
use git2::Repository;
use structopt::StructOpt;
use termcolor::StandardStream;

use crate::config::{Config, Settings};
use crate::walk::walk_repos;
use crate::{alias, cli, print_utils};

#[derive(Debug, StructOpt)]
#[structopt(about = "Pull changes in your repos")]
pub struct PullArgs {
    #[structopt(help = "the path or alias of the repo to pull")]
    name: Option<String>,
}

pub fn run(
    stdout: &StandardStream,
    args: &cli::Args,
    pull_args: &PullArgs,
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
        |path, (), settings, repo| visit_repo(stdout, config, path, settings, repo),
    );
    Ok(())
}

fn visit_dir(stdout: &StandardStream, path: &Path, repos: &[(PathBuf, Settings, Repository)]) {
    if !repos.is_empty() && !path.as_os_str().is_empty() {
        print_utils::print_dir(&mut stdout.lock(), path)
            .unwrap_or_else(print_utils::handle_print_error);
    }
}

fn visit_repo(
    stdout: &StandardStream,
    config: &Config,
    path: &Path,
    settings: &Settings,
    _repo: &mut Repository,
) {
    writeln!(stdout.lock(), "{} {:#?}", path.display(), settings).unwrap();
}
