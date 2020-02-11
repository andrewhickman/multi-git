use structopt::clap::AppSettings;
use structopt::StructOpt;
use termcolor::ColorChoice;

use crate::{edit, logger, pull, status};

pub fn parse_args() -> Args {
    Args::from_args()
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Utility for managing multiple git repos")]
#[structopt(global_setting = AppSettings::DisableVersion)]
#[structopt(global_setting = AppSettings::UnifiedHelpMessage)]
#[structopt(global_setting = AppSettings::VersionlessSubcommands)]
pub struct Args {
    #[structopt(subcommand)]
    pub command: Command,
    #[structopt(flatten)]
    pub logger: logger::Opts,
    #[structopt(long, short = "A", help = "Disable aliases")]
    pub no_alias: bool,
    #[structopt(
        long,
        help = "Control when to use colored output",
        parse(try_from_str = parse_color_choice),
        possible_values = COLOR_CHOICE_VALUES,
        global = true
    )]
    pub color: Option<ColorChoice>,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "edit")]
    Edit(edit::EditArgs),
    #[structopt(name = "status")]
    Status(status::StatusArgs),
    #[structopt(name = "pull")]
    Pull(pull::PullArgs),
}

impl Args {
    pub fn color_choice(&self, stream: atty::Stream) -> ColorChoice {
        match self.color {
            Some(ColorChoice::Auto) | None => {
                if atty::is(stream) {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
            Some(color_choice) => color_choice,
        }
    }
}

const COLOR_CHOICE_VALUES: &[&str] = &["always", "ansi", "auto", "never"];

fn parse_color_choice(input: &str) -> Result<ColorChoice, String> {
    match input {
        "always" => Ok(ColorChoice::Always),
        "ansi" => Ok(ColorChoice::AlwaysAnsi),
        "auto" => Ok(ColorChoice::Auto),
        "never" => Ok(ColorChoice::Never),
        _ => unreachable!(),
    }
}
