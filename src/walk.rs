use std::fs;
use std::path::{Path, PathBuf};

use git2::Repository;
use rayon::prelude::*;

pub fn walk_repos<I, D, F>(root: &Path, mut visit_dir: D, visit_repo: F)
where
    I: Sync + Default,
    D: FnMut(&Path, &[(PathBuf, Repository)]) -> I + Sync,
    F: Fn(&Path, &I, &mut Repository) + Sync,
{
    match try_open_repo(root) {
        Ok(Some(mut repo)) => visit_repo(root, &Default::default(), &mut repo),
        Ok(None) => walk_repos_inner(root, root, &mut visit_dir, &visit_repo),
        Err(_) => (),
    }
}

fn walk_repos_inner<I, D, F>(root: &Path, path: &Path, visit_dir: &mut D, visit_repo: &F)
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
                    Ok(file_type) if file_type.is_dir() => match try_open_repo(&sub_path) {
                        Ok(Some(repo)) => {
                            repos.push((relative_path(root, &sub_path).to_owned(), repo));
                        }
                        Ok(None) => {
                            log::trace!("visiting subdirectory at `{}`", sub_path.display());
                            subdirectories.push(sub_path);
                        }
                        Err(_) => (),
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

    let init = visit_dir(relative_path(root, path), &repos);
    repos.into_par_iter().for_each(|(repo_path, mut repo)| {
        visit_repo(&repo_path, &init, &mut repo);
    });

    for subdirectory in subdirectories {
        walk_repos_inner(root, &subdirectory, visit_dir, visit_repo);
    }
}

fn try_open_repo(path: &Path) -> Result<Option<Repository>, git2::Error> {
    match Repository::open(path) {
        Ok(repo) => {
            log::debug!("opened repo at `{}`", path.display());
            Ok(Some(repo))
        }
        Err(err) if err.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(err) => {
            log::error!(
                "failed to open repo at `{}`\ncaused by: {}",
                path.display(),
                err.message()
            );
            Err(err)
        }
    }
}

fn relative_path<'a>(root: &Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}
