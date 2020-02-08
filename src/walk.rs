use std::fs;
use std::path::{Path, PathBuf};

use git2::Repository;
use rayon::prelude::*;

use crate::config::Config;

pub fn walk_repos<I, D, F>(config: &Config, mut visit_dir: D, visit_repo: F)
where
    I: Sync,
    D: FnMut(&Path, &[(PathBuf, Repository)]) -> I + Sync,
    F: Fn(&Path, &I, &mut Repository) + Sync,
{
    walk_repos_inner(config, &config.root, &mut visit_dir, &visit_repo)
}

fn walk_repos_inner<I, D, F>(config: &Config, path: &Path, visit_dir: &mut D, visit_repo: &F)
where
    I: Sync,
    D: FnMut(&Path, &[(PathBuf, Repository)]) -> I + Sync,
    F: Fn(&Path, &I, &mut Repository) + Sync,
{
    log::trace!("visiting entries in `{}`", path.display());
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            log::error!(
                "failed to read entries from `{}`\ncaused by: {}",
                path.display(),
                err
            );
            return;
        }
    };

    let mut repos = Vec::new();
    let mut subdirectories = Vec::new();
    for entry in entries {
        match entry {
            Err(err) => log::error!(
                "failed to read entry in `{}`\ncaused by: {}",
                path.display(),
                err
            ),
            Ok(entry) => {
                let sub_path = entry.path();
                match entry.file_type() {
                    Ok(file_type) if file_type.is_dir() => match Repository::open(&sub_path) {
                        Ok(repo) => {
                            log::trace!("visiting repo at `{}`", sub_path.display());
                            repos.push((relative_path(config, &sub_path).to_owned(), repo));
                        }
                        Err(err)
                            if err.class() == git2::ErrorClass::Repository
                                && err.code() == git2::ErrorCode::NotFound =>
                        {
                            log::trace!("visiting subdirectory at `{}`", sub_path.display());
                            subdirectories.push(sub_path);
                        }
                        Err(err) => log::error!(
                            "failed to open repo at `{}`\ncaused by: {}",
                            sub_path.display(),
                            err.message()
                        ),
                    },
                    Err(err) => log::error!(
                        "failed to get metadata for `{}`\ncaused by: {}",
                        sub_path.display(),
                        err
                    ),
                    _ => log::trace!("skipping non-directory `{}`", sub_path.display()),
                }
            }
        }
    }

    let init = visit_dir(relative_path(config, path), &repos);
    repos.into_par_iter().for_each(|(repo_path, mut repo)| {
        visit_repo(&repo_path, &init, &mut repo);
    });

    for subdirectory in subdirectories {
        walk_repos_inner(config, &subdirectory, visit_dir, visit_repo);
    }
}

fn relative_path<'a>(config: &Config, path: &'a Path) -> &'a Path {
    path.strip_prefix(&config.root).unwrap_or(path)
}
