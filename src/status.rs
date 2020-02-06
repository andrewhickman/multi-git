use failure::Error;
use termcolor::WriteColor;

use crate::cli;
use crate::config::Config;
use crate::walk::walk_repos;

pub fn run(
    stdout: &mut impl WriteColor,
    _args: cli::StatusArgs,
    config: &Config,
) -> Result<(), Error> {
    walk_repos(config, |entry, _| {
        writeln!(stdout, "{} is a repo!", entry.path().display()).unwrap();
    });
    Ok(())
}
