use std::{
    borrow::Cow,
    io::{self, Write as _},
};
use std::{
    ffi::OsString,
    process::{Child, ExitStatus},
    sync::{Arc, Mutex},
};
use std::{path::PathBuf, process::Command};
use std::{process::Stdio, str::FromStr};

use crossterm::{
    style::{Attribute, SetAttribute},
    terminal::{self, Clear, ClearType},
};
use serde::de::IntoDeserializer;
use serde::Deserialize;
use structopt::StructOpt;

use crate::{
    alias, cli,
    config::{Config, Shell},
    output::{self, LineContent, Output},
    walk::{self, walk_with_output},
};

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

    // let mut join_handles = Vec::new();
    walk_with_output(
        args,
        out,
        config,
        root,
        |block, entry| ExecLineContent::build(block, entry, shell, exec_args),
        ExecLineContent::update,
    )
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

    pub fn command(self, args: &[OsString]) -> Command {
        assert!(!args.is_empty());

        match self {
            Shell::None => {
                let mut command = Command::new(&args[0]);
                command.args(&args[1..]);
                command
            }
            Shell::Bash => {
                let mut command = Command::new("/bin/sh");
                command.arg("-c").args(args);
                command
            }
            Shell::Cmd => {
                let mut command = Command::new("cmd");
                command.arg("/S").arg("/C").args(args);
                command
            }
            Shell::Powershell => {
                let mut command = Command::new("powershell");
                command.arg("-Command").args(args);
                command
            }
            Shell::PowershellCore => {
                let mut command = Command::new("pwsh");
                command.arg("-Command").args(args);
                command
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

struct ExecLineContent {
    relative_path: PathBuf,
    state: Arc<Mutex<ExecState>>,
}

enum ExecState {
    NotStarted(Command),
    Running(u32),
    Finished(ExitStatus),
    Error(crate::Error),
}

impl ExecLineContent {
    fn build<'out, 'block>(
        block: &'block output::Block<'out>,
        entry: &walk::Entry,
        shell: Shell,
        exec_args: &ExecArgs,
    ) -> output::Line<'out, 'block, Self> {
        let mut command = shell.command(&exec_args.command);
        command.current_dir(&entry.path);

        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        block.add_line(ExecLineContent {
            relative_path: entry.relative_path.clone(),
            state: Arc::new(Mutex::new(ExecState::NotStarted(command))),
        })
    }

    fn update<'out, 'block>(_: &walk::Entry, line: &output::Line<'out, 'block, Self>) {
        let child = line.content().state.lock().unwrap().spawn();
        if let Some(mut child) = child {
            line.update();
            let wait_result = child.wait();
            line.content().state.lock().unwrap().finish(wait_result);
        }
    }
}

impl ExecState {
    fn spawn(&mut self) -> Option<Child> {
        let command = match self {
            ExecState::NotStarted(command) => command,
            _ => unreachable!(),
        };

        match command.spawn() {
            Ok(child) => {
                *self = ExecState::Running(child.id());
                Some(child)
            }
            Err(err) => {
                let error = crate::Error::with_context(err, "failed to spawn command");
                *self = ExecState::Error(error);
                None
            }
        }
    }

    fn finish(&mut self, status: io::Result<ExitStatus>) {
        match status {
            Ok(status) => {
                *self = ExecState::Finished(status);
            }
            Err(err) => {
                *self = ExecState::Error(crate::Error::with_context(err, "failed to run command"));
            }
        }
    }
}

impl LineContent for ExecLineContent {
    fn write(&self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        crossterm::queue!(stdout, Clear(ClearType::CurrentLine))?;

        let (cols, _) = terminal::size()?;

        write!(
            stdout,
            "{:padding$} ",
            self.relative_path.display(),
            padding = cols as usize / 2
        )?;

        let state = self.state.lock().unwrap();

        match &*state {
            ExecState::NotStarted(_) => (),
            ExecState::Running(id) => {
                write!(stdout, "Running process ")?;
                crossterm::queue!(stdout, SetAttribute(Attribute::Bold))?;
                write!(stdout, "{}", id)?;
                crossterm::queue!(stdout, SetAttribute(Attribute::Reset))?;
            }
            ExecState::Finished(status) => {
                write!(stdout, "{}", status)?;
            }
            ExecState::Error(error) => {
                error.write(stdout)?;
            }
        }

        Ok(())
    }
}
