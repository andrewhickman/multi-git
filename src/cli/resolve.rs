use clap::Parser;

use crate::config::Config;
use crate::output::Output;
use crate::{alias, cli};

#[derive(Debug, Parser)]
#[clap(about = "Resolve a path or alias")]
pub struct ResolveArgs {
    #[clap(name = "TARGET", help = "the path or alias of the repo or folder")]
    target: String,
}

pub fn run(
    out: &Output,
    args: &cli::Args,
    resolve_args: &ResolveArgs,
    config: &Config,
) -> crate::Result<()> {
    let path = alias::resolve(&resolve_args.target, args, config)?;
    out.writeln_message(path.display());
    Ok(())
}
