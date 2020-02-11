use std::fs;
use std::path::{Path, PathBuf};

use git2::Repository;
use rayon::prelude::*;

use crate::config::{Config, Settings};
use crate::git_utils;

pub fn walk_repos<I, D, F>(config: &Config, root: &Path, mut visit_dir: D, visit_repo: F)
where
    I: Sync + Default,
    D: FnMut(&Path, &[(PathBuf, Settings, Repository)]) -> I + Sync,
    F: Fn(&Path, &I, &Settings, &mut Repository) + Sync,
{
    let relative_path = config.get_relative_path(root);
    match git_utils::try_open_repo(root) {
        Ok(Some(mut repo)) => visit_repo(
            relative_path,
            &Default::default(),
            &config.settings(relative_path),
            &mut repo,
        ),
        Ok(None) => walk_repos_inner(config, root, root, &mut visit_dir, &visit_repo),
        Err(_) => (),
    }
}

fn walk_repos_inner<I, D, F>(
    config: &Config,
    root: &Path,
    path: &Path,
    visit_dir: &mut D,
    visit_repo: &F,
) where
    I: Sync,
    D: FnMut(&Path, &[(PathBuf, Settings, Repository)]) -> I + Sync,
    F: Fn(&Path, &I, &Settings, &mut Repository) + Sync,
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
                let relative_path = config.get_relative_path(&sub_path);
                let settings = config.settings(relative_path);

                if settings.ignore.unwrap_or(false) {
                    log::debug!("ignoring `{}` due to config setting", sub_path.display());
                    continue;
                }

                match entry.file_type() {
                    Ok(file_type) if file_type.is_dir() => {
                        match git_utils::try_open_repo(&sub_path) {
                            Ok(Some(repo)) => {
                                repos.push((relative_path.to_owned(), settings, repo));
                            }
                            Ok(None) => {
                                log::trace!("visiting subdirectory at `{}`", sub_path.display());
                                subdirectories.push(sub_path);
                            }
                            Err(_) => (),
                        }
                    }
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

    let init = visit_dir(config.get_relative_path(path), &repos);
    repos
        .into_par_iter()
        .for_each(|(repo_path, settings, mut repo)| {
            visit_repo(&repo_path, &init, &settings, &mut repo);
        });

    for subdirectory in subdirectories {
        walk_repos_inner(config, root, &subdirectory, visit_dir, visit_repo);
    }
}
