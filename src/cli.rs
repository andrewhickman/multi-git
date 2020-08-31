use structopt::clap::AppSettings;
use structopt::StructOpt;

use crate::{edit, exec, pull, resolve, status};

pub fn parse_args() -> Args {
    Args::from_args()
}

const VERSION: &str = env!("VERGEN_SHA_SHORT");
const LONG_VERSION: &str = env!("VERGEN_SHA");

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Utility for managing multiple git repos",
    bin_name = "mgit",
    version = VERSION,
    long_version = LONG_VERSION,
)]
#[structopt(version = VERSION, long_version = LONG_VERSION)]
#[structopt(global_setting = AppSettings::UnifiedHelpMessage)]
#[structopt(global_setting = AppSettings::VersionlessSubcommands)]
pub struct Args {
    #[structopt(subcommand)]
    pub command: Command,
    #[structopt(long, short = "A", help = "Disable aliases")]
    pub no_alias: bool,
    #[structopt(
        long,
        short,
        help = "Number of threads to use. If set to 0, uses the number of available CPUs",
        default_value = "0"
    )]
    pub jobs: usize,
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
    #[structopt(name = "exec", no_version)]
    Exec(exec::ExecArgs),
}
