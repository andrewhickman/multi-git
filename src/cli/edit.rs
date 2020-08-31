use std::process::Command;

use structopt::StructOpt;

use crate::config::Config;
use crate::{alias, cli, config, git};

#[derive(Debug, StructOpt)]
#[structopt(about = "Open a repo in an editor", no_version)]
pub struct EditArgs {
    #[structopt(
        value_name = "TARGET",
        help = "the path or alias of the repo to edit",
        required_unless = "config"
    )]
    target: Option<String>,
    #[structopt(long, short, help = "override the editor program")]
    editor: Option<String>,
    #[structopt(long, short, help = "create a new branch")]
    branch: Option<String>,
    #[structopt(
        long,
        short,
        help = "Edit the config file",
        conflicts_with = "target",
        conflicts_with = "branch"
    )]
    config: bool,
}

pub fn run(args: &cli::Args, edit_args: &EditArgs, config: &Config) -> crate::Result<()> {
    let path = if let Some(name) = &edit_args.target {
        alias::resolve(name, args, config)?
    } else if edit_args.config {
        config::expect_file_path()?
    } else {
        unreachable!()
    };

    let settings = config.settings(config.get_relative_path(&path));

    let editor = match (&edit_args.editor, &settings.editor) {
        (Some(arg), _) => arg,
        (None, Some(config)) => config,
        (None, None) => {
            return Err(crate::Error::from_message(
                "either the `--editor` option or the `editor` config value must be provided",
            ))
        }
    };

    if let Some(branch_name) = &edit_args.branch {
        let repo = git::Repository::open(&path)?;
        repo.create_branch(&settings, branch_name)?;
    }

    let mut command = shell();
    command.arg(editor).arg(&path);
    if path.is_dir() {
        command.current_dir(&path);
    }
    log::debug!("spawning command `${:?}`", command);

    let child = command
        .spawn()
        .map_err(|err| crate::Error::with_context(err, "failed to launch editor"))?;
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
