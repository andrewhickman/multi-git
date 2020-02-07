use std::fs;
use std::path::Path;

use git2::Repository;
use rayon::prelude::*;

use crate::config::Config;

pub fn walk_repos<D, F>(config: &Config, visit_dir: D, visit_repo: F)
where
    D: Fn(&Path) + Sync,
    F: Fn(&Path, &mut Repository) + Sync,
{
    walk_repos_inner(&config.root, &visit_dir, &visit_repo)
}

fn walk_repos_inner<D, F>(path: &Path, visit_dir: &D, visit_repo: &F)
where
    D: Fn(&Path) + Sync,
    F: Fn(&Path, &mut Repository) + Sync,
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
                            repos.push((sub_path, repo));
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

    if !repos.is_empty() {
        visit_dir(path);
    }

    repos.into_par_iter().for_each(|(repo_path, mut repo)| {
        visit_repo(&repo_path, &mut repo);
    });

    for subdirectory in subdirectories {
        walk_repos_inner(&subdirectory, visit_dir, visit_repo);
    }
}
