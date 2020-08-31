use std::borrow::Cow;
use std::ffi::OsString;
use std::process::Command;
use std::str::FromStr;

use serde::de::IntoDeserializer;
use serde::Deserialize;
use structopt::StructOpt;

use crate::config::{Config, Shell};
use crate::output::Output;
use crate::walk::walk;
use crate::{alias, cli};

#[derive(Debug, StructOpt)]
#[structopt(about = "Run a command in one or more repos", no_version)]
#[structopt(setting = structopt::clap::AppSettings::TrailingVarArg)]
#[structopt(setting = structopt::clap::AppSettings::AllowMissingPositional)]
pub struct ExecArgs {
    #[structopt(
        value_name = "TARGET",
        help = "the path or alias of the repo(s) to execute the command in"
    )]
    target: Option<String>,
    #[structopt(
        value_name = "COMMAND",
        help = "the command to execute",
        required = true,
        parse(from_os_str)
    )]
    command: Vec<OsString>,
    #[structopt(
        long,
        short,
        value_name = "SHELL",
        help = "the shell to execute the command in",
        possible_values = Shell::POSSIBLE_VALUES,
        parse(try_from_str)
    )]
    shell: Option<Shell>,
}

pub fn run(
    out: &Output,
    args: &cli::Args,
    exec_args: &ExecArgs,
    config: &Config,
) -> crate::Result<()> {
    let shell = exec_args.shell.unwrap_or(config.default_shell);

    let root = if let Some(name) = &exec_args.target {
        Cow::Owned(alias::resolve(name, args, config)?)
    } else {
        Cow::Borrowed(&*config.root)
    };

    let mut command = match shell.command() {
        Some(mut command) => {
            command.args(&exec_args.command);
            command
        }
        None => {
            let mut command = Command::new(&exec_args.command[0]);
            command.args(&exec_args.command[1..]);
            command
        }
    };

    let mut join_handles = Vec::new();
    walk(
        config,
        root,
        |entry| {
            command.current_dir(&entry.path);
            join_handles.push((entry, command.spawn()));
        },
        |_| {},
        |err| out.writeln_error(&err),
    );

    for (entry, handle) in join_handles {
        let status = handle
            .map_err(|err| crate::Error::with_context(err, "failed to spawn command"))?
            .wait()
            .map_err(|err| crate::Error::with_context(err, "failed to run command"))?;
        if !status.success() {
            out.writeln_error(&crate::Error::from_message(format!(
                "{}: process exited with {}",
                entry.relative_path.display(),
                status
            )));
        }
    }

    Ok(())
}

impl Shell {
    const POSSIBLE_VALUES: &'static [&'static str] = &[
        "none",
        "bash",
        "sh",
        "cmd",
        "powershell",
        "pwsh",
        "powershell-core",
    ];

    pub fn command(self) -> Option<Command> {
        match self {
            Shell::None => None,
            Shell::Bash => {
                let mut command = Command::new("/bin/sh");
                command.arg("-c");
                Some(command)
            }
            Shell::Cmd => {
                let mut command = Command::new("cmd");
                command.arg("/S").arg("/C");
                Some(command)
            }
            Shell::Powershell => {
                let mut command = Command::new("powershell");
                command.arg("-Command");
                Some(command)
            }
            Shell::PowershellCore => {
                let mut command = Command::new("pwsh");
                command.arg("-Command");
                Some(command)
            }
        }
    }
}

impl Default for Shell {
    fn default() -> Shell {
        if cfg!(unix) {
            Shell::Bash
        } else if cfg!(windows) {
            Shell::Cmd
        } else {
            Shell::None
        }
    }
}

impl FromStr for Shell {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Shell::deserialize(s.into_deserializer())
    }
}
