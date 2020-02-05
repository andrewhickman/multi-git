use argh::FromArgs;
use termcolor::ColorChoice;

use crate::logger;

pub fn parse_args() -> Args {
    argh::from_env()
}

#[derive(Debug, FromArgs)]
#[argh(description = "Utility for managing multiple git repos")]
pub struct Args {
    #[argh(subcommand)]
    pub command: Command,
    #[argh(switch, description = "don't print anything to stderr")]
    pub quiet: bool,
    #[argh(switch, description = "enable debug logging")]
    pub debug: bool,
    #[argh(switch, description = "enable trace logging")]
    pub trace: bool,
    #[argh(
        option,
        description = "controls when to use colored output (options: always, ansi, auto, never)",
        from_str_fn(parse_color_choice)
    )]
    pub color: Option<ColorChoice>,
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

impl Args {
    pub fn logger_options(&self) -> logger::Options {
        logger::Options {
            quiet: self.quiet,
            debug: self.debug,
            trace: self.trace,
            color_choice: self.color,
        }
    }
}

fn parse_color_choice(input: &str) -> Result<ColorChoice, String> {
    match input {
        "always" => Ok(ColorChoice::Always),
        "ansi" => Ok(ColorChoice::AlwaysAnsi),
        "auto" => Ok(ColorChoice::Auto),
        "never" => Ok(ColorChoice::Never),
        _ => Err("must be one of 'always', 'ansi', 'auto' or 'never'".to_owned()),
    }
}
