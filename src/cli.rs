use argh::FromArgs;

pub fn parse_args() -> Args {
    argh::from_env()
}

#[derive(Debug, FromArgs)]
#[argh(description = "Utility for managing multiple git repos")]
pub struct Args {
    #[argh(subcommand)]
    pub command: Command,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
pub enum Command {
    Status(StatusArgs),
    Pull(PullArgs),
}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "status",
    description = "show the status of your repos"
)]
pub struct StatusArgs {}

#[derive(Debug, FromArgs)]
#[argh(
    subcommand,
    name = "pull",
    description = "pull latest changes in your repos"
)]
pub struct PullArgs {}
