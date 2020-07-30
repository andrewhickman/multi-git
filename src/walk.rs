mod skip_range;

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};
use rayon::prelude::*;

use self::skip_range::SkipRange;
use crate::config::{Config, Settings};
use crate::git;
use crate::output::{Block, Line, LineContent, Output};

pub struct Entry {
    pub relative_path: PathBuf,
    pub repo: git::Repository,
    pub settings: Settings,
}

pub fn init_thread_pool() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get() * 2)
        .build_global()
        .unwrap()
}

pub fn walk<'out, C, B, U>(
    output: &'out Output,
    config: &Config,
    path: &Path,
    build: B,
    update: U,
) -> crate::Result<()>
where
    C: LineContent + 'out,
    B: for<'block> FnMut(&'block Block<'out>, &Entry) -> Line<'out, 'block, C>,
    U: for<'block> Fn(&Entry, Line<'out, 'block, C>) + Sync,
{
    let block = output.block()?;
    let mut lines = walk_build(&block, config, path, build);
    walk_update(&mut lines, update);
    Ok(())
}

fn walk_build<'out, 'block, C, B>(
    block: &'block Block<'out>,
    config: &Config,
    path: &Path,
    mut build: B,
) -> Vec<(Entry, Line<'out, 'block, C>)>
where
    C: LineContent + 'out,
    B: FnMut(&'block Block<'out>, &Entry) -> Line<'out, 'block, C>,
{
    match git::Repository::try_open(path) {
        Ok(Some(repo)) => {
            block.add_finished_line(DirectoryLineContent::new(path));
            let entry = Entry::from_path(config, path, repo);
            let line = build(block, &entry);
            vec![(entry, line)]
        }
        Ok(None) => {
            let mut result = vec![];
            walk_build_inner(block, config, path, &mut result, &mut build);
            result
        }
        Err(err) => {
            block.add_error_line(err);
            vec![]
        }
    }
}

fn walk_build_inner<'out, 'block, C, B>(
    block: &'block Block<'out>,
    config: &Config,
    path: &Path,
    result: &mut Vec<(Entry, Line<'out, 'block, C>)>,
    build: &mut B,
) where
    C: LineContent,
    B: FnMut(&'block Block<'out>, &Entry) -> Line<'out, 'block, C>,
{
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            return block.add_error_line(crate::Error::with_context(
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
                            Err(err) => block.add_error_line(crate::Error::with_context(
                                err,
                                format!("failed to open repo at `{}`", sub_path.display()),
                            )),
                        }
                    }
                    Err(err) => block.add_error_line(crate::Error::with_context(
                        err,
                        format!("failed to get metadata for `{}`", sub_path.display()),
                    )),
                    _ => (),
                }
            }
            Err(err) => block.add_error_line(crate::Error::with_context(
                err,
                format!("failed to read entry in `{}`", path.display()),
            )),
        }
    }

    if !repos.is_empty() {
        block.add_finished_line(DirectoryLineContent::new(path));
        result.extend(repos.into_iter().map(|repo| {
            let line = build(block, &repo);
            (repo, line)
        }));
    }

    for subdirectory in subdirectories {
        walk_build_inner(block, config, &subdirectory, result, build);
    }
}

fn walk_update<'out, 'block, C, U>(lines: &mut [(Entry, Line<'out, 'block, C>)], update: U)
where
    C: LineContent,
    U: Fn(&Entry, Line<'out, 'block, C>) + Sync,
{
    rayon::iter::split(SkipRange::new(lines), SkipRange::split).for_each(|line_range| {
        for (entry, line) in line_range {
            update(entry, line.clone());
        }
    })
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

struct DirectoryLineContent {
    path: PathBuf,
}

impl DirectoryLineContent {
    fn new(path: impl Into<PathBuf>) -> Self {
        DirectoryLineContent { path: path.into() }
    }
}

impl LineContent for DirectoryLineContent {
    fn write(&self, stdout: &mut io::StdoutLock) -> crossterm::Result<()> {
        crossterm::queue!(
            stdout,
            SetForegroundColor(Color::Yellow),
            SetAttribute(Attribute::Underlined)
        )?;
        write!(stdout, "{}", self.path.display())?;
        stdout.flush()?;
        crossterm::queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
        Ok(())
    }
}
