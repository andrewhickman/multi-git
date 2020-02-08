use std::io;
use std::path::Path;

use bstr::{BString, ByteSlice};
use failure::Error;
use git2::Repository;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::cli;
use crate::config::Config;
use crate::walk::walk_repos;

pub fn run(
    stdout: &StandardStream,
    _args: &cli::Args,
    status_args: &cli::StatusArgs,
    config: &Config,
) -> Result<(), Error> {
    walk_repos(
        config,
        |path| visit_dir(stdout, config, path),
        |path, repo| visit_repo(stdout, config, status_args, path, repo),
    );
    Ok(())
}

fn visit_dir(stdout: &StandardStream, config: &Config, path: &Path) {
    let relative_path = path.strip_prefix(&config.root).unwrap_or(path);

    if !relative_path.as_os_str().is_empty() {
        if let Err(err) = print_dir(&mut stdout.lock(), relative_path) {
            log::error!("failed to write to stdout\ncaused by: {}", err);
        }
    }
}

fn print_dir(stdout: &mut impl WriteColor, relative_path: &Path) -> io::Result<()> {
    stdout
        .set_color(
            &ColorSpec::new()
                .set_fg(Some(Color::Yellow))
                .set_underline(true),
        )
        .ok();
    write!(stdout, "{}", relative_path.display())?;
    stdout.reset().ok();
    writeln!(stdout)
}

const REPO_PATH_PADDING: usize = 40;

fn visit_repo(
    stdout: &StandardStream,
    config: &Config,
    _status_args: &cli::StatusArgs,
    path: &Path,
    repo: &mut Repository,
) {
    log::debug!("getting status for repo at `{}`", path.display());
    let relative_path = path.strip_prefix(&config.root).unwrap_or(path);

    let status = match get_status(repo) {
        Ok(status) => status,
        Err(err) => {
            return log::error!(
                "failed to get repo status for `{}`\ncaused by: {}",
                relative_path.display(),
                err.message()
            )
        }
    };

    if let Err(err) = print_status(&mut stdout.lock(), relative_path, &status) {
        log::error!("failed to write to stdout\ncaused by: {}", err);
    }
}

struct Status {
    head: BString,
    detached: bool,
}

fn get_status<'a>(repo: &'a mut Repository) -> Result<Status, git2::Error> {
    let head = repo.head()?;
    let detached = repo.head_detached()?;
    let pretty_head = if detached {
        let object = head.peel(git2::ObjectType::Any)?;
        object
            .short_id()?
            .as_str()
            .expect("oid is invalid utf-8")
            .into()
    } else {
        head.shorthand_bytes().as_bstr().to_owned()
    };
    Ok(Status {
        head: pretty_head,
        detached,
    })
}

fn print_status(
    stdout: &mut impl WriteColor,
    relative_path: &Path,
    status: &Status,
) -> io::Result<()> {
    write!(
        stdout,
        "{:<pad$}",
        relative_path.display(),
        pad = REPO_PATH_PADDING
    )?;

    stdout
        .set_color(&ColorSpec::new().set_fg(Some(Color::Cyan)))
        .ok();
    if status.detached {
        write!(stdout, "({})", status.head)?;
    } else {
        write!(stdout, "{}", status.head)?;
    }
    stdout.reset().ok();

    writeln!(stdout)
}
