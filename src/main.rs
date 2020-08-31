mod alias;
mod cli;
mod clone;
mod config;
mod edit;
mod error;
mod exec;
mod git;
mod output;
mod progress;
mod pull;
mod resolve;
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
        out.writeln_error(&err);
        process::exit(1);
    }
}

fn run(out: &Output, args: &cli::Args) -> Result<()> {
    let config = config::parse().map_err(|err| Error::with_context(err, "failed to get config"))?;
    log::trace!("{:#?}", config);

    match &args.command {
        cli::Command::Edit(edit_args) => edit::run(args, edit_args, &config),
        cli::Command::Status(status_args) => status::run(out, &args, status_args, &config),
        cli::Command::Pull(pull_args) => pull::run(out, &args, pull_args, &config),
        cli::Command::Resolve(resolve_args) => resolve::run(out, &args, resolve_args, &config),
        cli::Command::Exec(exec_args) => exec::run(out, &args, exec_args, &config),
        cli::Command::Clone(clone_args) => clone::run(out, &args, clone_args, &config),
    }
}
