mod alias;
mod cli;
mod config;
mod edit;
mod logger;
mod pull;
mod status;
mod walk;

use std::process;

use failure::{Error, ResultExt};
use termcolor::StandardStream;

fn main() {
    human_panic::setup_panic!();

    let args = cli::parse_args();
    logger::init(&args.logger_options());
    log::trace!("{:?}", args);

    if let Err(err) = run(args) {
        log::error!("{}", fmt_error(&err));
        process::exit(1);
    }
}

fn run(args: cli::Args) -> Result<(), Error> {
    let config = config::parse().context("failed to get config")?;
    log::trace!("{:?}", config);

    let stdout = StandardStream::stdout(args.color_choice(atty::Stream::Stdout));
    let mut stdout = stdout.lock();

    match args.command {
        cli::Command::Edit(edit_args) => edit::run(&mut stdout, edit_args, &config),
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
