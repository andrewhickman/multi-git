mod alias;
mod cli;
mod config;
mod error;
mod git;
mod output;
mod progress;
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
    let config = config::parse(|ignored_path| {
        out.writeln_warning(format_args!("unused configuration key: {}", ignored_path))
    })
    .map_err(|err| Error::with_context(err, "failed to get config"))?;
    log::trace!("{:#?}", config);

    match &args.command {
        cli::Command::Edit(edit_args) => cli::edit(args, edit_args, &config),
        cli::Command::Status(status_args) => cli::status(out, &args, status_args, &config),
        cli::Command::Pull(pull_args) => cli::pull(out, &args, pull_args, &config),
        cli::Command::Resolve(resolve_args) => cli::resolve(out, &args, resolve_args, &config),
        cli::Command::Exec(exec_args) => cli::exec(out, &args, exec_args, &config),
        cli::Command::Clone(clone_args) => cli::clone(out, &args, clone_args, &config),
    }
}
