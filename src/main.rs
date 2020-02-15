mod alias;
mod cli;
mod config;
mod edit;
mod error;
mod git;
mod output;
mod progress;
mod pull;
mod status;
mod walk;

pub use crate::error::{Error, Result};

use std::process;

use crate::output::Output;

fn main() {
    human_panic::setup_panic!();

    let args = cli::parse_args();
    log::trace!("{:#?}", args);

    let out = Output::new();

    if let Err(err) = run(&out, &args) {
        out.write_error(&err);
        process::exit(1);
    }
}

fn run(out: &Output, args: &cli::Args) -> Result<()> {
    let config = config::parse().map_err(|err| Error::with_context(err, "failed to get config"))?;
    log::trace!("{:#?}", config);

    match &args.command {
        cli::Command::Edit(edit_args) => edit::run(out, args, edit_args, &config),
        cli::Command::Status(status_args) => status::run(out, &args, status_args, &config),
        cli::Command::Pull(pull_args) => pull::run(out, &args, pull_args, &config),
    }
}
