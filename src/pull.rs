use failure::Error;
use std::io::Write;
use termcolor::StandardStream;

use crate::cli;
use crate::config::Config;

pub fn run(
    stdout: &StandardStream,
    _args: &cli::Args,
    _pull_args: &cli::PullArgs,
    _config: &Config,
) -> Result<(), Error> {
    write!(stdout.lock(), "pull!")?;
    Ok(())
}
