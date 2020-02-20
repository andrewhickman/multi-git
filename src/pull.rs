use std::borrow::Cow;
use std::io::Write;

use crossterm::style::Colorize;
use structopt::StructOpt;

use crate::config::Config;
use crate::output::{self, Output};
use crate::walk::{self, walk};
use crate::{alias, cli};

#[derive(Debug, StructOpt)]
#[structopt(about = "Pull changes in your repos", no_version)]
pub struct PullArgs {
    #[structopt(name = "TARGET", help = "the path or alias of the repo to pull")]
    target: Option<String>,
}

pub fn run(
    out: &Output,
    args: &cli::Args,
    pull_args: &PullArgs,
    config: &Config,
) -> crate::Result<()> {
    let root = if let Some(name) = &pull_args.target {
        Cow::Owned(alias::resolve(name, args, config)?)
    } else {
        Cow::Borrowed(&config.root)
    };

    walk(out, config, &root, visit_repo);
    Ok(())
}

fn visit_repo(line: output::Line<'_, '_>, entry: &walk::Entry) -> crate::Result<()> {
    log::debug!("pulling repo at `{}`", entry.relative_path.display());

    let status = entry
        .repo
        .status()
        .map_err(|err| crate::Error::with_context(err, "failed to get repo status"))?;

    const STATUS_COLS: u16 = 13;

    let mut state = FetchState::Downloading;
    let mut bar = line.write_progress(STATUS_COLS, |stdout| {
        write!(stdout, "{}", "downloading:".grey())?;
        Ok(())
    })?;
    entry.repo.pull(&entry.settings, &status, |progress| {
        if state == FetchState::Downloading && progress.indexed_objects() != 0 {
            bar.finish()?;
            bar = line.write_progress(STATUS_COLS, |stdout| {
                write!(stdout, "{}", "indexing:".grey())?;
                Ok(())
            })?;
            state = FetchState::Indexing;
        }

        match state {
            FetchState::Downloading => {
                bar.set(progress.received_objects() as f64 / progress.total_objects() as f64)?
            }
            FetchState::Indexing => {
                bar.set(progress.indexed_objects() as f64 / progress.total_objects() as f64)?
            }
        }
        Ok(true)
    })?;
    drop(bar);

    line.write(|stdout| {
        write!(stdout, "{}", "pull successful".green())?;
        Ok(())
    })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FetchState {
    Downloading,
    Indexing,
}
