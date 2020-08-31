use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::str::FromStr;

use structopt::StructOpt;
use url::Url;

use crate::config::{self, Config};
use crate::output::Output;
use crate::{alias, cli, git};

#[derive(Debug, StructOpt)]
#[structopt(about = "Clone a new repo", no_version)]
#[structopt(setting = structopt::clap::AppSettings::AllowMissingPositional)]
pub struct CloneArgs {
    #[structopt(
        value_name = "TARGET",
        help = "the path or alias of the parent directory to clone into"
    )]
    target: Option<String>,
    #[structopt(
        value_name = "REPOSITORY",
        help = "the repository to clone from",
        parse(from_str)
    )]
    repo: UrlOrPath,
    #[structopt(
        long,
        short,
        value_name = "NAME",
        help = "the name of the directory to create for the new repository",
        parse(from_os_str)
    )]
    name: Option<OsString>,
    #[structopt(
        long,
        short,
        value_name = "ALIAS",
        help = "an alias to create for the new repository"
    )]
    alias: Option<String>,
}

pub fn run(
    out: &Output,
    args: &cli::Args,
    clone_args: &CloneArgs,
    config: &Config,
) -> crate::Result<()> {
    let root = if let Some(name) = &clone_args.target {
        Cow::Owned(alias::resolve(name, args, config)?)
    } else {
        Cow::Borrowed(&*config.root)
    };

    let path = if let Some(name) = &clone_args.name {
        root.join(name)
    } else if let Some(name) = clone_args.repo.dir_name() {
        root.join(name)
    } else {
        return Err(crate::Error::from_message(
            "failed to resolve directory name from url (try passing it with --name)",
        ));
    };

    out.writeln_fmt(format!("cloning into `{}`", path.display()));
    git::Repository::clone(&path, clone_args.repo.as_ref())?;

    if let Some(alias) = &clone_args.alias {
        out.writeln_fmt(format_args!(
            "creating alias `{} = \"{}\"`",
            alias,
            path.display()
        ));
        config::edit(|document| {
            let aliases = document
                .as_table_mut()
                .entry("aliases")
                .as_table_mut()
                .ok_or_else(|| crate::Error::from_message("expected `aliases` to be a table"))?;
            if aliases.contains_key(alias) {
                return Err(crate::Error::from_message(format!(
                    "alias `{}` already exists",
                    alias
                )));
            }

            let relative_path = path
                .strip_prefix(&config.root)
                .unwrap_or(&path)
                .to_str()
                .ok_or_else(|| crate::Error::from_message(format!("path is invalid UTF-16")))?;

            aliases[alias] = toml_edit::value(relative_path);

            Ok(())
        })?;
    }
    Ok(())
}

#[derive(Debug)]
enum UrlOrPath {
    Url(Url),
    Path(String),
}

impl UrlOrPath {
    fn dir_name(&self) -> Option<&OsStr> {
        match self {
            UrlOrPath::Url(url) => url.path_segments()?.rev().find_map(|segment| {
                let name = segment.strip_suffix(".git")?;
                if name.is_empty() {
                    None
                } else {
                    Some(name.as_ref())
                }
            }),
            UrlOrPath::Path(path) => Path::new(path).file_stem(),
        }
    }
}

impl AsRef<str> for UrlOrPath {
    fn as_ref(&self) -> &str {
        match self {
            UrlOrPath::Url(url) => url.as_ref(),
            UrlOrPath::Path(path) => path.as_ref(),
        }
    }
}

impl<'a> From<&'a str> for UrlOrPath {
    fn from(s: &'a str) -> Self {
        match Url::from_str(s) {
            Ok(url) => UrlOrPath::Url(url),
            Err(_) => UrlOrPath::Path(s.to_owned()),
        }
    }
}

#[test]
fn test_dir_name() {
    let cases = vec![
        "ssh://user@host.xz:45435/path/to/repo.git/",
        "ssh://host.xz/path/to/repo.git",
        "https://host.xz:3545/path/to/repo.git/",
        "http://host.xz/path/to/repo.git/",
        "git://host.xz:5435/path/to/repo.git/",
        "git://host.xz/path/to/repo.git/",
        "ftp://host.xz:4354/path/to/repo.git/",
        "ftps://host.xz:4354/path/to/repo.git/",
        "ftps://host.xz:4354/path/to/repo.git/",
        "user@host.xz:path/to/repo.git/",
        "/path/to/repo.git/",
        "/path/to/repo.git",
        "/path/to/repo.git",
        "file:///path/to/repo.git/",
        "file:///path/to/repo.git",
    ];

    for case in cases {
        assert_eq!(UrlOrPath::from(case).dir_name(), Some("repo".as_ref()));
    }
}
