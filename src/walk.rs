use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crossterm::style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor};
use serde::Serialize;

use crate::config::{Config, Settings};
use crate::output::{Block, Line, LineContent, Output};
use crate::{cli, git};

pub struct Entry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub repo: git::Repository,
    pub settings: Settings,
}

pub fn walk_with_output<'out, C, B, U>(
    args: &cli::Args,
    output: &'out Output,
    config: &Config,
    path: impl Into<PathBuf> + AsRef<Path>,
    build: B,
    update: U,
) -> crate::Result<()>
where
    C: LineContent + 'out,
    B: for<'block> FnMut(&'block Block<'out>, &Entry) -> Line<'out, 'block, C>,
    U: for<'block> Fn(&Entry, &Line<'out, 'block, C>) + Sync,
{
    let block = output.block()?;
    let mut lines = walk_build(&block, config, path, build);
    walk_update(args, &block, &mut lines, update);
    Ok(())
}

pub fn walk<F, G, H>(
    config: &Config,
    path: impl Into<PathBuf> + AsRef<Path>,
    mut visit_repo: F,
    mut visit_dir: G,
    mut visit_err: H,
) where
    F: FnMut(Entry),
    G: FnMut(&Path),
    H: FnMut(crate::Error),
{
    match git::Repository::try_open(path.as_ref()) {
        Ok(Some(repo)) => {
            visit_repo(Entry::from_path(config, path.into(), repo));
        }
        Ok(None) => {
            walk_inner(
                config,
                path.as_ref(),
                &mut visit_repo,
                &mut visit_dir,
                &mut visit_err,
            );
        }
        Err(err) => {
            visit_err(err);
        }
    }
}

fn walk_inner<F, G, H>(
    config: &Config,
    path: &Path,
    visit_repo: &mut F,
    visit_dir: &mut G,
    visit_err: &mut H,
) where
    F: FnMut(Entry),
    G: FnMut(&Path),
    H: FnMut(crate::Error),
{
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            return visit_err(crate::Error::with_context(
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
                                let relative_path = relative_path.to_owned();
                                repos.push(Entry::new(sub_path, relative_path, repo, settings));
                            }
                            Ok(None) => {
                                subdirectories.push(sub_path);
                            }
                            Err(err) => visit_err(crate::Error::with_context(
                                err,
                                format!("failed to open repo at `{}`", sub_path.display()),
                            )),
                        }
                    }
                    Err(err) => visit_err(crate::Error::with_context(
                        err,
                        format!("failed to get metadata for `{}`", sub_path.display()),
                    )),
                    _ => (),
                }
            }
            Err(err) => visit_err(crate::Error::with_context(
                err,
                format!("failed to read entry in `{}`", path.display()),
            )),
        }
    }

    if !repos.is_empty() {
        visit_dir(path);
        for repo in repos {
            visit_repo(repo);
        }
    }

    for subdirectory in subdirectories {
        walk_inner(config, &subdirectory, visit_repo, visit_dir, visit_err);
    }
}

fn walk_build<'out, 'block, C, B>(
    block: &'block Block<'out>,
    config: &Config,
    path: impl Into<PathBuf> + AsRef<Path>,
    mut build: B,
) -> Vec<(Entry, Line<'out, 'block, C>)>
where
    C: LineContent + 'out,
    B: FnMut(&'block Block<'out>, &Entry) -> Line<'out, 'block, C>,
{
    let mut result = Vec::new();

    walk(
        config,
        path,
        |repo| {
            let line = build(block, &repo);
            result.push((repo, line));
        },
        |path| {
            block.add_finished_line(DirectoryLineContent::new(path));
        },
        |err| {
            block.add_error_line(err);
        },
    );

    result
}

fn walk_update<'out, 'block, C, U>(
    args: &cli::Args,
    block: &'block Block<'out>,
    lines: &mut [(Entry, Line<'out, 'block, C>)],
    update: U,
) where
    C: LineContent,
    U: Fn(&Entry, &Line<'out, 'block, C>) + Sync,
{
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.jobs)
        .thread_name(|index| format!("rayon-work-thread-{}", index))
        .build()
        .unwrap();

    let update = &update;
    thread_pool.in_place_scope_fifo(move |scope| {
        block.update_all().ok();
        for (entry, line) in lines {
            scope.spawn_fifo(move |_| {
                update(&*entry, line);
                line.finish();
            });
        }
    });
}

impl Entry {
    fn new(
        path: PathBuf,
        relative_path: PathBuf,
        repo: git::Repository,
        settings: Settings,
    ) -> Self {
        Entry {
            path,
            relative_path,
            settings,
            repo,
        }
    }

    fn from_path(config: &Config, path: PathBuf, repo: git::Repository) -> Self {
        let relative_path = config.get_relative_path(&path).to_owned();
        let settings = config.settings(&relative_path);
        Entry::new(path, relative_path, repo, settings)
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

    fn write_json(&self, stdout: &mut io::StdoutLock) -> serde_json::Result<()> {
        #[derive(Serialize)]
        #[serde(tag = "kind", rename_all = "snake_case")]
        enum JsonDirectory {
            Directory { path: String },
        }

        serde_json::to_writer(
            stdout,
            &JsonDirectory::Directory {
                path: self.path.display().to_string(),
            },
        )
    }
}
