use structopt::clap::AppSettings;
use structopt::StructOpt;

use crate::{edit, pull, resolve, status};

pub fn parse_args() -> Args {
    Args::from_args()
}

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Utility for managing multiple git repos",
    bin_name = "mgit",
    no_version
)]
#[structopt(global_setting = AppSettings::DisableVersion)]
#[structopt(global_setting = AppSettings::UnifiedHelpMessage)]
#[structopt(global_setting = AppSettings::VersionlessSubcommands)]
pub struct Args {
    #[structopt(subcommand)]
    pub command: Command,
    #[structopt(long, short = "A", help = "Disable aliases")]
    pub no_alias: bool,
}

#[derive(Debug, StructOpt)]
#[structopt(no_version)]
pub enum Command {
    #[structopt(name = "edit", no_version)]
    Edit(edit::EditArgs),
    #[structopt(name = "status", no_version)]
    Status(status::StatusArgs),
    #[structopt(name = "pull", no_version)]
    Pull(pull::PullArgs),
    #[structopt(name = "resolve", no_version)]
    Resolve(resolve::ResolveArgs),
}
