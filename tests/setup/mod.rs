use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use assert_fs::fixture::TempDir;
use which::which;

pub struct Context {
    temp_dir: TempDir,
    working_dir: PathBuf,
    git_exe: PathBuf,
}

pub fn run(data: &str) -> Context {
    let mut context = Context::new();

    for line in data.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (cmd, rem) = line.split_once(" ").expect("invalid syntax");

        match cmd {
            "CD" => context.run_cd(rem),
            "GIT" => context.run_git(rem),
            "WRITE" => context.run_write(rem),
            _ => panic!("Invalid command {}", cmd),
        }
    }

    context
}

impl Context {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let git_exe = which("git").unwrap();

        Context {
            working_dir: temp_dir.path().to_owned(),
            temp_dir,
            git_exe,
        }
    }

    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    fn run_cd(&mut self, name: &str) {
        let path = Path::new(name);
        let working_dir = if path.has_root() {
            let mut working_dir = self.temp_dir.path().to_owned();
            working_dir.extend(path.components().skip_while(|&c| c == Component::RootDir));
            working_dir
        } else {
            self.working_dir.join(path)
        };

        match fs_err::create_dir(&working_dir) {
            Ok(_) => (),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
            Err(err) => panic!("error creating directory {}", err),
        };

        self.working_dir = working_dir;
    }

    fn run_git(&mut self, cmd: &str) {
        let status = Command::new(&self.git_exe)
            .args(shell_words::split(cmd).unwrap())
            .current_dir(&self.working_dir)
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        if !status.success() {
            panic!("git exited unsuccessfully: {}", status);
        }
    }

    fn run_write(&mut self, cmd: &str) {
        let (filename, text) = match cmd.split_once(' ') {
            Some((filename, text)) => (filename, text),
            None => (cmd, ""),
        };
        fs_err::write(self.working_dir.join(filename), text).unwrap();
    }
}
