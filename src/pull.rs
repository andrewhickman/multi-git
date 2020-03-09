use std::borrow::Cow;
use std::io::Write;

use crossterm::style::{Color, Colorize, ResetColor, SetForegroundColor};
use structopt::StructOpt;

use crate::config::Config;
use crate::output::{self, Output};
use crate::walk::{self, walk};
use crate::{alias, cli, git, progress};

#[derive(Debug, StructOpt)]
#[structopt(about = "Pull changes in your repos", no_version)]
pub struct PullArgs {
    #[structopt(value_name = "TARGET", help = "the path or alias of the repo to pull")]
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

    let mut status = entry
        .repo
        .status()
        .map_err(|err| crate::Error::with_context(err, "failed to get repo status"))?;

    let mut fetch_state = FetchState::Pending(line);
    let outcome = entry
        .repo
        .pull(&entry.settings, &mut status, move |progress| {
            fetch_state.tick(progress)?;
            Ok(true)
        })?;

    line.write(|stdout| {
        crossterm::queue!(stdout, SetForegroundColor(Color::Green))?;
        match outcome {
            git::PullOutcome::UpToDate => {
                write!(stdout, "branch `{}` is up to date", status.head.name)?
            }
            git::PullOutcome::CreatedUnborn => {
                write!(stdout, "created branch `{}`", status.head.name)?
            }
            git::PullOutcome::FastForwarded => {
                write!(stdout, "fast-forwarded branch `{}`", status.head.name)?
            }
        }
        crossterm::queue!(stdout, ResetColor)?;
        Ok(())
    })
}

#[derive(Clone, Debug)]
enum FetchState<'out, 'block> {
    Pending(output::Line<'out, 'block>),
    Downloading(progress::ProgressBar<'out, 'block>),
    Indexing(progress::ProgressBar<'out, 'block>),
}

impl<'out, 'block> FetchState<'out, 'block> {
    fn tick(&mut self, progress: git2::Progress<'_>) -> crate::Result<()> {
        const STATUS_COLS: u16 = 13;

        match *self {
            FetchState::Pending(ref line) => {
                *self = FetchState::Downloading(line.write_progress(STATUS_COLS, |stdout| {
                    write!(stdout, "{}", "downloading:".grey())?;
                    Ok(())
                })?);
            }
            FetchState::Downloading(ref bar)
                if progress.received_objects() != progress.total_objects() =>
            {
                bar.set(progress.received_objects() as f64 / progress.total_objects() as f64)?;
            }
            FetchState::Downloading(ref mut bar) => {
                let line = bar.finish()?;
                *self = FetchState::Indexing(line.write_progress(STATUS_COLS, |stdout| {
                    write!(stdout, "{}", "indexing:   ".grey())?;
                    Ok(())
                })?);
            }
            FetchState::Indexing(ref bar) => {
                bar.set(progress.indexed_objects() as f64 / progress.total_objects() as f64)?
            }
        };
        Ok(())
    }
}
