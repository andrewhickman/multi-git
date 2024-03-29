use std::borrow::Cow;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::sync::Mutex;

use clap::Parser;
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};
use serde::Serialize;

use crate::config::Config;
use crate::output::{self, LineContent, Output};
use crate::progress::ProgressBar;
use crate::walk::{self, walk_with_output};
use crate::{alias, cli, git};

#[derive(Debug, Parser)]
#[clap(about = "Pull changes in your repos")]
pub struct PullArgs {
    #[clap(
        value_name = "TARGET",
        help = "the path or alias of the repo(s) to pull"
    )]
    target: Option<String>,
    #[clap(long, help = "whether to switch to the default branch before pulling")]
    switch: bool,
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
        Cow::Borrowed(&*config.root)
    };

    walk_with_output(
        args,
        out,
        config,
        root,
        PullLineContent::build,
        |entry, line| PullLineContent::update(entry, line, pull_args.switch),
    )
}

pub(super) struct PullLineContent {
    relative_path: PathBuf,
    state: Mutex<PullState>,
}

enum PullState {
    Pending,
    Downloading(ProgressBar),
    Indexing(ProgressBar),
    Finished(crate::Result<git::PullOutcome>),
}

impl PullLineContent {
    pub fn new(relative_path: PathBuf) -> Self {
        PullLineContent {
            relative_path,
            state: Mutex::new(PullState::Pending),
        }
    }

    pub fn tick(&self, progress: git2::Progress<'_>) {
        self.state.lock().unwrap().tick(progress)
    }

    fn build<'out, 'block>(
        block: &'block output::Block<'out>,
        entry: &walk::Entry,
    ) -> output::Line<'out, 'block, Self> {
        block.add_line(PullLineContent::new(entry.relative_path.clone()))
    }

    fn update<'out, 'block>(
        entry: &walk::Entry,
        line: &output::Line<'out, 'block, Self>,
        switch: bool,
    ) {
        log::debug!("pulling repo at `{}`", entry.relative_path.display());

        let outcome = entry
            .repo
            .status(&entry.settings)
            .map_err(|err| crate::Error::with_context(err, "failed to get repo status"))
            .and_then(|(status, remote)| {
                entry
                    .repo
                    .pull(&entry.settings, &status, remote, switch, move |progress| {
                        line.content().tick(progress);
                        line.update();
                    })
            });

        *line.content().state.lock().unwrap() = PullState::Finished(outcome);
    }
}

impl PullState {
    pub fn tick(&mut self, progress: git2::Progress<'_>) {
        match *self {
            PullState::Pending => {
                *self = PullState::Downloading(ProgressBar::new());
            }
            PullState::Downloading(ref mut bar)
                if progress.received_objects() != progress.total_objects() =>
            {
                bar.set(progress.received_objects() as f64 / progress.total_objects() as f64);
            }
            PullState::Downloading(_) => {
                *self = PullState::Indexing(ProgressBar::new());
            }
            PullState::Indexing(ref mut bar) => {
                bar.set(progress.indexed_objects() as f64 / progress.total_objects() as f64);
            }
            PullState::Finished(_) => {}
        }
    }
}

impl LineContent for PullLineContent {
    fn write(&self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        crossterm::queue!(stdout, Clear(ClearType::CurrentLine))?;

        let (cols, _) = terminal::size()?;

        let relative_path = format!(
            "{:padding$}",
            self.relative_path.display(),
            padding = cols as usize / 2,
        );
        write!(stdout, "{}", relative_path)?;

        let remaining_cols = cols.saturating_sub(relative_path.len() as u16);
        let status_cols = 13;
        let bar_cols = remaining_cols.saturating_sub(status_cols);

        let state = self.state.lock().unwrap();
        match &*state {
            PullState::Pending => {}
            PullState::Downloading(progress) => {
                crossterm::queue!(stdout, SetForegroundColor(Color::Grey))?;
                write!(
                    stdout,
                    "{:padding$}",
                    "downloading:",
                    padding = status_cols as usize
                )?;
                crossterm::queue!(stdout, ResetColor)?;

                progress.write(stdout, bar_cols)?;
            }
            PullState::Indexing(progress) => {
                crossterm::queue!(stdout, SetForegroundColor(Color::Grey))?;
                write!(
                    stdout,
                    "{:padding$}",
                    "indexing:",
                    padding = status_cols as usize
                )?;
                crossterm::queue!(stdout, ResetColor)?;

                progress.write(stdout, bar_cols)?;
            }
            PullState::Finished(Ok(outcome)) => {
                crossterm::queue!(stdout, SetForegroundColor(Color::Green))?;

                match outcome {
                    git::PullOutcome::UpToDate(branch) => {
                        write!(stdout, "branch `{}` is up to date", branch)?
                    }
                    git::PullOutcome::CreatedUnborn(branch) => {
                        write!(stdout, "created branch `{}`", branch)?
                    }
                    git::PullOutcome::FastForwarded(branch) => {
                        write!(stdout, "fast-forwarded branch `{}`", branch)?
                    }
                }

                crossterm::queue!(stdout, ResetColor)?;
            }
            PullState::Finished(Err(err)) => err.write(stdout)?,
        }

        Ok(())
    }

    fn write_json(&self, stdout: &mut io::StdoutLock) -> serde_json::Result<()> {
        #[derive(Serialize)]
        #[serde(tag = "kind", rename_all = "snake_case")]
        enum JsonPull<'a> {
            Pull {
                path: String,
                #[serde(flatten)]
                outcome: &'a git::PullOutcome,
            },
            Error {
                path: String,
                #[serde(flatten)]
                error: &'a crate::Error,
            },
        }

        let state = self.state.lock().unwrap();

        let json = match &*state {
            PullState::Pending | PullState::Downloading(_) | PullState::Indexing(_) => {
                unreachable!()
            }
            PullState::Finished(Ok(outcome)) => JsonPull::Pull {
                path: self.relative_path.display().to_string(),
                outcome,
            },
            PullState::Finished(Err(error)) => JsonPull::Error {
                path: self.relative_path.display().to_string(),
                error,
            },
        };

        serde_json::to_writer(stdout, &json)
    }
}
