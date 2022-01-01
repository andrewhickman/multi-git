use std::cmp;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::ops::Bound;
use std::path::{Path, PathBuf};

use crate::cli;
use crate::config::Config;

pub fn resolve(name: &str, args: &cli::Args, config: &Config) -> crate::Result<PathBuf> {
    if let Some(path) = resolve_prefix(&config.aliases, name, args)? {
        let full_path = config.root.join(path);
        log::trace!("resolved alias `{}` to `{}`", name, full_path.display());

        if !full_path.exists() {
            Err(crate::Error::from_message(format!(
                "alias `{}` resolved to invalid path `{}`",
                name,
                full_path.display()
            )))
        } else {
            Ok(full_path)
        }
    } else {
        let full_path = config.root.join(name);
        log::trace!("resolved path `{}` to `{}`", name, full_path.display());

        if !full_path.exists() {
            Err(crate::Error::from_message(resolve_error_message(
                name, &full_path, args, config,
            )))
        } else {
            Ok(full_path)
        }
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

pub fn resolve_error_message(name: &str, path: &Path, args: &cli::Args, config: &Config) -> String {
    let mut message = format!("failed to resolve path or alias `{}`", name);

    if !args.no_alias {
        let mut alias_suggestions = best_suggestions(suggest_aliases(name, config));
        if let Some(first) = alias_suggestions.next() {
            write!(
                &mut message,
                "\ndid you mean one of these aliases: {}",
                first
            )
            .unwrap();
            for suggestion in alias_suggestions {
                write!(&mut message, ", {}", suggestion).unwrap();
            }
            write!(&mut message, "?").unwrap()
        }
    }

    let mut path_suggestions = best_suggestions(suggest_paths(path, config));
    if let Some(first) = path_suggestions.next() {
        write!(
            &mut message,
            "\ndid you mean one of these paths: {}",
            first.display()
        )
        .unwrap();
        for suggestion in path_suggestions {
            write!(&mut message, ", {}", suggestion.display()).unwrap();
        }
        write!(&mut message, "?").unwrap()
    }

    message
}

fn best_suggestions<T>(mut result: Vec<(f64, T)>) -> impl Iterator<Item = T> {
    const MAX: usize = 4;

    result.sort_by(|&(l, _), &(r, _)| l.partial_cmp(&r).unwrap_or(cmp::Ordering::Less));
    result.into_iter().rev().map(|(_, value)| value).take(MAX)
}

fn suggest_aliases<'a>(name: &str, config: &'a Config) -> Vec<(f64, &'a str)> {
    const THRESHOLD: f64 = 0.8;

    let result: Vec<_> = config
        .aliases
        .keys()
        .filter_map(|alias| {
            let confidence = strsim::jaro_winkler(alias, name);
            if confidence > THRESHOLD {
                Some((confidence, alias.as_ref()))
            } else {
                None
            }
        })
        .collect();
    result
}

fn suggest_paths(path: &Path, config: &Config) -> Vec<(f64, PathBuf)> {
    let mut prefix = path;
    let mut suffix = Vec::new();

    while !prefix.exists() {
        match (prefix.parent(), prefix.file_name()) {
            (Some(parent), Some(component)) => {
                prefix = parent;
                suffix.push(component)
            }
            _ => return vec![],
        }
    }

    let mut result = vec![(1.0, prefix.to_owned())];

    while let Some(segment) = suffix.pop() {
        result = result
            .into_iter()
            .flat_map(|(confidence, prefix)| suggest_path_segments(&prefix, confidence, segment))
            .collect();
    }

    result
        .into_iter()
        .filter_map(|(confidence, path)| {
            path.strip_prefix(&config.root)
                .ok()
                .map(|stripped| (confidence, stripped.to_owned()))
        })
        .collect()
}

fn suggest_path_segments(path: &Path, confidence: f64, segment: &OsStr) -> Vec<(f64, PathBuf)> {
    const THRESHOLD: f64 = 0.8;

    let target = path.join(segment);
    if target.exists() {
        return vec![(confidence, target)];
    }

    let entries = match fs::read_dir(path) {
        Err(_) => return vec![],
        Ok(entries) => entries,
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let segment_confidence = jaro_winkler_path(&entry.file_name(), segment);
            if segment_confidence > THRESHOLD {
                Some((confidence * segment_confidence, entry.path()))
            } else {
                None
            }
        })
        .collect()
}

fn jaro_winkler_path(a: &OsStr, b: &OsStr) -> f64 {
    strsim::jaro_winkler(a.to_string_lossy().as_ref(), b.to_string_lossy().as_ref())
}
