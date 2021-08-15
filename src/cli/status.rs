use std::borrow::Cow;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use clap::Clap;
use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};
use serde::Serialize;

use crate::config::Config;
use crate::output::{self, LineContent, Output};
use crate::walk::{self, walk_with_output};
use crate::{alias, cli, git};

#[derive(Debug, Clap)]
#[clap(about = "Show the status of your repos")]
pub struct StatusArgs {
    #[clap(
        value_name = "TARGET",
        about = "the path or alias of the repo(s) to get status for"
    )]
    target: Option<String>,
}

pub fn run(
    out: &Output,
    args: &cli::Args,
    status_args: &StatusArgs,
    config: &Config,
) -> crate::Result<()> {
    let root = if let Some(name) = &status_args.target {
        Cow::Owned(alias::resolve(name, args, config)?)
    } else {
        Cow::Borrowed(&*config.root)
    };

    walk_with_output(
        args,
        out,
        config,
        root,
        StatusLineContent::build,
        StatusLineContent::update,
    )
}

struct StatusLineContent {
    relative_path: PathBuf,
    state: Mutex<Option<crate::Result<git::RepositoryStatus>>>,
}

impl StatusLineContent {
    fn build<'out, 'block>(
        block: &'block output::Block<'out>,
        entry: &walk::Entry,
    ) -> output::Line<'out, 'block, Self> {
        block.add_line(StatusLineContent {
            relative_path: entry.relative_path.clone(),
            state: Mutex::new(None),
        })
    }

    fn update<'out, 'block>(entry: &walk::Entry, line: &output::Line<'out, 'block, Self>) {
        let status_result = entry.repo.status(&entry.settings).map(|(status, _)| status);
        *line.content().state.lock().unwrap() = Some(status_result);
    }
}

impl LineContent for StatusLineContent {
    fn write(&self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        crossterm::queue!(stdout, Clear(ClearType::CurrentLine))?;

        let (cols, _) = terminal::size()?;

        write!(
            stdout,
            "{:padding$} ",
            self.relative_path.display(),
            padding = cols as usize / 2
        )?;

        let status = self.state.lock().unwrap();
        match &*status {
            Some(Ok(status)) => {
                let (text, color) = match status.upstream {
                    git::UpstreamStatus::None => (String::new(), Color::Reset),
                    git::UpstreamStatus::Gone => ("×".to_owned(), Color::Red),
                    git::UpstreamStatus::Upstream {
                        ahead: 0,
                        behind: 0,
                    } => ("≡".to_owned(), Color::DarkCyan),
                    git::UpstreamStatus::Upstream { ahead, behind: 0 } => {
                        (format!("{}↑", ahead), Color::Green)
                    }
                    git::UpstreamStatus::Upstream { ahead: 0, behind } => {
                        (format!("{}↓", behind), Color::Red)
                    }
                    git::UpstreamStatus::Upstream { ahead, behind } => {
                        (format!("{}↓ {}↑", behind, ahead), Color::Yellow)
                    }
                };
                crossterm::queue!(stdout, SetForegroundColor(color))?;
                write!(stdout, "{:>8} ", text)?;
                stdout.flush()?;
                crossterm::queue!(stdout, ResetColor)?;

                if status.working_tree.working_changed {
                    crossterm::queue!(
                        stdout,
                        SetForegroundColor(Color::Red),
                        SetAttribute(Attribute::Bold)
                    )?;
                    write!(stdout, "! ")?;
                    crossterm::queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
                } else if status.working_tree.index_changed {
                    crossterm::queue!(stdout, SetForegroundColor(Color::Cyan),)?;
                    write!(stdout, "~ ")?;
                    crossterm::queue!(stdout, ResetColor)?;
                } else {
                    write!(stdout, "  ")?;
                }

                crossterm::queue!(stdout, SetForegroundColor(Color::DarkCyan))?;
                if !status.on_default_branch() {
                    crossterm::queue!(stdout, SetAttribute(Attribute::Bold))?;
                }
                write!(stdout, "{}", status.head)?;
                stdout.flush()?;
                crossterm::queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
            }
            Some(Err(err)) => {
                err.write(stdout)?;
            }
            None => {}
        }

        Ok(())
    }

    fn write_json(&self, stdout: &mut io::StdoutLock) -> serde_json::Result<()> {
        #[derive(Serialize)]
        #[serde(tag = "kind", rename_all = "snake_case")]
        enum JsonStatus<'a> {
            Status {
                path: String,
                #[serde(flatten)]
                status: &'a git::RepositoryStatus,
            },
            Error {
                path: String,
                #[serde(flatten)]
                error: &'a crate::Error,
            },
        }

        let state = self.state.lock().unwrap();

        let json = match &*state {
            None => unreachable!(),
            Some(Ok(status)) => JsonStatus::Status {
                path: self.relative_path.display().to_string(),
                status,
            },
            Some(Err(error)) => JsonStatus::Error {
                path: self.relative_path.display().to_string(),
                error,
            },
        };

        serde_json::to_writer(stdout, &json)
    }
}
