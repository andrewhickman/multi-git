use std::collections::BTreeMap;
use std::ops::Bound;
use std::path::{Path, PathBuf};

use crate::cli;
use crate::config::Config;

pub fn resolve(name: &str, args: &cli::Args, config: &Config) -> crate::Result<PathBuf> {
    let full_path = if let Some(path) = resolve_prefix(&config.aliases, name, args)? {
        let full_path = config.root.join(path);
        log::trace!("resolved alias `{}` to `{}`", name, full_path.display());
        full_path
    } else {
        let full_path = config.root.join(name);
        log::trace!("resolved path `{}` to `{}`", name, full_path.display());
        full_path
    };

    if !full_path.exists() {
        Err(crate::Error::from_message(format!(
            "failed to resolve path or alias `{}` (path `{}` does not exist)",
            name,
            full_path.display()
        )))
    } else {
        Ok(full_path)
    }
}

fn resolve_prefix<'a>(
    map: &'a BTreeMap<String, PathBuf>,
    prefix: &str,
    args: &cli::Args,
) -> crate::Result<Option<&'a Path>> {
    if args.no_alias {
        return Ok(None);
    }

    let mut iter = map
        .range::<str, _>((Bound::Included(prefix), Bound::Unbounded))
        .take_while(move |(key, _)| key.starts_with(prefix));

    match iter.next() {
        None => Ok(None),
        Some((key1, path)) => match iter.next() {
            None => Ok(Some(path.as_ref())),
            Some((key2, _)) => {
                if key1 == prefix {
                    log::warn!("alias `{}` is a prefix of alias `{}`", key1, key2);
                    Ok(Some(path.as_ref()))
                } else {
                    Err(crate::Error::from_message(format!(
                        "ambiguous alias `{}` (could match either `{}` or `{}`)",
                        prefix, key1, key2
                    )))
                }
            }
        },
    }
}
