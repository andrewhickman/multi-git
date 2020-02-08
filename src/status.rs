use std::io;
use std::path::{Path, PathBuf};

use failure::Error;
use git2::Repository;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::config::Config;
use crate::walk::walk_repos;
use crate::{cli, git_util};

pub fn run(
    stdout: &StandardStream,
    _args: &cli::Args,
    status_args: &cli::StatusArgs,
    config: &Config,
) -> Result<(), Error> {
    walk_repos(
        config,
        |path, repos| visit_dir(stdout, path, repos),
        |path, init, repo| visit_repo(stdout, status_args, path, init, repo),
    );
    Ok(())
}

fn visit_dir(stdout: &StandardStream, path: &Path, repos: &[(PathBuf, Repository)]) -> usize {
    if !repos.is_empty() {
        if !path.as_os_str().is_empty() {
            if let Err(err) = print_dir(&mut stdout.lock(), path) {
                log::error!("failed to write to stdout\ncaused by: {}", err);
            }
        }
    }

    repos
        .iter()
        .map(|(path, _)| path.as_os_str().len())
        .max()
        .unwrap_or(0)
}

fn print_dir(stdout: &mut impl WriteColor, path: &Path) -> io::Result<()> {
    stdout.set_color(
        &ColorSpec::new()
            .set_fg(Some(Color::Yellow))
            .set_bold(true)
            .set_underline(true),
    )?;
    write!(stdout, "{}", path.display())?;
    stdout.reset()?;
    writeln!(stdout)
}

fn visit_repo(
    stdout: &StandardStream,
    _status_args: &cli::StatusArgs,
    path: &Path,
    &repo_path_padding: &usize,
    repo: &mut Repository,
) {
    log::debug!("getting status for repo at `{}`", path.display());

    let status = match git_util::get_status(repo) {
        Ok(status) => status,
        Err(err) => {
            return log::error!(
                "failed to get repo status for `{}`\ncaused by: {}",
                path.display(),
                err.message()
            )
        }
    };

    if let Err(err) = print_status(&mut stdout.lock(), path, repo_path_padding, &status) {
        log::error!("failed to write to stdout\ncaused by: {}", err);
    }
}

fn print_status(
    stdout: &mut impl WriteColor,
    path: &Path,
    repo_path_padding: usize,
    status: &git_util::Status,
) -> io::Result<()> {
    write!(stdout, "{:<pad$} ", path.display(), pad = repo_path_padding,)?;

    match status.upstream {
        git_util::UpstreamStatus::None => write!(stdout, "        ")?,
        git_util::UpstreamStatus::Upstream {
            ahead: 0,
            behind: 0,
        } => {
            stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Cyan)))?;
            write!(stdout, "      ≡ ")?;
            stdout.reset()?;
        }
        git_util::UpstreamStatus::Upstream { ahead, behind: 0 } => {
            stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Green)))?;
            write!(stdout, "    {:>2}↑ ", ahead)?;
            stdout.reset()?;
        }
        git_util::UpstreamStatus::Upstream { ahead: 0, behind } => {
            stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Red)))?;
            write!(stdout, "    {:>2}↓ ", behind)?;
            stdout.reset()?;
        }
        git_util::UpstreamStatus::Upstream { ahead, behind } => {
            stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Yellow)))?;
            write!(stdout, "{:2}↑ {:2}↓ ", ahead, behind)?;
            stdout.reset()?;
        }
    }

    stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Cyan)))?;
    write!(stdout, "{}", status.head)?;
    stdout.reset()?;

    writeln!(stdout)
}
