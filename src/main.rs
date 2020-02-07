mod alias;
mod cli;
mod config;
mod edit;
mod logger;
mod pull;
mod status;
mod walk;

use std::error::Error;
use std::process;

use failure::ResultExt;
use termcolor::StandardStream;

fn main() {
    human_panic::setup_panic!();

    let args = cli::parse_args();
    logger::init(&args.logger_options());
    log::trace!("{:#?}", args);

    if let Err(err) = run(args) {
        log::error!("{}", fmt_error(&err.compat()));
        process::exit(1);
    }
}

fn run(args: cli::Args) -> Result<(), failure::Error> {
    let config = config::parse().context("failed to get config")?;
    log::trace!("{:#?}", config);

    let stdout = StandardStream::stdout(args.color_choice(atty::Stream::Stdout));

    match &args.command {
        cli::Command::Edit(edit_args) => edit::run(&stdout, &args, edit_args, &config),
        cli::Command::Status(status_args) => status::run(&stdout, &args, status_args, &config),
        cli::Command::Pull(pull_args) => pull::run(&stdout, &args, pull_args, &config),
    }
}

fn fmt_error(err: &impl Error) -> String {
    let mut pretty = err.to_string();
    let mut source = err.source();
    while let Some(err) = source {
        pretty.push_str(&format!("\ncaused by: {}", err));
        source = err.source();
    }
    pretty
}
