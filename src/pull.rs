use std::borrow::Cow;

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

fn visit_repo(_line: output::Line<'_, '_>, _entry: &walk::Entry) -> crate::Result<()> {
    Ok(())
}
