use std::process::Command;

use failure::{bail, Error, ResultExt};
use termcolor::WriteColor;

use crate::config::Config;
use crate::{alias, cli};

pub fn run(
    _stdout: &mut impl WriteColor,
    args: &cli::Args,
    edit_args: &cli::EditArgs,
    config: &Config,
) -> Result<(), Error> {
    let editor = match (&edit_args.editor, &config.editor) {
        (Some(arg), _) => arg,
        (None, Some(config)) => config,
        (None, None) => {
            bail!("either the `--editor` option or the `editor` config value must be provided")
        }
    };

    let path = alias::resolve(&edit_args.name, args, config)?;

    let mut command = shell();
    command.arg(editor).arg(path);
    log::debug!("spawning command `${:?}`", command);

    let child = command.spawn().context("failed to launch editor")?;
    log::debug!("spawned editor with PID {}", child.id());

    Ok(())
}

#[cfg(windows)]
fn shell() -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C");
    cmd
}

#[cfg(unix)]
fn shell() -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c");
    cmd
}
