mod clone;
mod edit;
mod exec;
mod pull;
mod resolve;
mod status;

pub use self::clone::{run as clone, CloneArgs};
pub use self::edit::{run as edit, EditArgs};
pub use self::exec::{run as exec, ExecArgs};
pub use self::pull::{run as pull, PullArgs};
pub use self::resolve::{run as resolve, ResolveArgs};
pub use self::status::{run as status, StatusArgs};

use structopt::clap::AppSettings;
use structopt::StructOpt;

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
    Edit(EditArgs),
    #[structopt(name = "status", no_version)]
    Status(StatusArgs),
    #[structopt(name = "pull", no_version)]
    Pull(PullArgs),
    #[structopt(name = "resolve", no_version)]
    Resolve(ResolveArgs),
    #[structopt(name = "exec", no_version)]
    Exec(ExecArgs),
    #[structopt(name = "clone", no_version)]
    Clone(CloneArgs),
}
