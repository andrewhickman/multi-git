use git2::Repository;
use walkdir::{DirEntry, WalkDir};

use crate::config::Config;

pub fn walk_repos<F>(config: &Config, mut visit: F)
where
    F: FnMut(&DirEntry, &mut Repository),
{
    let mut iter = WalkDir::new(&config.root).into_iter();
    loop {
        let entry = match iter.next() {
            None => break,
            Some(Err(err)) => {
                log::error!("{}", err);
                continue;
            }
            Some(Ok(entry)) if entry.file_type().is_dir() => entry,
            Some(Ok(entry)) => {
                log::trace!("skipping non-directory `{}`", entry.path().display());
                continue;
            }
        };

        match Repository::open(entry.path()) {
            Ok(mut repo) => {
                visit(&entry, &mut repo);
                iter.skip_current_dir();
            }
            Err(err)
                if err.class() == git2::ErrorClass::Repository
                    && err.code() == git2::ErrorCode::NotFound =>
            {
                log::trace!("skipping non-repo `{}`", entry.path().display());
            }
            Err(err) => log::error!("{}", err.message()),
        }
    }
}
