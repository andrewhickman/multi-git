use std::io::Write;

use structopt::StructOpt;

use crate::config::Config;
use crate::output::Output;
use crate::{alias, cli};

#[derive(Debug, StructOpt)]
#[structopt(about = "Resolve a path or alias", no_version)]
pub struct ResolveArgs {
    #[structopt(name = "TARGET", help = "the path or alias of the repo or folder")]
    target: String,
}

pub fn run(
    out: &Output,
    args: &cli::Args,
    resolve_args: &ResolveArgs,
    config: &Config,
) -> crate::Result<()> {
    let path = alias::resolve(&resolve_args.target, args, config)?;
    out.writeln(|stdout| {
        write!(stdout, "{}", path.display())?;
        Ok(())
    })?;
    Ok(())
}
