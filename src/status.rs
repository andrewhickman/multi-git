use std::borrow::Cow;
use std::io::Write;

use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};
use structopt::StructOpt;

use crate::config::Config;
use crate::output::{self, Output};
use crate::walk::{self, walk};
use crate::{alias, cli, git};

#[derive(Debug, StructOpt)]
#[structopt(about = "Show the status of your repos", no_version)]
pub struct StatusArgs {
    #[structopt(
        name = "TARGET",
        help = "the path or alias of the repo to get status for"
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
        Cow::Borrowed(&config.root)
    };

    walk(out, config, &root, visit_repo);
    Ok(())
}

fn visit_repo(line: output::Line<'_, '_>, entry: &walk::Entry) -> crate::Result<()> {
    log::debug!(
        "getting status for repo at `{}`",
        entry.relative_path.display()
    );

    let status = entry
        .repo
        .status()
        .map_err(|err| crate::Error::with_context(err, "failed to get repo status"))?;

    line.write(|stdout| {
        let (text, color) = match status.upstream {
            git::UpstreamStatus::None => (String::new(), Color::Reset),
            git::UpstreamStatus::Gone => ("×".to_owned(), Color::Red),
            git::UpstreamStatus::Upstream {
                ahead: 0,
                behind: 0,
            } => ("≡".to_owned(), Color::Cyan),
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
        crossterm::queue!(stdout, ResetColor)?;

        if status.working_tree.working_changed {
            crossterm::queue!(
                stdout,
                SetForegroundColor(Color::Red),
                SetAttribute(Attribute::Bold)
            )?;
            write!(stdout, "! ")?;
            crossterm::queue!(stdout, SetAttribute(Attribute::NoBold))?;
        } else if status.working_tree.index_changed {
            crossterm::queue!(stdout, SetForegroundColor(Color::Cyan))?;
            write!(stdout, "~ ")?;
        } else {
            write!(stdout, "  ")?;
        }

        crossterm::queue!(stdout, SetForegroundColor(Color::Cyan))?;
        if !status.head.on_default_branch(&entry.settings) {
            crossterm::queue!(stdout, SetAttribute(Attribute::Bold))?;
        }
        write!(stdout, "{}", status.head)?;
        crossterm::queue!(stdout, ResetColor, SetAttribute(Attribute::NoBold))?;

        Ok(())
    })
}
