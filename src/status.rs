use std::path::Path;

use bstr::ByteSlice;
use failure::Error;
use git2::Repository;
use termcolor::WriteColor;

use crate::cli;
use crate::config::Config;
use crate::walk::walk_repos;

pub fn run(
    stdout: &mut impl WriteColor,
    _args: &cli::Args,
    status_args: &cli::StatusArgs,
    config: &Config,
) -> Result<(), Error> {
    walk_repos(config, |path, repo| {
        log::debug!("getting status for repo at {}", path.display());
        if let Err(err) = print_repo(stdout, config, &status_args, path, repo) {
            log::error!(
                "error getting status for repo at {}\ncaused by: {}",
                path.display(),
                err
            );
        }
    });
    Ok(())
}

const PATH_PADDING: usize = 40;

fn print_repo(
    stdout: &mut impl WriteColor,
    config: &Config,
    _status_args: &cli::StatusArgs,
    path: &Path,
    repo: &Repository,
) -> Result<(), Error> {
    let relative_path = path.strip_prefix(&config.root).unwrap_or(path);
    write!(
        stdout,
        "{:<pad$}",
        relative_path.display(),
        pad = PATH_PADDING
    )?;

    let head = repo.head()?;
    if repo.head_detached()? {
        write!(stdout, "{:.8}", head.name_bytes().as_bstr())?;
    } else {
        write!(stdout, "{}", head.shorthand_bytes().as_bstr())?;
    }

    stdout.reset()?;
    writeln!(stdout)?;
    Ok(())
}
