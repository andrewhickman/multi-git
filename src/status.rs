use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};

use failure::Error;
use git2::Repository;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::config::Config;
use crate::walk::walk_repos;
use crate::{alias, cli, git_util};

pub fn run(
    stdout: &StandardStream,
    args: &cli::Args,
    status_args: &cli::StatusArgs,
    config: &Config,
) -> Result<(), Error> {
    let root = if let Some(name) = &status_args.name {
        Cow::Owned(alias::resolve(name, args, config)?)
    } else {
        Cow::Borrowed(&config.root)
    };

    walk_repos(
        &root,
        |path, repos| visit_dir(stdout, path, repos),
        |path, init, repo| visit_repo(stdout, path, init, repo),
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
    status: &git_util::RepoStatus,
) -> io::Result<()> {
    write!(stdout, "{:<pad$} ", path.display(), pad = repo_path_padding,)?;

    let (text, color) = match status.upstream {
        git_util::UpstreamStatus::None => (String::new(), None),
        git_util::UpstreamStatus::Upstream {
            ahead: 0,
            behind: 0,
        } => ("≡".to_owned(), Some(Color::Cyan)),
        git_util::UpstreamStatus::Upstream { ahead, behind: 0 } => {
            (format!("{}↑", ahead), Some(Color::Green))
        }
        git_util::UpstreamStatus::Upstream { ahead: 0, behind } => {
            (format!("{}↓", behind), Some(Color::Red))
        }
        git_util::UpstreamStatus::Upstream { ahead, behind } => {
            (format!("{}↑ {}↓", ahead, behind), Some(Color::Yellow))
        }
    };
    stdout.set_color(&ColorSpec::new().set_fg(color))?;
    write!(stdout, "{:>8} ", text)?;
    stdout.reset()?;

    if status.working_tree.working_changed {
        stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        write!(stdout, "! ")?;
        stdout.reset()?;
    } else if status.working_tree.index_changed {
        stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Cyan)))?;
        write!(stdout, "~ ")?;
        stdout.reset()?;
    } else {
        write!(stdout, "  ")?;
    }

    stdout.set_color(&ColorSpec::new().set_fg(Some(Color::Cyan)))?;
    write!(stdout, "{}", status.head)?;
    stdout.reset()?;

    writeln!(stdout)
}
