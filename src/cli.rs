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

use clap::{AppSettings, Clap};

pub fn parse_args() -> Args {
    Args::parse()
}

const VERSION: &str = env!("VERGEN_GIT_SHA");

#[derive(Debug, Clap)]
#[clap(
    about = "Utility for managing multiple git repos",
    bin_name = "mgit",
    version = VERSION,
)]
#[clap(global_setting = AppSettings::UnifiedHelpMessage)]
#[clap(global_setting = AppSettings::ColoredHelp)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
    #[clap(long, global = true, short = 'A', about = "Disable aliases")]
    pub no_alias: bool,
    #[clap(
        long,
        short,
        global = true,
        about = "Number of threads to use. If set to 0, uses the number of available CPUs",
        default_value = "0"
    )]
    pub jobs: usize,
    #[clap(long, global = true, about = "Print output in JSON Lines format")]
    pub json: bool,
}

#[derive(Debug, Clap)]
pub enum Command {
    #[clap(name = "edit")]
    Edit(EditArgs),
    #[clap(name = "status")]
    Status(StatusArgs),
    #[clap(name = "pull")]
    Pull(PullArgs),
    #[clap(name = "resolve")]
    Resolve(ResolveArgs),
    #[clap(name = "exec")]
    Exec(ExecArgs),
    #[clap(name = "clone")]
    Clone(CloneArgs),
}
