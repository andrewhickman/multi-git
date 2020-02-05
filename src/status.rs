use failure::Error;
use termcolor::WriteColor;

use crate::cli;
use crate::config::Config;

pub fn run(
    stdout: &mut impl WriteColor,
    _args: cli::StatusArgs,
    _config: &Config,
) -> Result<(), Error> {
    write!(stdout, "status!")?;
    Ok(())
}
