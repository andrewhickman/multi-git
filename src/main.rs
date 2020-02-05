mod cli;
mod config;
mod logger;
mod pull;
mod status;

use failure::{Error, ResultExt};
use termcolor::StandardStream;

fn main() {
    human_panic::setup_panic!();

    let args = cli::parse_args();
    logger::init(&args.logger_options());
    log::trace!("{:?}", args);

    if let Err(err) = run(args) {
        log::error!("{}", fmt_error(&err));
    }
}

fn run(args: cli::Args) -> Result<(), Error> {
    let config = config::parse().context("failed to get config")?;
    log::trace!("{:?}", config);

    let mut stdout = StandardStream::stdout(args.color_choice(atty::Stream::Stdout));
    match args.command {
        cli::Command::Status(status_args) => status::run(&mut stdout, status_args, &config),
        cli::Command::Pull(pull_args) => pull::run(&mut stdout, pull_args, &config),
    }
}

fn fmt_error(err: &Error) -> String {
    let mut pretty = err.to_string();
    for cause in err.iter_causes() {
        pretty.push_str(&format!("\ncaused by: {}", cause));
    }
    pretty
}
