use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::config::{Config, Settings};
use crate::output::{self, Output};
use crate::{git, Error};

pub struct Entry {
    pub relative_path: PathBuf,
    pub repo: git::Repository,
    pub settings: Settings,
}

pub fn walk<F>(out: &Output, config: &Config, path: &Path, visit: F)
where
    F: Fn(output::Line, &Entry) -> Result<(), Error> + Sync,
{
    match git::Repository::try_open(path) {
        Ok(Some(repo)) => visit_repos(
            out,
            path,
            &mut [Entry::from_path(config, path, repo)],
            &visit,
        ),
        Ok(None) => walk_inner(out, config, path, &visit),
        Err(err) => out.write_error(&err.into()),
    }
}

fn walk_inner<F>(out: &Output, config: &Config, path: &Path, visit: &F)
where
    F: Fn(output::Line, &Entry) -> Result<(), Error> + Sync,
{
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            return out.write_error(&Error::with_context(
                err,
                format!("failed to read directory `{}`", path.display()),
            ))
        }
    };

    let mut repos = Vec::new();
    let mut subdirectories = Vec::new();

    for entry in entries {
        match entry {
            Ok(entry) => {
                let sub_path = entry.path();
                let relative_path = config.get_relative_path(&sub_path);
                let settings = config.settings(relative_path);

                if settings.ignore == Some(true) {
                    continue;
                }

                match entry.file_type() {
                    Ok(file_type) if file_type.is_dir() => {
                        match git::Repository::try_open(&sub_path) {
                            Ok(Some(repo)) => {
                                repos.push(Entry::new(relative_path.to_owned(), repo, settings));
                            }
                            Ok(None) => {
                                subdirectories.push(sub_path);
                            }
                            Err(err) => out.write_error(&Error::with_context(
                                err,
                                format!("failed to open repo at `{}`", sub_path.display()),
                            )),
                        }
                    }
                    Err(err) => out.write_error(&Error::with_context(
                        err,
                        format!("failed to get metadata for `{}`", sub_path.display()),
                    )),
                    _ => (),
                }
            }
            Err(err) => out.write_error(&Error::with_context(
                err,
                format!("failed to read entry in `{}`", path.display()),
            )),
        }
    }

    if !repos.is_empty() {
        visit_repos(out, path, &mut repos, visit);
    }

    for subdirectory in subdirectories {
        walk_inner(out, config, &subdirectory, visit);
    }
}

fn visit_repos<F>(out: &Output, path: &Path, entries: &mut [Entry], visit: &F)
where
    F: Fn(output::Line, &Entry) -> Result<(), Error> + Sync,
{
    let block = match out.write_block(
        path.display(),
        entries.iter().map(|entry| entry.relative_path.display()),
    ) {
        Ok(block) => block,
        Err(err) => {
            return out.write_error(&err);
        }
    };

    entries
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, entry)| {
            let line = block.line(index as u16);
            if let Err(err) = visit(line, entry) {
                line.write_error(&err);
            }
        });
}

impl Entry {
    fn new(relative_path: PathBuf, repo: git::Repository, settings: Settings) -> Self {
        Entry {
            relative_path,
            settings,
            repo,
        }
    }

    fn from_path(config: &Config, path: &Path, repo: git::Repository) -> Self {
        let relative_path = config.get_relative_path(path).to_owned();
        let settings = config.settings(&relative_path);
        Entry::new(relative_path, repo, settings)
    }
}
